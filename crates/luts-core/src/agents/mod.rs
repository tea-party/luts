//! Agent module for multiagent systems
//!
//! This module provides the core agent abstraction and basic communication
//! infrastructure for building multiagent systems with LUTS.

pub mod base_agent;
pub mod communication;
pub mod personality;
pub mod registry;

pub use base_agent::{BaseAgent, MessageSender};
pub use communication::{AgentMessage, MessageResponse, MessageType};
pub use personality::{PersonalityAgent, PersonalityAgentBuilder};
pub use registry::AgentRegistry;

use anyhow::Error;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Core trait for agents in the LUTS system
#[async_trait]
pub trait Agent: Send + Sync {
    /// Unique identifier for this agent
    fn agent_id(&self) -> &str;
    
    /// Human-readable name for this agent
    fn name(&self) -> &str;
    
    /// Role or type of this agent (e.g., "research", "memory", "coordinator")
    fn role(&self) -> &str;
    
    /// Process an incoming message and generate a response
    async fn process_message(&mut self, message: AgentMessage) -> Result<MessageResponse, Error>;
    
    /// Send a message to another agent (handled by registry)
    async fn send_message(&self, message: AgentMessage) -> Result<(), Error>;
    
    /// Get the list of available tools for this agent
    fn get_available_tools(&self) -> Vec<String>;
    
    /// Downcast helper for registry management
    fn as_any(&self) -> &dyn std::any::Any;
}

/// Configuration for creating an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Unique agent identifier
    pub agent_id: String,
    
    /// Human-readable name
    pub name: String,
    
    /// Agent role/type
    pub role: String,
    
    /// System prompt for this agent
    pub system_prompt: Option<String>,
    
    /// LLM provider to use
    pub provider: String,
    
    /// Tools available to this agent
    pub tool_names: Vec<String>,
    
    /// Data directory for this agent's memory
    pub data_dir: String,
}