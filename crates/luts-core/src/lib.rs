//! LUTS Core - Layered Universal Tiered Storage for AI
//!
//! This crate provides the core functionality for the LUTS system, including:
//! - Context management with pluggable backends
//! - Memory block system for structured AI context
//! - Tools for AI assistants
//! - Multiagent system support

pub mod agents;
pub mod block_utils;
pub mod context;
pub mod memory;
pub mod tools;

// Re-export key types for convenience
pub use agents::{Agent, AgentConfig, AgentRegistry, BaseAgent, AgentMessage, MessageResponse, PersonalityAgentBuilder};
pub use context::{ContextManager, ContextProvider, FjallContextProvider};
pub use memory::{
    BlockId, BlockType, FjallMemoryStore, MemoryBlock, MemoryBlockBuilder, MemoryContent,
    MemoryManager, MemoryQuery, MemoryStore, QuerySort, TimeRange,
};
pub use tools::AiTool;

/// The LLM service for interacting with AI models
pub mod llm;
