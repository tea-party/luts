//! Common types used across LUTS components

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Export format options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportFormat {
    Json,
    Csv,
    Yaml,
    Markdown,
}

impl std::fmt::Display for ExportFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExportFormat::Json => write!(f, "json"),
            ExportFormat::Csv => write!(f, "csv"),
            ExportFormat::Yaml => write!(f, "yaml"),
            ExportFormat::Markdown => write!(f, "markdown"),
        }
    }
}

/// Provider type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProviderType {
    OpenAI,
    Anthropic,
    Google,
    Azure,
    Local,
}

impl std::fmt::Display for ProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderType::OpenAI => write!(f, "openai"),
            ProviderType::Anthropic => write!(f, "anthropic"),
            ProviderType::Google => write!(f, "google"),
            ProviderType::Azure => write!(f, "azure"),
            ProviderType::Local => write!(f, "local"),
        }
    }
}

/// Model type for categorization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelType {
    /// Large language model for chat/completion
    Language,
    /// Embedding model for vector representations
    Embedding,
    /// Image generation model
    Image,
    /// Audio/speech model
    Audio,
}

/// Usage filter for querying historical data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageFilter {
    pub provider: Option<String>,
    pub model: Option<String>,
    pub operation_type: Option<String>,
    pub session_id: Option<String>,
    pub user_id: Option<String>,
    pub date_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
    pub min_tokens: Option<u32>,
    pub max_tokens: Option<u32>,
}

impl Default for UsageFilter {
    fn default() -> Self {
        Self {
            provider: None,
            model: None,
            operation_type: None,
            session_id: None,
            user_id: None,
            date_range: None,
            min_tokens: None,
            max_tokens: None,
        }
    }
}

/// Time range for filtering data
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeRange {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

impl TimeRange {
    /// Create a time range for the last N days
    pub fn last_days(days: i64) -> Self {
        let end = Utc::now();
        let start = end - chrono::Duration::days(days);
        Self { start, end }
    }
    
    /// Create a time range for the last N hours
    pub fn last_hours(hours: i64) -> Self {
        let end = Utc::now();
        let start = end - chrono::Duration::hours(hours);
        Self { start, end }
    }
    
    /// Check if a timestamp falls within this range
    pub fn contains(&self, timestamp: DateTime<Utc>) -> bool {
        timestamp >= self.start && timestamp <= self.end
    }
}

/// Pagination parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pagination {
    /// Page number (0-based)
    pub page: usize,
    /// Number of items per page
    pub page_size: usize,
    /// Optional offset (overrides page if provided)
    pub offset: Option<usize>,
}

impl Default for Pagination {
    fn default() -> Self {
        Self {
            page: 0,
            page_size: crate::constants::DEFAULT_PAGE_SIZE,
            offset: None,
        }
    }
}

impl Pagination {
    /// Calculate the actual offset for database queries
    pub fn get_offset(&self) -> usize {
        self.offset.unwrap_or(self.page * self.page_size)
    }
    
    /// Create pagination for a specific page
    pub fn page(page: usize, page_size: usize) -> Self {
        Self {
            page,
            page_size,
            offset: None,
        }
    }
    
    /// Create pagination with a direct offset
    pub fn offset(offset: usize, page_size: usize) -> Self {
        Self {
            page: offset / page_size,
            page_size,
            offset: Some(offset),
        }
    }
}

/// Sort order for queries
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortOrder {
    Ascending,
    Descending,
}

impl Default for SortOrder {
    fn default() -> Self {
        Self::Descending
    }
}