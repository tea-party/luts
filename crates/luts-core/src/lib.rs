//! LUTS Core - Layered Universal Tiered Storage for AI
//!
//! This crate provides the core functionality for the LUTS system, including:
//! - Context management with pluggable backends
//! - Memory block system for structured AI context
//! - Tools for AI assistants
//! - Multiagent system support

pub mod agents;
pub mod context;
pub mod conversation;
pub mod memory;
pub mod streaming;
pub mod tools;
pub mod utils;

// Re-export key types for convenience
pub use agents::{
    Agent, AgentConfig, AgentMessage, AgentRegistry, BaseAgent, MessageResponse,
    PersonalityAgentBuilder,
};
pub use context::{
    ContextManager, ContextProvider, ContextSaveConfig, ContextSavingManager, ContextSnapshot,
    ContextStorageStats, RestoredContext, SnapshotQuery,
    CoreBlock, CoreBlockManager, CoreBlockType, CoreBlockConfig, CoreBlockStats,
    ContextWindowManager, ContextWindowConfig, ContextWindow, ContextWindowStats,
    SelectionStrategy, TokenBreakdown, ContextMemoryBlock,
};
pub use conversation::{
    AutoSaveConfig, AutoSaveData, AutoSaveManager, AutoSaveState, AutoSaveStats, AutoSaveType,
    BatchEditOperation, BookmarkCollection, BookmarkColor, BookmarkManager, BookmarkPriority,
    BookmarkQuery, BookmarkStats, ConversationBookmark, ConversationExporter, ConversationMetadata,
    ConversationSearchEngine, ConversationSearchQuery, ConversationSearchResult,
    ConversationSegment, ConversationSegmentEditor, ConversationSummarizer, ConversationSummary,
    EditType, ExportFormat, ExportSettings, ExportableConversation, ExportableMessage,
    ImportSettings, ImportanceLevel, QuickAccessBookmark, SavedSearch, SearchAnalytics,
    SearchFilters, SegmentEdit, SegmentType, SummarizationAnalytics, SummarizationConfig,
    SummarizationStrategy, UndoRedoOperation,
};
pub use memory::{
    BlockId, BlockType, MemoryBlock, MemoryBlockBuilder, MemoryContent,
    MemoryManager, MemoryQuery, MemoryStore, QuerySort, TimeRange,
};
pub use streaming::{
    ChunkType, ResponseChunk, ResponseStreamManager, StreamConfig, StreamEvent, StreamableResponse,
    StreamingResponseBuilder, TypingIndicator, TypingStatus,
};
pub use tools::AiTool;
pub use utils::{BlockUtils, BudgetStatus, TokenAnalytics, TokenBudget, TokenManager, TokenUsage};

/// The LLM service for interacting with AI models
pub mod llm;
