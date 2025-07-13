//! LUTS Agents - Agent system and agent-specific tools
//!
//! This crate provides the agent system including base agent traits,
//! personality agents, agent registry, and agent-specific tools.

pub mod agents;
pub mod tools;

// Re-export key types for convenience
pub use agents::{
    Agent, AgentConfig, AgentMessage, BaseAgent, MessageResponse, MessageSender, MessageType,
    PersonalityAgent, PersonalityAgentBuilder, AgentRegistry, ToolCallInfo,
};
pub use tools::{
    BlockTool, DeleteBlockTool, InteractiveToolTester, ModifyCoreBlockTool, 
    RetrieveContextTool, UpdateBlockTool,
};