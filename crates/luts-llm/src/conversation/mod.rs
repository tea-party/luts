//! Conversation management and utilities
//!
//! This module contains all conversation-related functionality including
//! bookmarks, exports, search, segments, auto-save, and summarization.

pub mod auto_save;
pub mod bookmarks;
pub mod export;
pub mod search;
pub mod segments;
pub mod summarization;

// Re-export key types for convenience
pub use auto_save::{
    AutoSaveConfig, AutoSaveData, AutoSaveManager, AutoSaveState, AutoSaveStats, AutoSaveType,
};
pub use bookmarks::{
    BookmarkCollection, BookmarkColor, BookmarkManager, BookmarkPriority, BookmarkQuery,
    BookmarkStats, ConversationBookmark, QuickAccessBookmark,
};
pub use export::{
    ConversationExporter, ConversationMetadata, ExportFormat, ExportSettings,
    ExportableConversation, ExportableMessage, ImportSettings,
};
pub use search::{
    ConversationSearchEngine, ConversationSearchQuery, ConversationSearchResult, SavedSearch,
    SearchAnalytics, SearchFilters,
};
pub use segments::{
    BatchEditOperation, ConversationSegment, ConversationSegmentEditor, EditType, ImportanceLevel,
    SegmentEdit, SegmentType, UndoRedoOperation,
};
pub use summarization::{
    ConversationSummarizer, ConversationSummary, SummarizationAnalytics, SummarizationConfig,
    SummarizationStrategy,
};