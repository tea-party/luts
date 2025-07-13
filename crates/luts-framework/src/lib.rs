//! LUTS Framework - Unified framework re-exporting all LUTS components
//!
//! This meta-crate provides a unified API surface by re-exporting
//! all functionality from the individual LUTS crates.

// Re-export all functionality from LUTS crates
pub use luts_common as common;
pub use luts_memory as memory;
pub use luts_llm as llm;
pub use luts_tools as tools;
pub use luts_agents as agents;

// Re-export luts-core modules that haven't been migrated yet
pub use luts_core::{context, streaming};
// Re-export utils from layered crates instead of luts-core
pub use luts_memory::BlockUtils;

// Re-export top-level types for convenience
pub use luts_common::{LutsError, Result};
pub use luts_memory::{MemoryManager, MemoryBlock, BlockType, MemoryContent, BlockId};
pub use luts_llm::{LLMService, AiTool, ResponseStreamManager};
pub use luts_tools::{MathTool, DDGSearchTool, WebsiteTool, SemanticSearchTool};
pub use luts_agents::{Agent, AgentConfig, PersonalityAgentBuilder, AgentMessage, MessageResponse};

/// Convenience prelude module for common imports
pub mod prelude {
    // Common types and errors
    pub use luts_common::{LutsError, Result, BaseConfig, ProviderConfig, TokenPricing};
    
    // Memory management
    pub use luts_memory::{
        MemoryManager, MemoryBlock, MemoryBlockBuilder, BlockType, MemoryContent, BlockId,
        MemoryStore, MemoryQuery, VectorSearchConfig, EmbeddingService
    };
    
    // LLM and streaming
    pub use luts_llm::{
        LLMService, AiService, ResponseStreamManager, StreamConfig, InternalChatMessage,
        ConversationExporter, ConversationSearchEngine, AutoSaveManager
    };
    
    // Streaming (from luts-core until migrated)
    pub use luts_core::streaming::{ChunkType, ResponseChunk, StreamEvent, StreamableResponse};
    
    // Context management (from luts-core until migrated)
    pub use luts_core::context::{
        ContextManager, ContextWindowManager, CoreBlockManager, CoreBlockType
    };
    
    // Memory utilities
    pub use luts_memory::BlockUtils;
    
    // Context and token utils (from luts-core until migrated)
    pub use luts_core::utils::{TokenManager, TokenBudget, TokenUsage};
    
    // Tools
    pub use luts_tools::{MathTool, DDGSearchTool, WebsiteTool, SemanticSearchTool};
    pub use luts_llm::AiTool;
    
    // Agent system
    pub use luts_agents::{
        Agent, AgentConfig, PersonalityAgentBuilder, AgentMessage, MessageResponse, MessageType,
        BlockTool, RetrieveContextTool, ModifyCoreBlockTool, UpdateBlockTool, DeleteBlockTool
    };
}

