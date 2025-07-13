//! LUTS Common Error Types
//!
//! Centralized error handling for all LUTS components

use std::fmt;

/// Main error type for LUTS operations
#[derive(Debug)]
pub enum LutsError {
    /// Generic error with message
    Generic(String),
    /// IO-related errors
    Io(std::io::Error),
    /// Serialization/deserialization errors
    Serde(serde_json::Error),
    /// Database/storage errors
    Storage(String),
    /// Configuration errors
    Config(String),
    /// Agent/LLM related errors
    Agent(String),
    /// Tool execution errors
    Tool(String),
    /// Memory/context management errors
    Memory(String),
}

impl fmt::Display for LutsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LutsError::Generic(msg) => write!(f, "LUTS error: {}", msg),
            LutsError::Io(err) => write!(f, "IO error: {}", err),
            LutsError::Serde(err) => write!(f, "Serialization error: {}", err),
            LutsError::Storage(msg) => write!(f, "Storage error: {}", msg),
            LutsError::Config(msg) => write!(f, "Configuration error: {}", msg),
            LutsError::Agent(msg) => write!(f, "Agent error: {}", msg),
            LutsError::Tool(msg) => write!(f, "Tool error: {}", msg),
            LutsError::Memory(msg) => write!(f, "Memory error: {}", msg),
        }
    }
}

impl std::error::Error for LutsError {}

/// Convenience result type for LUTS operations
pub type Result<T> = std::result::Result<T, LutsError>;

// Implement From traits for common error types
impl From<std::io::Error> for LutsError {
    fn from(err: std::io::Error) -> Self {
        LutsError::Io(err)
    }
}

impl From<serde_json::Error> for LutsError {
    fn from(err: serde_json::Error) -> Self {
        LutsError::Serde(err)
    }
}

impl From<anyhow::Error> for LutsError {
    fn from(err: anyhow::Error) -> Self {
        LutsError::Generic(err.to_string())
    }
}