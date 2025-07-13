use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Sse},
    routing::{get, post},
};
use axum::response::sse::{Event, KeepAlive};
use chrono;
use futures::Stream;
use futures_util::StreamExt;
use genai::chat;
use luts_framework::agents::{AgentRegistry, AgentMessage, MessageType};
use luts_framework::llm::{AiService, InternalChatMessage as ChatMessage, LLMService, ToolResponse};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{debug, error, info};
use uuid::Uuid;

pub struct OpenAIState {
    pub llm_service: LLMService,
    pub agent_registry: Arc<AgentRegistry>,
    pub _conversation_store: Arc<Mutex<HashMap<String, Vec<ChatMessage>>>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct OpenAIChatMessage {
    pub role: String,
    pub content: String,
    pub name: Option<String>,
    pub tool_calls: Option<Vec<OpenAIToolCall>>,
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct OpenAIToolCall {
    pub id: String,
    pub function: OpenAIFunctionCall,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct OpenAIFunctionCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<OpenAIChatMessage>,
    pub stream: Option<bool>,
    pub agent: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<ChatCompletionChoice>,
    pub usage: Usage,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ChatCompletionChoice {
    pub index: u32,
    pub message: OpenAIChatMessage,
    pub finish_reason: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Serialize)]
pub struct ChatCompletionChunk {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<ChatCompletionChunkChoice>,
}

#[derive(Debug, Serialize)]
pub struct ChatCompletionChunkChoice {
    pub index: u32,
    pub delta: ChatCompletionDelta,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ChatCompletionDelta {
    pub role: Option<String>,
    pub content: Option<String>,
    pub tool_calls: Option<Vec<OpenAIToolCall>>,
}

/// Convert OpenAI chat messages to LUTS format
pub fn openai_to_luts_messages(messages: &[OpenAIChatMessage]) -> Vec<ChatMessage> {
    messages
        .iter()
        .map(|msg| {
            match msg.role.as_str() {
                "system" => ChatMessage::System {
                    content: msg.content.clone(),
                },
                "user" => ChatMessage::User {
                    content: msg.content.clone(),
                },
                "assistant" => {
                    // Convert tool_calls to tool_responses if present
                    let tool_responses = msg.tool_calls.as_ref().map(|calls| {
                        calls
                            .iter()
                            .map(|call| ToolResponse {
                                tool_name: call.function.name.clone(),
                                content: call.function.arguments.clone(),
                                call_id: Some(call.id.clone()),
                            })
                            .collect()
                    });
                    ChatMessage::Assistant {
                        content: msg.content.clone(),
                        tool_responses,
                    }
                }
                "tool" => ChatMessage::Tool {
                    tool_name: msg.name.clone().unwrap_or_default(),
                    content: msg.content.clone(),
                    call_id: msg.tool_call_id.clone(),
                },
                _ => {
                    // Fallback: treat as user message
                    ChatMessage::User {
                        content: msg.content.clone(),
                    }
                }
            }
        })
        .collect()
}

/// Handler for the chat completions endpoint
pub async fn chat_completions(
    State(state): State<Arc<OpenAIState>>,
    Json(request): Json<ChatCompletionRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    info!("Chat completion request for model: {}", request.model);
    debug!("Request: {:?}", request);

    // Convert OpenAI messages to LUTS format
    let messages = openai_to_luts_messages(&request.messages);

    let completion_id = Uuid::new_v4().to_string();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Check if streaming is requested
    if request.stream.unwrap_or(false) {
        // Handle streaming response
        let stream = create_streaming_response(
            state,
            messages,
            completion_id,
            now,
            request.model,
            request.agent,
        ).await.map_err(|e| {
            error!("Error creating stream: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Error creating stream: {}", e))
        })?;

        Ok(Sse::new(stream)
            .keep_alive(KeepAlive::new()
                .interval(Duration::from_secs(15))
                .text("keep-alive-text"))
            .into_response())
    } else {
        // Handle non-streaming response  
        let response = create_non_streaming_response(
            state,
            messages,
            completion_id,
            now,
            request,
        ).await?;
        Ok(response.into_response())
    }
}

/// Create a non-streaming response
async fn create_non_streaming_response(
    state: Arc<OpenAIState>,
    messages: Vec<ChatMessage>,
    completion_id: String,
    created: u64,
    request: ChatCompletionRequest,
) -> Result<Json<ChatCompletionResponse>, (StatusCode, String)> {
    // Use agent if specified, otherwise fallback to LLM service
    let (response_text, openai_tool_calls) = if let Some(agent_name) = &request.agent {
        // Check if agent exists in registry
        if !state.agent_registry.has_agent(agent_name).await {
            error!("Agent {} not found in registry", agent_name);
            return Err((StatusCode::BAD_REQUEST, format!("Agent '{}' not found", agent_name)));
        }
        
        // Process message with agent
        let agent_message = AgentMessage {
            message_id: Uuid::new_v4().to_string(),
            from_agent_id: "user".to_string(),
            to_agent_id: agent_name.clone(),
            content: messages.last().map(|m| match m {
                ChatMessage::User { content } => content.clone(),
                ChatMessage::Assistant { content, .. } => content.clone(),
                ChatMessage::System { content } => content.clone(),
                ChatMessage::Tool { content, .. } => content.clone(),
            }).unwrap_or_default(),
            data: None,
            message_type: MessageType::Chat,
            correlation_id: None,
            timestamp: chrono::Utc::now().timestamp(),
        };
        
        let response = state.agent_registry.send_message_and_wait(agent_message).await
            .map_err(|e| {
                error!("Error processing message with agent: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, format!("Error processing message: {}", e))
            })?;
        
        debug!("Non-streaming agent response received with {} tool calls", response.tool_calls.len());
        for (i, tool_call) in response.tool_calls.iter().enumerate() {
            debug!("Tool call {}: {} -> {}", i, tool_call.tool_name, tool_call.tool_result);
        }
        
        // Convert tool calls to OpenAI format
        let openai_tool_calls = if !response.tool_calls.is_empty() {
            Some(response.tool_calls.iter().map(|tool_call| OpenAIToolCall {
                id: tool_call.call_id.clone().unwrap_or_else(|| Uuid::new_v4().to_string()),
                function: OpenAIFunctionCall {
                    name: tool_call.tool_name.clone(),
                    arguments: serde_json::to_string(&tool_call.tool_args).unwrap_or_else(|_| "{}".to_string()),
                },
            }).collect())
        } else {
            None
        };
        
        (response.content, openai_tool_calls)
    } else {
        // Fallback to LLM service
        let res = state
            .llm_service
            .generate_response(&messages)
            .await
            .map_err(|e| {
                error!("Error generating response: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Error generating response: {}", e),
                )
            })?;

        let response_text = res.into_text().ok_or({
            error!("Error converting response to text");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Error converting response to text".to_string(),
            )
        })?;
        
        (response_text, None)
    };

    // Simple token counting (not accurate, just for the API format)
    let prompt_tokens = request
        .messages
        .iter()
        .map(|m| m.content.len() as u32 / 4)
        .sum();
    let completion_tokens = response_text.len() as u32 / 4;

    let api_response = ChatCompletionResponse {
        id: completion_id,
        object: "chat.completion".to_string(),
        created,
        model: request.model,
        choices: vec![ChatCompletionChoice {
            index: 0,
            message: OpenAIChatMessage {
                role: "assistant".to_string(),
                content: response_text,
                name: None,
                tool_calls: openai_tool_calls,
                tool_call_id: None,
            },
            finish_reason: "stop".to_string(),
        }],
        usage: Usage {
            prompt_tokens,
            completion_tokens,
            total_tokens: prompt_tokens + completion_tokens,
        },
    };

    Ok(Json(api_response))
}

/// Create a streaming response
async fn create_streaming_response(
    state: Arc<OpenAIState>,
    messages: Vec<ChatMessage>,
    completion_id: String,
    created: u64,
    model: String,
    agent_name: Option<String>,
) -> Result<impl Stream<Item = Result<Event, Infallible>>, anyhow::Error> {
    // Use a channel to collect the stream items
    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
    
    // Clone data for the async task
    let completion_id_clone = completion_id.clone();
    let model_clone = model.clone();
    
    // Spawn a task to consume the stream and send to channel
    tokio::spawn(async move {
        use futures_util::StreamExt;
        
        // Use agent if specified, otherwise fallback to LLM service
        let stream_result = if let Some(agent_name) = &agent_name {
            // Check if agent exists in registry
            if !state.agent_registry.has_agent(agent_name).await {
                error!("Agent {} not found in registry", agent_name);
                let _ = sender.send(Event::default().data(format!("{{\"error\":\"Agent '{}' not found\"}}", agent_name)));
                return;
            }
            
            // Process message with agent
            let agent_message = AgentMessage {
                message_id: Uuid::new_v4().to_string(),
                from_agent_id: "user".to_string(),
                to_agent_id: agent_name.clone(),
                content: messages.last().map(|m| match m {
                    ChatMessage::User { content } => content.clone(),
                    ChatMessage::Assistant { content, .. } => content.clone(),
                    ChatMessage::System { content } => content.clone(),
                    ChatMessage::Tool { content, .. } => content.clone(),
                }).unwrap_or_default(),
                data: None,
                message_type: MessageType::Chat,
                correlation_id: None,
                timestamp: chrono::Utc::now().timestamp(),
            };
            
            // For now, agents don't support streaming, so we'll get the full response
            // and simulate streaming by sending it as chunks
            match state.agent_registry.send_message_and_wait(agent_message).await {
                Ok(response) => {
                    debug!("Agent response received with {} tool calls", response.tool_calls.len());
                    for (i, tool_call) in response.tool_calls.iter().enumerate() {
                        debug!("Tool call {}: {} -> {}", i, tool_call.tool_name, tool_call.tool_result);
                    }
                    
                    // Send start event
                    let start_chunk = ChatCompletionChunk {
                        id: completion_id_clone.clone(),
                        object: "chat.completion.chunk".to_string(),
                        created,
                        model: model_clone.clone(),
                        choices: vec![ChatCompletionChunkChoice {
                            index: 0,
                            delta: ChatCompletionDelta {
                                role: Some("assistant".to_string()),
                                content: None,
                                tool_calls: None,
                            },
                            finish_reason: None,
                        }],
                    };
                    
                    if let Ok(json_data) = serde_json::to_string(&start_chunk) {
                        let _ = sender.send(Event::default().data(json_data));
                    }
                    
                    // Send tool call chunks if any tools were used
                    for tool_call in &response.tool_calls {
                        let tool_call_chunk = ChatCompletionChunk {
                            id: completion_id_clone.clone(),
                            object: "chat.completion.chunk".to_string(),
                            created,
                            model: model_clone.clone(),
                            choices: vec![ChatCompletionChunkChoice {
                                index: 0,
                                delta: ChatCompletionDelta {
                                    role: None,
                                    content: Some(format!(
                                        "🔧 Calling {} with args: {}\n✅ Result: {}",
                                        tool_call.tool_name,
                                        serde_json::to_string(&tool_call.tool_args)
                                            .unwrap_or_else(|_| "{}".to_string()),
                                        if tool_call.success {
                                            tool_call.tool_result.clone()
                                        } else {
                                            format!("❌ Error: {}", tool_call.tool_result)
                                        }
                                    )),
                                    tool_calls: None,
                                },
                                finish_reason: None,
                            }],
                        };
                        
                        if let Ok(json_data) = serde_json::to_string(&tool_call_chunk) {
                            let _ = sender.send(Event::default().data(json_data));
                        }
                    }
                    
                    // Send content chunk
                    let content_chunk = ChatCompletionChunk {
                        id: completion_id_clone.clone(),
                        object: "chat.completion.chunk".to_string(),
                        created,
                        model: model_clone.clone(),
                        choices: vec![ChatCompletionChunkChoice {
                            index: 0,
                            delta: ChatCompletionDelta {
                                role: None,
                                content: Some(response.content),
                                tool_calls: None,
                            },
                            finish_reason: None,
                        }],
                    };
                    
                    if let Ok(json_data) = serde_json::to_string(&content_chunk) {
                        let _ = sender.send(Event::default().data(json_data));
                    }
                    
                    // Send end chunk
                    let end_chunk = ChatCompletionChunk {
                        id: completion_id_clone.clone(),
                        object: "chat.completion.chunk".to_string(),
                        created,
                        model: model_clone.clone(),
                        choices: vec![ChatCompletionChunkChoice {
                            index: 0,
                            delta: ChatCompletionDelta {
                                role: None,
                                content: None,
                                tool_calls: None,
                            },
                            finish_reason: Some("stop".to_string()),
                        }],
                    };
                    
                    if let Ok(json_data) = serde_json::to_string(&end_chunk) {
                        let _ = sender.send(Event::default().data(json_data));
                    }
                    
                    return;
                }
                Err(e) => {
                    error!("Error processing message with agent: {}", e);
                    let _ = sender.send(Event::default().data(format!("{{\"error\":\"{}\"}}", e)));
                    return;
                }
            }
        } else {
            // Fallback to LLM service streaming
            state
                .llm_service
                .generate_response_stream(&messages)
                .await
        };
        
        // If we're using LLM service, handle the streaming
        if agent_name.is_none() {
            let mut stream = match stream_result {
                Ok(stream) => stream,
                Err(e) => {
                    error!("Error creating stream: {}", e);
                    let _ = sender.send(Event::default().data(format!("{{\"error\":\"{}\"}}", e)));
                    return;
                }
            };
            
            while let Some(chunk_result) = stream.next().await {
                let event = match chunk_result {
                    Ok(chunk) => {
                        // Convert genai ChatStreamEvent to OpenAI format
                        let chunk_data = match chunk {
                            chat::ChatStreamEvent::Start => {
                                ChatCompletionChunk {
                                    id: completion_id_clone.clone(),
                                    object: "chat.completion.chunk".to_string(),
                                    created,
                                    model: model_clone.clone(),
                                    choices: vec![ChatCompletionChunkChoice {
                                        index: 0,
                                        delta: ChatCompletionDelta {
                                            role: Some("assistant".to_string()),
                                            content: None,
                                            tool_calls: None,
                                        },
                                        finish_reason: None,
                                    }],
                                }
                            },
                            chat::ChatStreamEvent::Chunk(stream_chunk) => {
                                // stream_chunk.content is a String, not an Option<MessageContent>
                                let content_text = if stream_chunk.content.is_empty() {
                                    None
                                } else {
                                    Some(stream_chunk.content.clone())
                                };
                                
                                ChatCompletionChunk {
                                    id: completion_id_clone.clone(),
                                    object: "chat.completion.chunk".to_string(),
                                    created,
                                    model: model_clone.clone(),
                                    choices: vec![ChatCompletionChunkChoice {
                                        index: 0,
                                        delta: ChatCompletionDelta {
                                            role: None,
                                            content: content_text,
                                            tool_calls: None,
                                        },
                                        finish_reason: None,
                                    }],
                                }
                            },
                            chat::ChatStreamEvent::End(_) => {
                                ChatCompletionChunk {
                                    id: completion_id_clone.clone(),
                                    object: "chat.completion.chunk".to_string(),
                                    created,
                                    model: model_clone.clone(),
                                    choices: vec![ChatCompletionChunkChoice {
                                        index: 0,
                                        delta: ChatCompletionDelta {
                                            role: None,
                                            content: None,
                                            tool_calls: None,
                                        },
                                        finish_reason: Some("stop".to_string()),
                                    }],
                                }
                            },
                            chat::ChatStreamEvent::ReasoningChunk(_) => {
                                // Handle reasoning chunks - for now just skip them
                                ChatCompletionChunk {
                                    id: completion_id_clone.clone(),
                                    object: "chat.completion.chunk".to_string(),
                                    created,
                                    model: model_clone.clone(),
                                    choices: vec![ChatCompletionChunkChoice {
                                        index: 0,
                                        delta: ChatCompletionDelta {
                                            role: None,
                                            content: None,
                                            tool_calls: None,
                                        },
                                        finish_reason: None,
                                    }],
                                }
                            },
                            chat::ChatStreamEvent::ToolCallChunk(tool_chunk) => {
                                // Handle tool call chunk - show the tool being called
                                let tool_content = format!(
                                    "🔧 Calling {} with args: {}",
                                    tool_chunk.tool_call.fn_name,
                                    serde_json::to_string(&tool_chunk.tool_call.fn_arguments)
                                        .unwrap_or_else(|_| "{}".to_string())
                                );
                                
                                ChatCompletionChunk {
                                    id: completion_id_clone.clone(),
                                    object: "chat.completion.chunk".to_string(),
                                    created,
                                    model: model_clone.clone(),
                                    choices: vec![ChatCompletionChunkChoice {
                                        index: 0,
                                        delta: ChatCompletionDelta {
                                            role: None,
                                            content: Some(tool_content),
                                            tool_calls: None,
                                        },
                                        finish_reason: None,
                                    }],
                                }
                            },
                        };

                        // Serialize to JSON and create SSE event
                        match serde_json::to_string(&chunk_data) {
                            Ok(json_data) => {
                                Event::default().data(json_data)
                            }
                            Err(e) => {
                                error!("Failed to serialize chunk: {}", e);
                                Event::default().data("{\"error\":\"serialization_error\"}")
                            }
                        }
                    }
                    Err(e) => {
                        error!("Error in stream: {}", e);
                        Event::default().data(format!("{{\"error\":\"{}\"}}", e))
                    }
                };
                
                // Send to channel
                if sender.send(event).is_err() {
                    break; // Receiver dropped
                }
            }
        }
    });

    // Create a stream from the receiver
    let event_stream = tokio_stream::wrappers::UnboundedReceiverStream::new(receiver);
    
    Ok(Box::pin(event_stream.map(Ok)))
}

/// Handler for the models endpoint
pub async fn list_models() -> impl IntoResponse {
    Json(serde_json::json!({
        "object": "list",
        "data": [
            {
                "id": "DeepSeek-R1-0528",
                "object": "model",
                "created": 1716508800,
                "owned_by": "luts"
            }
        ]
    }))
}

/// Handler for the health check endpoint
pub async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

pub fn openai_routes(state: std::sync::Arc<OpenAIState>) -> Router {
    Router::new()
        .route("/v1/chat/completions", post(chat_completions))
        .route("/v1/models", get(list_models))
        .route("/health", get(health_check))
        .with_state(state)
}
