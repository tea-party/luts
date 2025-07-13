//! Configuration types and utilities for LUTS

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Base configuration that all components can use
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseConfig {
    pub data_dir: String,
    pub log_level: String,
}

impl Default for BaseConfig {
    fn default() -> Self {
        Self {
            data_dir: "./data".to_string(),
            log_level: "info".to_string(),
        }
    }
}

/// Provider configuration for LLM services
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// Provider name (e.g., "openai", "anthropic", "google")
    pub name: String,
    /// API key (optional, can use environment variables)
    pub api_key: Option<String>,
    /// Base URL for API (optional, uses provider default)
    pub base_url: Option<String>,
    /// Default model to use
    pub default_model: String,
    /// Request timeout in seconds
    pub timeout_seconds: Option<u64>,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            name: "openai".to_string(),
            api_key: None,
            base_url: None,
            default_model: "gpt-4".to_string(),
            timeout_seconds: Some(30),
        }
    }
}

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Storage backend type
    pub backend: StorageBackend,
    /// Storage path for file-based backends
    pub path: Option<PathBuf>,
    /// Connection string for database backends
    pub connection_string: Option<String>,
    /// Namespace for multi-tenant storage
    pub namespace: String,
    /// Database name
    pub database: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageBackend {
    /// SurrealDB file-based storage
    SurrealFile,
    /// SurrealDB in-memory storage
    SurrealMemory,
    /// Redis backend
    Redis,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            backend: StorageBackend::SurrealFile,
            path: Some(PathBuf::from("./data/memory.db")),
            connection_string: None,
            namespace: "luts".to_string(),
            database: "memory".to_string(),
        }
    }
}