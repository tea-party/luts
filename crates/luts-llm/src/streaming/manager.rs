//! Response streaming and typing indicators system
//!
//! This module provides real-time response streaming capabilities with typing indicators,
//! progress tracking, and smooth UI updates for both TUI and API interfaces.

use crate::llm::{AiService, InternalChatMessage};
use anyhow::Result;
use chrono::{DateTime, Utc};
use futures_util::{Stream, StreamExt};
use genai::chat::ChatStreamEvent;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::sync::{RwLock, broadcast, mpsc};
use tokio_stream::wrappers::ReceiverStream;
use tracing::{debug, info, warn};

/// Streaming response chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseChunk {
    /// Chunk ID for ordering
    pub id: String,
    /// Sequence number
    pub sequence: u64,
    /// Content of this chunk
    pub content: String,
    /// Whether this is the final chunk
    pub is_final: bool,
    /// Timestamp when chunk was generated
    pub timestamp: DateTime<Utc>,
    /// Chunk type
    pub chunk_type: ChunkType,
    /// Metadata for this chunk
    pub metadata: ChunkMetadata,
}

/// Types of response chunks
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChunkType {
    /// Regular text content
    Text,
    /// Tool call information
    ToolCall,
    /// Tool response
    ToolResponse,
    /// Thinking/reasoning content
    Reasoning,
    /// Error message
    Error,
    /// Status update
    Status,
    /// Completion marker
    Complete,
}

/// Metadata for response chunks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkMetadata {
    /// Token count for this chunk
    pub token_count: Option<u32>,
    /// Processing time for this chunk
    pub processing_time_ms: Option<u64>,
    /// Model used for generation
    pub model: Option<String>,
    /// Confidence score
    pub confidence: Option<f64>,
    /// Custom metadata
    pub custom: HashMap<String, serde_json::Value>,
}

/// Typing indicator state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypingIndicator {
    /// Session ID
    pub session_id: String,
    /// Who is typing
    pub typing_entity: String,
    /// Typing status
    pub status: TypingStatus,
    /// When typing started
    pub started_at: DateTime<Utc>,
    /// Last activity timestamp
    pub last_activity: DateTime<Utc>,
    /// Estimated completion time
    pub estimated_completion: Option<DateTime<Utc>>,
    /// Progress percentage (0-100)
    pub progress_percent: Option<u8>,
}

/// Typing status states
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TypingStatus {
    /// Currently typing
    Typing,
    /// Thinking/processing
    Thinking,
    /// Calling tools
    CallingTools,
    /// Waiting for response
    Waiting,
    /// Stopped typing
    Stopped,
}

/// Stream configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamConfig {
    /// Enable response streaming
    pub enable_streaming: bool,
    /// Enable typing indicators
    pub enable_typing_indicators: bool,
    /// Chunk size for streaming (characters)
    pub chunk_size: usize,
    /// Minimum delay between chunks (ms)
    pub min_chunk_delay_ms: u64,
    /// Maximum delay between chunks (ms)
    pub max_chunk_delay_ms: u64,
    /// Enable progress estimation
    pub enable_progress_estimation: bool,
    /// Buffer size for streaming
    pub buffer_size: usize,
    /// Timeout for streaming responses
    pub stream_timeout_seconds: u64,
    /// Enable chunk compression
    pub enable_chunk_compression: bool,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            enable_streaming: true,
            enable_typing_indicators: true,
            chunk_size: 50, // Stream in 50-character chunks
            min_chunk_delay_ms: 10,
            max_chunk_delay_ms: 100,
            enable_progress_estimation: true,
            buffer_size: 1000,
            stream_timeout_seconds: 300, // 5 minute timeout
            enable_chunk_compression: false,
        }
    }
}

/// Streaming response stats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingStats {
    /// Total chunks sent
    pub total_chunks: u64,
    /// Total characters streamed
    pub total_characters: u64,
    /// Average chunk size
    pub avg_chunk_size: f64,
    /// Total streaming time
    pub total_stream_time_ms: u64,
    /// Characters per second
    pub chars_per_second: f64,
    /// Number of active streams
    pub active_streams: usize,
}

/// Response streaming manager
pub struct ResponseStreamManager {
    /// Configuration
    config: RwLock<StreamConfig>,
    /// Active streams
    active_streams: RwLock<HashMap<String, StreamSession>>,
    /// Typing indicators
    typing_indicators: RwLock<HashMap<String, TypingIndicator>>,
    /// Event broadcaster for UI updates
    event_sender: broadcast::Sender<StreamEvent>,
    /// Statistics
    stats: RwLock<StreamingStats>,
}

/// Individual streaming session
#[allow(dead_code)]
struct StreamSession {
    /// Session ID
    session_id: String,
    /// Message sender
    chunk_sender: mpsc::Sender<ResponseChunk>,
    /// Started timestamp
    started_at: DateTime<Utc>,
    /// Total chunks sent
    chunks_sent: u64,
    /// Total characters sent
    characters_sent: u64,
}

/// Stream events for UI updates
#[derive(Debug, Clone)]
pub enum StreamEvent {
    /// New chunk received
    ChunkReceived {
        session_id: String,
        chunk: ResponseChunk,
    },
    /// Typing status changed
    TypingStatusChanged {
        session_id: String,
        indicator: TypingIndicator,
    },
    /// Stream started
    StreamStarted { session_id: String },
    /// Stream completed
    StreamCompleted {
        session_id: String,
        total_chunks: u64,
        total_characters: u64,
        duration_ms: u64,
    },
    /// Stream error
    StreamError { session_id: String, error: String },
}

/// Streamable response wrapper
#[allow(dead_code)]
pub struct StreamableResponse {
    receiver: ReceiverStream<ResponseChunk>,
    session_id: String,
}

impl Stream for StreamableResponse {
    type Item = ResponseChunk;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.receiver).poll_next(cx)
    }
}

impl ResponseStreamManager {
    /// Create a new stream manager
    pub fn new() -> Self {
        let (event_sender, _) = broadcast::channel(1000);

        Self {
            config: RwLock::new(StreamConfig::default()),
            active_streams: RwLock::new(HashMap::new()),
            typing_indicators: RwLock::new(HashMap::new()),
            event_sender,
            stats: RwLock::new(StreamingStats {
                total_chunks: 0,
                total_characters: 0,
                avg_chunk_size: 0.0,
                total_stream_time_ms: 0,
                chars_per_second: 0.0,
                active_streams: 0,
            }),
        }
    }

    /// Update streaming configuration
    pub async fn update_config(&self, config: StreamConfig) -> Result<()> {
        *self.config.write().await = config;
        info!("Updated streaming configuration");
        Ok(())
    }

    /// Start streaming a response
    pub async fn start_streaming_response(
        &self,
        session_id: String,
        ai_service: Arc<dyn AiService>,
        messages: Vec<InternalChatMessage>,
    ) -> Result<StreamableResponse> {
        let config = self.config.read().await.clone();

        if !config.enable_streaming {
            return Err(anyhow::anyhow!("Streaming is disabled"));
        }

        // Create channel for streaming chunks
        let (chunk_sender, chunk_receiver) = mpsc::channel::<ResponseChunk>(config.buffer_size);

        // Start typing indicator
        if config.enable_typing_indicators {
            self.start_typing_indicator(session_id.clone(), "Assistant".to_string())
                .await;
        }

        // Create stream session
        let stream_session = StreamSession {
            session_id: session_id.clone(),
            chunk_sender: chunk_sender.clone(),
            started_at: Utc::now(),
            chunks_sent: 0,
            characters_sent: 0,
        };

        self.active_streams
            .write()
            .await
            .insert(session_id.clone(), stream_session);

        // Broadcast stream started event
        let _ = self.event_sender.send(StreamEvent::StreamStarted {
            session_id: session_id.clone(),
        });

        // Spawn background task for streaming
        let session_id_clone = session_id.clone();
        let config_clone = config.clone();
        let event_sender = self.event_sender.clone();

        tokio::spawn(async move {
            if let Err(e) = Self::stream_response_task(
                session_id_clone,
                ai_service,
                messages,
                chunk_sender,
                config_clone,
                event_sender,
            )
            .await
            {
                warn!("Streaming error: {}", e);
            }
        });

        Ok(StreamableResponse {
            receiver: ReceiverStream::new(chunk_receiver),
            session_id,
        })
    }

    /// Start typing indicator
    pub async fn start_typing_indicator(&self, session_id: String, entity: String) {
        let indicator = TypingIndicator {
            session_id: session_id.clone(),
            typing_entity: entity,
            status: TypingStatus::Typing,
            started_at: Utc::now(),
            last_activity: Utc::now(),
            estimated_completion: None,
            progress_percent: None,
        };

        self.typing_indicators
            .write()
            .await
            .insert(session_id.clone(), indicator.clone());

        // Broadcast typing event
        let _ = self.event_sender.send(StreamEvent::TypingStatusChanged {
            session_id,
            indicator,
        });
    }

    /// Update typing status
    pub async fn update_typing_status(
        &self,
        session_id: &str,
        status: TypingStatus,
        progress: Option<u8>,
    ) {
        let mut indicators = self.typing_indicators.write().await;
        if let Some(indicator) = indicators.get_mut(session_id) {
            indicator.status = status;
            indicator.last_activity = Utc::now();
            indicator.progress_percent = progress;

            // Broadcast updated typing event
            let _ = self.event_sender.send(StreamEvent::TypingStatusChanged {
                session_id: session_id.to_string(),
                indicator: indicator.clone(),
            });
        }
    }

    /// Stop typing indicator
    pub async fn stop_typing_indicator(&self, session_id: &str) {
        self.typing_indicators.write().await.remove(session_id);

        // Broadcast typing stopped
        let indicator = TypingIndicator {
            session_id: session_id.to_string(),
            typing_entity: "Assistant".to_string(),
            status: TypingStatus::Stopped,
            started_at: Utc::now(),
            last_activity: Utc::now(),
            estimated_completion: None,
            progress_percent: None,
        };

        let _ = self.event_sender.send(StreamEvent::TypingStatusChanged {
            session_id: session_id.to_string(),
            indicator,
        });
    }

    /// Subscribe to stream events
    pub fn subscribe_to_events(&self) -> broadcast::Receiver<StreamEvent> {
        self.event_sender.subscribe()
    }

    /// Get current typing indicators
    pub async fn get_typing_indicators(&self) -> HashMap<String, TypingIndicator> {
        self.typing_indicators.read().await.clone()
    }

    /// Get streaming statistics
    pub async fn get_stats(&self) -> StreamingStats {
        let mut stats = self.stats.read().await.clone();
        stats.active_streams = self.active_streams.read().await.len();
        stats
    }

    /// Stream response from an AI service with live genai streaming and tool calling
    pub async fn stream_genai_response(
        &self,
        session_id: String,
        ai_service: Arc<dyn AiService>,
        messages: Vec<InternalChatMessage>,
    ) -> Result<StreamableResponse> {
        let (chunk_sender, chunk_receiver) = mpsc::channel(1000);

        let config = self.config.read().await.clone();
        let event_sender = self.event_sender.clone();

        // Start streaming session
        let session_info = StreamSession {
            session_id: session_id.clone(),
            chunk_sender: chunk_sender.clone(),
            started_at: Utc::now(),
            chunks_sent: 0,
            characters_sent: 0,
        };

        self.active_streams
            .write()
            .await
            .insert(session_id.clone(), session_info);

        // Send stream started event
        let _ = event_sender.send(StreamEvent::StreamStarted {
            session_id: session_id.clone(),
        });

        // Spawn genai streaming task
        tokio::spawn(Self::genai_stream_task(
            session_id.clone(),
            ai_service,
            messages,
            chunk_sender,
            config.clone(),
            event_sender.clone(),
        ));

        Ok(StreamableResponse {
            receiver: ReceiverStream::new(chunk_receiver),
            session_id,
        })
    }

    // Private helper methods

    async fn stream_response_task(
        session_id: String,
        ai_service: Arc<dyn AiService>,
        messages: Vec<InternalChatMessage>,
        chunk_sender: mpsc::Sender<ResponseChunk>,
        config: StreamConfig,
        event_sender: broadcast::Sender<StreamEvent>,
    ) -> Result<()> {
        let start_time = Utc::now();
        let mut sequence = 0u64;

        // Generate response (this would ideally be streaming from the AI service)
        let response = ai_service.generate_response(&messages).await?;

        let content = match response {
            genai::chat::MessageContent::Text(text) => text,
            _ => return Err(anyhow::anyhow!("Unsupported response type for streaming")),
        };

        // Stream the response in chunks
        let mut total_chars = 0u64;
        let chars: Vec<char> = content.chars().collect();

        for chunk_start in (0..chars.len()).step_by(config.chunk_size) {
            let chunk_end = (chunk_start + config.chunk_size).min(chars.len());
            let chunk_content: String = chars[chunk_start..chunk_end].iter().collect();
            let is_final = chunk_end >= chars.len();

            let chunk = ResponseChunk {
                id: format!("chunk_{}_{}", session_id, sequence),
                sequence,
                content: chunk_content.clone(),
                is_final,
                timestamp: Utc::now(),
                chunk_type: if is_final {
                    ChunkType::Complete
                } else {
                    ChunkType::Text
                },
                metadata: ChunkMetadata {
                    token_count: Some(
                        (chunk_content.split_whitespace().count() as f32 * 1.3) as u32,
                    ),
                    processing_time_ms: None,
                    model: Some("streaming_model".to_string()),
                    confidence: None,
                    custom: HashMap::new(),
                },
            };

            // Send chunk
            if chunk_sender.send(chunk.clone()).await.is_err() {
                break; // Receiver dropped
            }

            // Broadcast chunk event
            let _ = event_sender.send(StreamEvent::ChunkReceived {
                session_id: session_id.clone(),
                chunk: chunk.clone(),
            });

            total_chars += chunk_content.len() as u64;
            sequence += 1;

            // Simulate realistic streaming delay
            let delay = std::cmp::max(
                config.min_chunk_delay_ms,
                std::cmp::min(config.max_chunk_delay_ms, chunk_content.len() as u64 * 2),
            );
            tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;

            // Update progress if enabled (simplified - would need manager reference for full implementation)
            if config.enable_progress_estimation {
                let progress = ((chunk_end as f64 / chars.len() as f64) * 100.0) as u8;
                // Note: In full implementation, would update typing status via manager
                debug!("Progress: {}%", progress);
            }
        }

        let duration = Utc::now().signed_duration_since(start_time);
        let duration_ms = duration.num_milliseconds() as u64;

        // Note: In full implementation, would stop typing indicator and update stats via manager
        // manager.stop_typing_indicator(&session_id).await;
        // manager.active_streams.write().await.remove(&session_id);

        // Broadcast completion event
        let _ = event_sender.send(StreamEvent::StreamCompleted {
            session_id: session_id.clone(),
            total_chunks: sequence,
            total_characters: total_chars,
            duration_ms,
        });

        info!(
            "Completed streaming response: {} chars in {}ms",
            total_chars, duration_ms
        );
        Ok(())
    }

    // Genai streaming task with tool calling support
    async fn genai_stream_task(
        session_id: String,
        ai_service: Arc<dyn AiService>,
        messages: Vec<InternalChatMessage>,
        chunk_sender: mpsc::Sender<ResponseChunk>,
        _config: StreamConfig,
        event_sender: broadcast::Sender<StreamEvent>,
    ) -> Result<()> {
        let start_time = Utc::now();
        let mut sequence = 0u64;
        let mut total_chars = 0u64;

        debug!("Starting genai streaming for session: {}", session_id);

        // Get streaming response from AI service
        let mut stream = ai_service.generate_response_stream(&messages).await?;

        let mut accumulated_text = String::new();
        let mut tool_calls: Vec<genai::chat::ToolCall> = Vec::new();

        // Process stream events
        while let Some(event_result) = stream.next().await {
            match event_result {
                Ok(event) => {
                    debug!("Received stream event: {:?}", event);

                    match event {
                        ChatStreamEvent::Start => {
                            info!("Stream started for session: {}", session_id);

                            // Send typing indicator
                            let chunk = ResponseChunk {
                                id: format!("{}_{}", session_id, sequence),
                                sequence,
                                content: "".to_string(),
                                is_final: false,
                                timestamp: Utc::now(),
                                chunk_type: ChunkType::Status,
                                metadata: ChunkMetadata {
                                    token_count: None,
                                    processing_time_ms: None,
                                    model: None,
                                    confidence: None,
                                    custom: HashMap::new(),
                                },
                            };

                            if chunk_sender.send(chunk).await.is_err() {
                                warn!("Failed to send status chunk for session: {}", session_id);
                                break;
                            }
                            sequence += 1;
                        }

                        ChatStreamEvent::End(_m) => {
                            info!("Stream ended for session: {}", session_id);

                            // Send final completion chunk
                            let duration_ms = (Utc::now() - start_time).num_milliseconds() as u64;

                            let chunk = ResponseChunk {
                                id: format!("{}_{}", session_id, sequence),
                                sequence,
                                content: "".to_string(),
                                is_final: true,
                                timestamp: Utc::now(),
                                chunk_type: ChunkType::Complete,
                                metadata: ChunkMetadata {
                                    token_count: None,
                                    processing_time_ms: Some(duration_ms),
                                    model: None,
                                    confidence: None,
                                    custom: {
                                        let mut custom = HashMap::new();
                                        custom.insert(
                                            "total_chunks".to_string(),
                                            serde_json::Value::Number(sequence.into()),
                                        );
                                        custom.insert(
                                            "total_characters".to_string(),
                                            serde_json::Value::Number(total_chars.into()),
                                        );
                                        custom.insert(
                                            "tool_calls_count".to_string(),
                                            serde_json::Value::Number(tool_calls.len().into()),
                                        );
                                        custom
                                    },
                                },
                            };

                            if chunk_sender.send(chunk).await.is_err() {
                                warn!(
                                    "Failed to send completion chunk for session: {}",
                                    session_id
                                );
                            }

                            // Send stream completed event
                            let _ = event_sender.send(StreamEvent::StreamCompleted {
                                session_id: session_id.clone(),
                                total_chunks: sequence,
                                total_characters: total_chars,
                                duration_ms,
                            });

                            break;
                        }

                        ChatStreamEvent::ToolCallChunk(t) => {
                            // Handle tool call chunk with proper formatting
                            debug!("Received tool call chunk: {:?}", t);

                            // Store the tool call for execution
                            tool_calls.push(t.tool_call.clone());

                            // Create a formatted tool call chunk for UI
                            let tool_content = format!(
                                "🔧 Calling {} with args: {}",
                                t.tool_call.fn_name,
                                serde_json::to_string(&t.tool_call.fn_arguments)
                                    .unwrap_or_else(|_| "{}".to_string())
                            );

                            let chunk = ResponseChunk {
                                id: format!("{}_{}", session_id, sequence),
                                sequence,
                                content: tool_content,
                                is_final: false,
                                timestamp: Utc::now(),
                                chunk_type: ChunkType::ToolCall,
                                metadata: ChunkMetadata {
                                    token_count: None,
                                    processing_time_ms: Some(
                                        (Utc::now() - start_time).num_milliseconds() as u64,
                                    ),
                                    model: None,
                                    confidence: None,
                                    custom: {
                                        let mut custom = HashMap::new();
                                        custom.insert(
                                            "tool_name".to_string(),
                                            serde_json::Value::String(t.tool_call.fn_name.clone()),
                                        );
                                        custom.insert(
                                            "tool_args".to_string(),
                                            t.tool_call.fn_arguments.clone(),
                                        );
                                        custom
                                    },
                                },
                            };

                            if chunk_sender.send(chunk).await.is_err() {
                                warn!("Failed to send tool call chunk for session: {}", session_id);
                                break;
                            }
                            sequence += 1;

                            // Execute the tool call if we have access to the LLM service
                            if let Some(llm_service) = ai_service.as_any().downcast_ref::<crate::llm::LLMService>() {
                                if let Some(tool) = llm_service.find_tool(&t.tool_call.fn_name) {
                                    debug!("Executing tool: {}", t.tool_call.fn_name);
                                    
                                    // Execute the tool
                                    match tool.execute(t.tool_call.fn_arguments.clone()).await {
                                        Ok(result) => {
                                            debug!("Tool {} executed successfully: {:?}", t.tool_call.fn_name, result);
                                            
                                            // Send tool result chunk
                                            let result_content = format!("✅ Tool result: {}", serde_json::to_string(&result).unwrap_or_else(|_| result.to_string()));
                                            
                                            let result_chunk = ResponseChunk {
                                                id: format!("{}_{}", session_id, sequence),
                                                sequence,
                                                content: result_content,
                                                is_final: false,
                                                timestamp: Utc::now(),
                                                chunk_type: ChunkType::ToolResponse,
                                                metadata: ChunkMetadata {
                                                    token_count: None,
                                                    processing_time_ms: Some(
                                                        (Utc::now() - start_time).num_milliseconds() as u64,
                                                    ),
                                                    model: None,
                                                    confidence: None,
                                                    custom: {
                                                        let mut custom = HashMap::new();
                                                        custom.insert(
                                                            "tool_name".to_string(),
                                                            serde_json::Value::String(t.tool_call.fn_name.clone()),
                                                        );
                                                        custom.insert(
                                                            "tool_result".to_string(),
                                                            result.clone(),
                                                        );
                                                        custom
                                                    },
                                                },
                                            };

                                            if chunk_sender.send(result_chunk).await.is_err() {
                                                warn!("Failed to send tool result chunk for session: {}", session_id);
                                                break;
                                            }
                                            sequence += 1;
                                        }
                                        Err(e) => {
                                            warn!("Tool {} execution failed: {}", t.tool_call.fn_name, e);
                                            
                                            // Send error chunk  
                                            let error_content = format!("❌ Tool error: {}", e);
                                            
                                            let error_chunk = ResponseChunk {
                                                id: format!("{}_{}", session_id, sequence),
                                                sequence,
                                                content: error_content,
                                                is_final: false,
                                                timestamp: Utc::now(),
                                                chunk_type: ChunkType::ToolResponse,
                                                metadata: ChunkMetadata {
                                                    token_count: None,
                                                    processing_time_ms: Some(
                                                        (Utc::now() - start_time).num_milliseconds() as u64,
                                                    ),
                                                    model: None,
                                                    confidence: None,
                                                    custom: {
                                                        let mut custom = HashMap::new();
                                                        custom.insert(
                                                            "tool_name".to_string(),
                                                            serde_json::Value::String(t.tool_call.fn_name.clone()),
                                                        );
                                                        custom.insert(
                                                            "error".to_string(),
                                                            serde_json::Value::String(e.to_string()),
                                                        );
                                                        custom
                                                    },
                                                },
                                            };

                                            if chunk_sender.send(error_chunk).await.is_err() {
                                                warn!("Failed to send tool error chunk for session: {}", session_id);
                                                break;
                                            }
                                            sequence += 1;
                                        }
                                    }
                                } else {
                                    warn!("Tool not found: {}", t.tool_call.fn_name);
                                    
                                    // Send tool not found error
                                    let error_content = format!("❌ Tool error: Tool '{}' not found", t.tool_call.fn_name);
                                    
                                    let error_chunk = ResponseChunk {
                                        id: format!("{}_{}", session_id, sequence),
                                        sequence,
                                        content: error_content,
                                        is_final: false,
                                        timestamp: Utc::now(),
                                        chunk_type: ChunkType::ToolResponse,
                                        metadata: ChunkMetadata {
                                            token_count: None,
                                            processing_time_ms: Some(
                                                (Utc::now() - start_time).num_milliseconds() as u64,
                                            ),
                                            model: None,
                                            confidence: None,
                                            custom: {
                                                let mut custom = HashMap::new();
                                                custom.insert(
                                                    "tool_name".to_string(),
                                                    serde_json::Value::String(t.tool_call.fn_name.clone()),
                                                );
                                                custom.insert(
                                                    "error".to_string(),
                                                    serde_json::Value::String(format!("Tool '{}' not found", t.tool_call.fn_name)),
                                                );
                                                custom
                                            },
                                        },
                                    };

                                    if chunk_sender.send(error_chunk).await.is_err() {
                                        warn!("Failed to send tool not found error chunk for session: {}", session_id);
                                        break;
                                    }
                                    sequence += 1;
                                }
                            } else {
                                warn!("Cannot execute tools: AI service is not an LLMService instance");
                            }
                        }

                        ChatStreamEvent::ReasoningChunk(c) => {
                            // Handle reasoning chunk
                            debug!("Received reasoning chunk: {:?}", c);
                            let content = c.content;
                            if !content.is_empty() {
                                accumulated_text.push_str(&content);
                                total_chars += content.len() as u64;

                                let chunk = ResponseChunk {
                                    id: format!("{}_{}", session_id, sequence),
                                    sequence,
                                    content: content.clone(),
                                    is_final: false,
                                    timestamp: Utc::now(),
                                    chunk_type: ChunkType::Reasoning,
                                    metadata: ChunkMetadata {
                                        token_count: Some(
                                            (content.split_whitespace().count() as f32 * 1.3)
                                                as u32,
                                        ),
                                        processing_time_ms: Some(
                                            (Utc::now() - start_time).num_milliseconds() as u64,
                                        ),
                                        model: None,
                                        confidence: None,
                                        custom: HashMap::new(),
                                    },
                                };

                                if chunk_sender.send(chunk).await.is_err() {
                                    warn!(
                                        "Failed to send reasoning chunk for session: {}",
                                        session_id
                                    );
                                    break;
                                }
                                sequence += 1;
                            }
                        }

                        ChatStreamEvent::Chunk(c) => {
                            // Handle regular text chunk
                            debug!("Received text chunk: {:?}", c);
                            let content = c.content;
                            if !content.is_empty() {
                                accumulated_text.push_str(&content);
                                total_chars += content.len() as u64;

                                let chunk = ResponseChunk {
                                    id: format!("{}_{}", session_id, sequence),
                                    sequence,
                                    content: content.clone(),
                                    is_final: false,
                                    timestamp: Utc::now(),
                                    chunk_type: ChunkType::Text,
                                    metadata: ChunkMetadata {
                                        token_count: Some(
                                            (content.split_whitespace().count() as f32 * 1.3)
                                                as u32,
                                        ),
                                        processing_time_ms: Some(
                                            (Utc::now() - start_time).num_milliseconds() as u64,
                                        ),
                                        model: None,
                                        confidence: None,
                                        custom: HashMap::new(),
                                    },
                                };

                                if chunk_sender.send(chunk).await.is_err() {
                                    warn!("Failed to send text chunk for session: {}", session_id);
                                    break;
                                }
                                sequence += 1;
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("Stream error for session {}: {}", session_id, e);

                    // Send error chunk
                    let chunk = ResponseChunk {
                        id: format!("{}_{}", session_id, sequence),
                        sequence,
                        content: format!("Error: {}", e),
                        is_final: true,
                        timestamp: Utc::now(),
                        chunk_type: ChunkType::Error,
                        metadata: ChunkMetadata {
                            token_count: None,
                            processing_time_ms: Some(
                                (Utc::now() - start_time).num_milliseconds() as u64
                            ),
                            model: None,
                            confidence: None,
                            custom: HashMap::new(),
                        },
                    };

                    let _ = chunk_sender.send(chunk).await;
                    let _ = event_sender.send(StreamEvent::StreamError {
                        session_id: session_id.clone(),
                        error: e.to_string(),
                    });

                    break;
                }
            }

            // Respect streaming rate limits
            // if config.rate_limit_ms > 0 {
            //     tokio::time::sleep(tokio::time::Duration::from_millis(config.rate_limit_ms)).await;
            // }
        }

        info!("Genai streaming task completed for session: {}", session_id);
        Ok(())
    }
}

/// Streaming response builder for easier integration
pub struct StreamingResponseBuilder {
    session_id: String,
    config: StreamConfig,
    messages: Vec<InternalChatMessage>,
}

impl StreamingResponseBuilder {
    /// Create a new streaming response builder
    pub fn new(session_id: String) -> Self {
        Self {
            session_id,
            config: StreamConfig::default(),
            messages: Vec::new(),
        }
    }

    /// Set streaming configuration
    pub fn with_config(mut self, config: StreamConfig) -> Self {
        self.config = config;
        self
    }

    /// Set messages for the conversation
    pub fn with_messages(mut self, messages: Vec<InternalChatMessage>) -> Self {
        self.messages = messages;
        self
    }

    /// Build and start the streaming response
    pub async fn build(
        self,
        stream_manager: &ResponseStreamManager,
        ai_service: Arc<dyn AiService>,
    ) -> Result<StreamableResponse> {
        stream_manager.update_config(self.config).await?;
        stream_manager
            .start_streaming_response(self.session_id, ai_service, self.messages)
            .await
    }
}

/// Helper trait for AI services to support streaming
pub trait StreamingAiService: AiService {
    /// Generate a streaming response
    fn generate_streaming_response(
        &self,
        messages: &[InternalChatMessage],
    ) -> Pin<Box<dyn Stream<Item = Result<ResponseChunk>> + Send>>;
}

/// Utilities for working with streaming responses
pub mod streaming_utils {
    use super::*;
    use futures_util::StreamExt;

    /// Collect all chunks from a stream into a complete response
    pub async fn collect_stream_to_string(mut stream: StreamableResponse) -> Result<String> {
        let mut complete_response = String::new();

        while let Some(chunk) = stream.next().await {
            complete_response.push_str(&chunk.content);
            if chunk.is_final {
                break;
            }
        }

        Ok(complete_response)
    }

    /// Split a large text into streaming chunks
    pub fn split_text_into_chunks(text: &str, chunk_size: usize) -> Vec<String> {
        text.chars()
            .collect::<Vec<_>>()
            .chunks(chunk_size)
            .map(|chunk| chunk.iter().collect())
            .collect()
    }

    /// Calculate streaming statistics for a response
    pub fn calculate_streaming_stats(chunks: &[ResponseChunk]) -> StreamingStats {
        let total_chunks = chunks.len() as u64;
        let total_characters = chunks.iter().map(|c| c.content.len() as u64).sum();

        let avg_chunk_size = if total_chunks > 0 {
            total_characters as f64 / total_chunks as f64
        } else {
            0.0
        };

        let total_stream_time_ms =
            if let (Some(first), Some(last)) = (chunks.first(), chunks.last()) {
                last.timestamp
                    .signed_duration_since(first.timestamp)
                    .num_milliseconds() as u64
            } else {
                0
            };

        let chars_per_second = if total_stream_time_ms > 0 {
            (total_characters as f64 / total_stream_time_ms as f64) * 1000.0
        } else {
            0.0
        };

        StreamingStats {
            total_chunks,
            total_characters,
            avg_chunk_size,
            total_stream_time_ms,
            chars_per_second,
            active_streams: 0,
        }
    }
}
