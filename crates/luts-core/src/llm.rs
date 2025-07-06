//! LLM Service for interacting with AI models
//!
//! This module provides a service for interacting with Large Language Models,
//! supporting streaming responses, tool calling, and token usage tracking.

use crate::tools::AiTool;
use crate::token_manager::{TokenManager, TokenUsage};
use anyhow::{Error, anyhow};
use async_trait::async_trait;
use futures::TryStreamExt;
use futures_util::Stream;
use genai::Client as GenaiClient;
use genai::chat::{
    ChatMessage as GenaiChatMessage, ChatStreamEvent, MessageContent, Tool,
    ToolCall as GenaiToolCall, ToolResponse as GenaiToolResponse,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::pin::Pin;
use std::sync::Arc;
use tracing::{debug, info};

/// Response from a tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResponse {
    /// Name of the tool that was called
    pub tool_name: String,

    /// Response from the tool
    pub content: String,

    /// Call ID for this tool call (used by genai)
    pub call_id: Option<String>,
}

impl ToolResponse {
    /// Create a new tool response
    pub fn new(tool_name: impl Into<String>, content: impl Into<String>) -> Self {
        ToolResponse {
            tool_name: tool_name.into(),
            content: content.into(),
            call_id: None,
        }
    }

    /// Create a new tool response with a call ID
    pub fn with_call_id(
        tool_name: impl Into<String>,
        content: impl Into<String>,
        call_id: impl Into<String>,
    ) -> Self {
        ToolResponse {
            tool_name: tool_name.into(),
            content: content.into(),
            call_id: Some(call_id.into()),
        }
    }

    /// Convert to a genai ToolResponse
    pub fn to_genai(&self) -> GenaiToolResponse {
        if let Some(call_id) = &self.call_id {
            GenaiToolResponse::new(call_id.clone(), self.content.clone())
        } else {
            // Default to empty call_id if none provided
            GenaiToolResponse::new("", self.content.clone())
        }
    }
}

impl From<GenaiToolResponse> for ToolResponse {
    fn from(resp: GenaiToolResponse) -> Self {
        Self {
            tool_name: "".to_string(), // genai doesn't include tool name in response
            content: resp.content,
            call_id: Some(resp.call_id),
        }
    }
}

/// Internal representation of a chat message, replacing the old ChatMessage struct.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InternalChatMessage {
    System {
        content: String,
    },
    User {
        content: String,
    },
    Assistant {
        content: String,
        tool_responses: Option<Vec<ToolResponse>>,
    },
    Tool {
        tool_name: String,
        content: String,
        call_id: Option<String>,
    },
}

impl InternalChatMessage {
    pub fn to_genai(&self) -> GenaiChatMessage {
        match self {
            InternalChatMessage::System { content } => GenaiChatMessage::system(content),
            InternalChatMessage::User { content } => GenaiChatMessage::user(content),
            InternalChatMessage::Assistant { content, .. } => GenaiChatMessage::assistant(content),
            InternalChatMessage::Tool { content, .. } => {
                // For now, fall back to assistant message until we figure out the correct genai API
                // TODO: Fix when genai library provides proper tool response method
                GenaiChatMessage::assistant(format!("Tool result: {}", content))
            }
        }
    }
}

impl From<GenaiChatMessage> for InternalChatMessage {
    fn from(msg: GenaiChatMessage) -> Self {
        match msg.role {
            genai::chat::ChatRole::System => InternalChatMessage::System {
                content: msg.content.into_text().unwrap_or_default(),
            },
            genai::chat::ChatRole::User => InternalChatMessage::User {
                content: msg.content.into_text().unwrap_or_default(),
            },
            genai::chat::ChatRole::Assistant => InternalChatMessage::Assistant {
                content: msg.content.into_text().unwrap_or_default(),
                tool_responses: None,
            },
            genai::chat::ChatRole::Tool => InternalChatMessage::Tool {
                tool_name: "".to_string(),
                content: msg.content.into_text().unwrap_or_default(),
                call_id: None,
            },
        }
    }
}

/// A chunk of text from a streaming response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatStreamChunk {
    /// Content of the chunk
    pub content: String,
}

/// A trait for AI services that can generate responses
#[async_trait]
pub trait AiService: Send + Sync {
    /// Generate a response to a conversation
    async fn generate_response(
        &self,
        messages: &[InternalChatMessage],
    ) -> anyhow::Result<MessageContent>;

    /// Generate a streaming response to a conversation
    async fn generate_response_stream<'a>(
        &'a self,
        messages: &'a [InternalChatMessage],
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatStreamEvent, Error>> + Send + 'a>>, Error>;
}

/// A tool call extracted from text
#[derive(Debug, Clone)]
pub struct ToolCall {
    /// Name of the tool to call
    pub tool_name: String,

    /// Arguments for the tool call as JSON
    pub tool_args: Value,

    /// Call ID for this tool call (used by genai)
    pub call_id: String,
}

impl From<GenaiToolCall> for ToolCall {
    fn from(call: GenaiToolCall) -> Self {
        Self {
            tool_name: call.fn_name,
            tool_args: call.fn_arguments,
            call_id: call.call_id,
        }
    }
}

/// A service for interacting with LLMs
pub struct LLMService {
    /// System prompt to use for context
    system_prompt: Option<String>,

    /// Available tools
    pub tools: Vec<Box<dyn AiTool>>,

    /// Provider/model to use
    provider: String,

    /// Underlying client for the LLM
    client: GenaiClient,
    
    /// Token manager for usage tracking
    token_manager: Option<Arc<TokenManager>>,
    
    /// Session ID for token tracking
    session_id: String,
    
    /// User ID for token tracking
    user_id: String,
}

impl LLMService {
    /// Create a new LLM service
    pub fn new(
        system_prompt: Option<&str>,
        tools: Vec<Box<dyn AiTool>>,
        provider: &str,
    ) -> Result<Self, Error> {
        Self::new_with_token_manager(system_prompt, tools, provider, None, "default_session", "default_user")
    }
    
    /// Create a new LLM service with token tracking
    pub fn new_with_token_manager(
        system_prompt: Option<&str>,
        tools: Vec<Box<dyn AiTool>>,
        provider: &str,
        token_manager: Option<Arc<TokenManager>>,
        session_id: &str,
        user_id: &str,
    ) -> Result<Self, Error> {
        // Create a real genai client with usage tracking enabled
        let client = GenaiClient::builder()
            .with_chat_options(genai::chat::ChatOptions {
                capture_content: Some(true),
                capture_reasoning_content: Some(true),
                capture_tool_calls: Some(true),
                capture_usage: Some(true), // Enable token usage tracking
                ..Default::default()
            })
            .build();

        Ok(LLMService {
            provider: provider.to_string(),
            client,
            system_prompt: system_prompt.map(|s| s.to_string()),
            tools,
            token_manager,
            session_id: session_id.to_string(),
            user_id: user_id.to_string(),
        })
    }

    /// Add a tool to the service
    pub fn add_tool(&mut self, tool: Box<dyn AiTool>) {
        self.tools.push(tool);
    }

    /// Remove a tool from the service
    pub fn remove_tool(&mut self, tool_name: &str) -> Result<(), Error> {
        if let Some(pos) = self.tools.iter().position(|t| t.name() == tool_name) {
            self.tools.remove(pos);
            Ok(())
        } else {
            Err(anyhow!("Tool not found: {}", tool_name))
        }
    }

    /// Set the system prompt
    pub fn set_system_prompt(&mut self, prompt: String) {
        self.system_prompt = Some(prompt);
    }

    /// List all available tools
    pub fn list_tools(&self) -> Vec<String> {
        self.tools.iter().map(|t| t.name().to_string()).collect()
    }

    /// Find a tool by name
    pub fn find_tool(&self, tool_name: &str) -> Option<&dyn AiTool> {
        self.tools.iter().find(|t| t.name() == tool_name).map(|b| b.as_ref())
    }

    /// Convert tools to genai Tool format
    pub fn get_genai_tools(&self) -> Vec<Tool> {
        self.tools
            .iter()
            .map(|tool| {
                Tool::new(tool.name())
                    .with_description(tool.description())
                    .with_schema(tool.schema())
            })
            .collect()
    }
}

#[async_trait]
impl AiService for LLMService {
    async fn generate_response(
        &self,
        messages: &[InternalChatMessage],
    ) -> anyhow::Result<MessageContent> {
        debug!("Generating response for {} messages", messages.len());
        debug!("LLM service has {} tools available", self.tools.len());

        // Convert messages to genai format
        let genai_messages: Vec<GenaiChatMessage> =
            messages.iter().map(|msg| msg.to_genai()).collect();

        // Create chat request with tools
        let mut chat_req = genai::chat::ChatRequest::new(genai_messages);

        // Add tools if available
        if !self.tools.is_empty() {
            let genai_tools = self.get_genai_tools();
            debug!("Adding {} tools to LLM request: {:?}", genai_tools.len(), 
                   genai_tools.iter().map(|t| &t.name).collect::<Vec<_>>());
            chat_req = chat_req.with_tools(genai_tools);
        } else {
            debug!("No tools available - LLM will not be able to call tools");
        }

        // Add system prompt if available
        if let Some(prompt) = &self.system_prompt {
            // Check for system message variant
            let has_system = messages
                .iter()
                .any(|msg| matches!(msg, InternalChatMessage::System { .. }));
            if !has_system {
                debug!("Adding system prompt to chat request");
                chat_req = chat_req.with_system(prompt.clone());
            }
        }

        debug!("Executing chat request to provider: {}", self.provider);

        // Execute chat request
        let response = self
            .client
            .exec_chat(&self.provider, chat_req, None)
            .await
            .map_err(|e| anyhow!("GenAI API error: {}", e))?;

        debug!("Response received with {} content items", response.content.len());
        if let Some(content) = response.content.first() {
            match content {
                MessageContent::Text(text) => {
                    info!("LLM returned text response: {}", text);
                }
                MessageContent::ToolCalls(calls) => {
                    info!("=== LLM TOOL CALLS DEBUG ===");
                    info!("LLM returned {} tool calls", calls.len());
                    for (i, call) in calls.iter().enumerate() {
                        info!("Tool call #{}: name='{}', id='{}', args={:?}", 
                              i + 1, call.fn_name, call.call_id, call.fn_arguments);
                    }
                    info!("=== END TOOL CALLS DEBUG ===");
                }
                MessageContent::Parts(parts) => {
                    info!("LLM returned {} parts", parts.len());
                }
                MessageContent::ToolResponses(responses) => {
                    info!("LLM returned {} tool responses", responses.len());
                }
            }
        }

        // Record token usage if manager is available
        if let Some(token_manager) = &self.token_manager {
            let token_usage = TokenUsage::from_genai_usage(
                &response.usage,
                self.provider.clone(),
                self.provider.clone(), // For now, use provider as model name
                "chat".to_string(),
                self.session_id.clone(),
                self.user_id.clone(),
            );
            
            if let Err(e) = token_manager.record_usage(token_usage).await {
                debug!("Failed to record token usage: {}", e);
            }
        }

        response
            .content
            .first().cloned()
            .ok_or_else(|| anyhow!("No content in chat response"))
    }

    async fn generate_response_stream<'a>(
        &'a self,
        messages: &'a [InternalChatMessage],
    ) -> Result<
        Pin<Box<dyn futures_util::Stream<Item = Result<ChatStreamEvent, Error>> + Send + 'a>>,
        Error,
    > {
        debug!("Streaming response for {} messages", messages.len());

        // Convert messages to genai format
        let genai_messages: Vec<GenaiChatMessage> =
            messages.iter().map(|msg| msg.to_genai()).collect();

        // Create chat request with tools
        let mut chat_req = genai::chat::ChatRequest::new(genai_messages);

        // Add tools if available
        if !self.tools.is_empty() {
            chat_req = chat_req.with_tools(self.get_genai_tools());
        }

        // Add system prompt if available
        if let Some(prompt) = &self.system_prompt {
            let has_system = messages
                .iter()
                .any(|msg| matches!(msg, InternalChatMessage::System { .. }));
            if !has_system {
                chat_req = chat_req.with_system(prompt.clone());
            }
        }

        // Execute streaming chat request
        let genai_stream = self
            .client
            .exec_chat_stream(&self.provider, chat_req, None)
            .await
            .map_err(|e| anyhow!("GenAI API error: {}", e))?;

        Ok(Box::pin(genai_stream.stream.map_err(|e| anyhow!(e))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::AiTool;

    struct MockTool;

    #[async_trait]
    impl AiTool for MockTool {
        fn name(&self) -> &str {
            "mock"
        }

        fn description(&self) -> &str {
            "A mock tool for testing"
        }

        fn schema(&self) -> Value {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "echo": {
                        "type": "string",
                        "description": "Text to echo back"
                    }
                },
                "required": ["echo"]
            })
        }

        async fn execute(&self, params: Value) -> Result<Value, Error> {
            if let Some(echo) = params.get("echo").and_then(|e| e.as_str()) {
                Ok(Value::String(format!("Echo: {}", echo)))
            } else {
                Err(anyhow!("Missing 'echo' parameter"))
            }
        }
    }

    #[tokio::test]
    async fn test_llm_service_init() {
        let service = LLMService::new(
            Some("You are a helpful assistant"),
            vec![Box::new(MockTool)],
            "test_provider",
        )
        .unwrap();

        assert_eq!(service.tools.len(), 1);
        assert_eq!(service.tools[0].name(), "mock");
        assert!(service.system_prompt.is_some());
    }
}
