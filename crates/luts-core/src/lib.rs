//! LUTS Core - Layered Universal Tiered Storage for AI
//!
//! This crate provides the core functionality for the LUTS system, including:
//! - Context management with pluggable backends
//! - Memory block system for structured AI context
//! - Tools for AI assistants
//! - Multiagent system support

pub mod agents;
pub mod auto_save;
pub mod block_utils;
pub mod context;
pub mod context_saving;
pub mod conversation_bookmarks;
pub mod conversation_export;
pub mod conversation_search;
pub mod memory;
pub mod segment_editor;
pub mod response_streaming;
pub mod tools;
pub mod token_manager;
pub mod summarization;

// Re-export key types for convenience
pub use agents::{Agent, AgentConfig, AgentRegistry, BaseAgent, AgentMessage, MessageResponse, PersonalityAgentBuilder};
pub use context::{ContextManager, ContextProvider, FjallContextProvider};
pub use memory::{
    BlockId, BlockType, FjallMemoryStore, MemoryBlock, MemoryBlockBuilder, MemoryContent,
    MemoryManager, MemoryQuery, MemoryStore, QuerySort, TimeRange,
};
pub use tools::AiTool;
pub use token_manager::{TokenManager, TokenUsage, TokenBudget, TokenAnalytics, BudgetStatus};
pub use summarization::{ConversationSummarizer, ConversationSummary, SummarizationConfig, SummarizationStrategy, SummarizationAnalytics};
pub use context_saving::{ContextManager as ContextSavingManager, ContextSnapshot, ContextSaveConfig, RestoredContext, SnapshotQuery, ContextStorageStats};
pub use conversation_export::{ConversationExporter, ExportableConversation, ExportFormat, ExportSettings, ImportSettings, ConversationMetadata, ExportableMessage};
pub use conversation_search::{ConversationSearchEngine, ConversationSearchQuery, SearchFilters, ConversationSearchResult, SavedSearch, SearchAnalytics};
pub use conversation_bookmarks::{BookmarkManager, ConversationBookmark, BookmarkCollection, BookmarkQuery, BookmarkStats, QuickAccessBookmark, BookmarkPriority, BookmarkColor};
pub use auto_save::{AutoSaveManager, AutoSaveConfig, AutoSaveState, AutoSaveData, AutoSaveType, AutoSaveStats};
pub use segment_editor::{ConversationSegmentEditor, ConversationSegment, SegmentType, SegmentEdit, EditType, ImportanceLevel, BatchEditOperation, UndoRedoOperation};
pub use response_streaming::{ResponseStreamManager, ResponseChunk, ChunkType, TypingIndicator, TypingStatus, StreamConfig, StreamEvent, StreamableResponse, StreamingResponseBuilder};

/// The LLM service for interacting with AI models
pub mod llm;
