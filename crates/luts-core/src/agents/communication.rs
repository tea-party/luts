//! Communication primitives for agent messaging

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// A message sent between agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    /// Unique message identifier
    pub message_id: String,
    
    /// ID of the sending agent
    pub from_agent_id: String,
    
    /// ID of the receiving agent
    pub to_agent_id: String,
    
    /// Message content
    pub content: String,
    
    /// Optional structured data
    pub data: Option<Value>,
    
    /// Message type for routing/handling
    pub message_type: MessageType,
    
    /// Optional correlation ID for request/response pairs
    pub correlation_id: Option<String>,
    
    /// Timestamp when message was created
    pub timestamp: i64,
}

/// Response to an agent message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageResponse {
    /// ID of the original message being responded to
    pub in_response_to: String,
    
    /// Response content
    pub content: String,
    
    /// Optional structured response data
    pub data: Option<Value>,
    
    /// Whether the operation was successful
    pub success: bool,
    
    /// Optional error message if success is false
    pub error: Option<String>,
    
    /// Timestamp when response was created
    pub timestamp: i64,
}

/// Types of messages that can be sent between agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    /// Simple text message
    Chat,
    
    /// Request to perform a task
    TaskRequest,
    
    /// Response to a task request
    TaskResponse,
    
    /// System/control message
    System,
}

impl AgentMessage {
    /// Create a new chat message
    pub fn new_chat(
        from_agent_id: String,
        to_agent_id: String,
        content: String,
    ) -> Self {
        Self {
            message_id: Uuid::new_v4().to_string(),
            from_agent_id,
            to_agent_id,
            content,
            data: None,
            message_type: MessageType::Chat,
            correlation_id: None,
            timestamp: chrono::Utc::now().timestamp(),
        }
    }
    
    /// Create a new task request
    pub fn new_task_request(
        from_agent_id: String,
        to_agent_id: String,
        content: String,
        data: Option<Value>,
    ) -> Self {
        let correlation_id = Uuid::new_v4().to_string();
        Self {
            message_id: Uuid::new_v4().to_string(),
            from_agent_id,
            to_agent_id,
            content,
            data,
            message_type: MessageType::TaskRequest,
            correlation_id: Some(correlation_id),
            timestamp: chrono::Utc::now().timestamp(),
        }
    }
}

impl MessageResponse {
    /// Create a successful response
    pub fn success(
        in_response_to: String,
        content: String,
        data: Option<Value>,
    ) -> Self {
        Self {
            in_response_to,
            content,
            data,
            success: true,
            error: None,
            timestamp: chrono::Utc::now().timestamp(),
        }
    }
    
    /// Create an error response
    pub fn error(
        in_response_to: String,
        error_message: String,
    ) -> Self {
        Self {
            in_response_to,
            content: String::new(),
            data: None,
            success: false,
            error: Some(error_message),
            timestamp: chrono::Utc::now().timestamp(),
        }
    }
}