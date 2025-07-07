//! Type definitions for memory blocks
//!
//! This module defines the core types used in the memory block system.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

/// A unique identifier for a memory block
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct BlockId(pub String);

impl BlockId {
    /// Create a new block ID from a string
    pub fn new(id: impl Into<String>) -> Self {
        BlockId(id.into())
    }

    /// Generate a new random block ID
    pub fn generate() -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let random = rand::random::<u64>();
        BlockId(format!("block_{:x}_{:x}", timestamp, random))
    }

    /// Get the inner string value
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for BlockId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for BlockId {
    fn from(s: String) -> Self {
        BlockId(s)
    }
}

impl From<&str> for BlockId {
    fn from(s: &str) -> Self {
        BlockId(s.to_string())
    }
}

/// Types of memory blocks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BlockType {
    /// A message in a conversation
    Message,

    /// A summary of conversation or information
    Summary,

    /// A factual piece of information
    Fact,

    /// User preferences or settings
    Preference,

    /// A personal detail about the user
    PersonalInfo,

    /// A goal or objective
    Goal,

    /// A task to be performed
    Task,

    /// A custom block type
    Custom(u8),
}

impl fmt::Display for BlockType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BlockType::Message => write!(f, "message"),
            BlockType::Summary => write!(f, "summary"),
            BlockType::Fact => write!(f, "fact"),
            BlockType::Preference => write!(f, "preference"),
            BlockType::PersonalInfo => write!(f, "personal_info"),
            BlockType::Goal => write!(f, "goal"),
            BlockType::Task => write!(f, "task"),
            BlockType::Custom(id) => write!(f, "custom_{}", id),
        }
    }
}

/// Content of a memory block
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MemoryContent {
    /// Text content
    Text(String),

    /// JSON content
    Json(serde_json::Value),

    /// Binary content with MIME type
    Binary {
        /// MIME type of the binary data
        mime_type: String,

        /// The binary data encoded as base64
        data: String,
    },
}

impl MemoryContent {
    /// Get text content if available
    pub fn as_text(&self) -> Option<&str> {
        match self {
            MemoryContent::Text(text) => Some(text),
            _ => None,
        }
    }

    /// Get JSON content if available
    pub fn as_json(&self) -> Option<&serde_json::Value> {
        match self {
            MemoryContent::Json(json) => Some(json),
            _ => None,
        }
    }

    /// Get binary content if available
    pub fn as_binary(&self) -> Option<(&str, &str)> {
        match self {
            MemoryContent::Binary { mime_type, data } => Some((mime_type, data)),
            _ => None,
        }
    }
}

/// A time range for querying memory blocks
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TimeRange {
    /// Start time as Unix timestamp (milliseconds)
    pub start: Option<u64>,

    /// End time as Unix timestamp (milliseconds)
    pub end: Option<u64>,
}

impl TimeRange {
    /// Create a new time range
    pub fn new(start: Option<u64>, end: Option<u64>) -> Self {
        TimeRange { start, end }
    }

    /// Create a time range for the last N milliseconds
    pub fn last_ms(ms: u64) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        TimeRange {
            start: Some(now.saturating_sub(ms)),
            end: Some(now),
        }
    }

    /// Create a time range for the last N seconds
    pub fn last_seconds(seconds: u64) -> Self {
        Self::last_ms(seconds * 1000)
    }

    /// Create a time range for the last N minutes
    pub fn last_minutes(minutes: u64) -> Self {
        Self::last_seconds(minutes * 60)
    }

    /// Create a time range for the last N hours
    pub fn last_hours(hours: u64) -> Self {
        Self::last_minutes(hours * 60)
    }

    /// Create a time range for the last N days
    pub fn last_days(days: u64) -> Self {
        Self::last_hours(days * 24)
    }
}

/// Relevance score for memory blocks
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Relevance(pub f32);

impl Relevance {
    /// Create a new relevance score
    ///
    /// The score should be between 0.0 and 1.0, where 1.0 is the most relevant.
    pub fn new(score: f32) -> Self {
        let clamped = score.clamp(0.0, 1.0);
        Relevance(clamped)
    }

    /// Get the relevance score
    pub fn score(&self) -> f32 {
        self.0
    }

    /// Check if this is high relevance (above 0.7)
    pub fn is_high(&self) -> bool {
        self.0 >= 0.7
    }

    /// Check if this is medium relevance (between 0.3 and 0.7)
    pub fn is_medium(&self) -> bool {
        self.0 >= 0.3 && self.0 < 0.7
    }

    /// Check if this is low relevance (below 0.3)
    pub fn is_low(&self) -> bool {
        self.0 < 0.3
    }
}

impl From<f32> for Relevance {
    fn from(score: f32) -> Self {
        Relevance::new(score)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_id_generation() {
        let id1 = BlockId::generate();
        let id2 = BlockId::generate();

        // IDs should be different
        assert_ne!(id1, id2);

        // IDs should start with "block_"
        assert!(id1.as_str().starts_with("block_"));
        assert!(id2.as_str().starts_with("block_"));
    }

    #[test]
    fn test_time_range() {
        let range = TimeRange::last_days(1);

        // End time should be now (approximately)
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        if let Some(end) = range.end {
            // Allow small difference due to execution time
            assert!((now - end) < 1000);
        } else {
            panic!("End time should be set");
        }

        // Start time should be 24 hours earlier
        if let (Some(start), Some(end)) = (range.start, range.end) {
            let diff = end - start;
            let day_ms = 24 * 60 * 60 * 1000;

            // Allow small difference due to execution time
            assert!((diff as i64 - day_ms as i64).abs() < 1000);
        } else {
            panic!("Start and end times should be set");
        }
    }

    #[test]
    fn test_relevance() {
        let high = Relevance::new(0.8);
        let medium = Relevance::new(0.5);
        let low = Relevance::new(0.2);

        assert!(high.is_high());
        assert!(!high.is_medium());
        assert!(!high.is_low());

        assert!(!medium.is_high());
        assert!(medium.is_medium());
        assert!(!medium.is_low());

        assert!(!low.is_high());
        assert!(!low.is_medium());
        assert!(low.is_low());

        // Test clamping
        let too_high = Relevance::new(1.5);
        let too_low = Relevance::new(-0.5);

        assert_eq!(too_high.score(), 1.0);
        assert_eq!(too_low.score(), 0.0);
    }
}
