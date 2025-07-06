use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use luts_core::llm::{AiService, InternalChatMessage as ChatMessage, LLMService, ToolResponse};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, info};
use uuid::Uuid;

pub struct OpenAIState {
    pub llm_service: LLMService,
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

    // Generate response
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

    let response = res.into_text().ok_or({
        {
            error!("Error converting response to text");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Error converting response to text".to_string(),
            )
        }
    })?;

    // Create response
    let completion_id = Uuid::new_v4().to_string();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Simple token counting (not accurate, just for the API format)
    let prompt_tokens = request
        .messages
        .iter()
        .map(|m| m.content.len() as u32 / 4)
        .sum();
    let completion_tokens = response.len() as u32 / 4;

    let api_response = ChatCompletionResponse {
        id: completion_id,
        object: "chat.completion".to_string(),
        created: now,
        model: request.model,
        choices: vec![ChatCompletionChoice {
            index: 0,
            message: OpenAIChatMessage {
                role: "assistant".to_string(),
                content: response.to_string(),
                name: None,
                tool_calls: None,
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
