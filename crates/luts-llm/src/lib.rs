//! LLM integration and streaming for LUTS
//!
//! This crate provides LLM integration, streaming infrastructure, 
//! and conversation management for the LUTS system.

pub mod tools;
pub mod llm;
pub mod streaming;
pub mod conversation;

// Re-export key types for convenience
pub use llm::{
    AiService, ChatStreamChunk, InternalChatMessage, LLMService, ToolCall, ToolResponse,
};
pub use streaming::{
    ChunkType, ResponseChunk, ResponseStreamManager, StreamConfig, StreamEvent, StreamableResponse,
    StreamingResponseBuilder, TypingIndicator, TypingStatus,
};
pub use conversation::{
    AutoSaveConfig, AutoSaveData, AutoSaveManager, AutoSaveState, AutoSaveStats, AutoSaveType,
    BookmarkCollection, BookmarkColor, BookmarkManager, BookmarkPriority, BookmarkQuery,
    BookmarkStats, ConversationBookmark, ConversationExporter, ConversationMetadata,
    ConversationSearchEngine, ConversationSearchQuery, ConversationSearchResult,
    ConversationSegment, ConversationSegmentEditor, ConversationSummarizer,
    ConversationSummary, ExportFormat, ExportSettings, ExportableConversation,
    ExportableMessage, ImportSettings, QuickAccessBookmark, SavedSearch, SearchAnalytics,
    SearchFilters, SegmentEdit, SegmentType, SummarizationAnalytics, SummarizationConfig,
    SummarizationStrategy, UndoRedoOperation,
};
pub use tools::AiTool;