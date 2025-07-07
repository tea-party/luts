//! Memory block implementation for AI context management
//!
//! This module provides structures and traits for implementing memory blocks
//! that help manage and retrieve AI context, similar to Letta's approach.

mod block;
mod embeddings;
mod surreal;
mod types;

pub use block::{MemoryBlock, MemoryBlockBuilder, MemoryBlockMetadata};
pub use embeddings::{
    EmbeddingService, EmbeddingConfig, EmbeddingProvider, EmbeddingServiceFactory,
    VectorSimilarity, VectorSearchConfig, SimilarityMetric
};
pub use surreal::{SurrealMemoryStore, SurrealConfig, AuthConfig, RelationType};
pub use types::{BlockId, BlockType, MemoryContent, Relevance, TimeRange};

use anyhow::{Error, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

// Re-export key types for external use

/// A trait defining operations for a memory storage system
#[async_trait]
pub trait MemoryStore: Send + Sync {
    /// Store a memory block
    async fn store(&self, block: MemoryBlock) -> Result<BlockId, Error>;

    /// Retrieve a memory block by its ID
    async fn retrieve(&self, id: &BlockId) -> Result<Option<MemoryBlock>, Error>;

    /// Delete a memory block
    async fn delete(&self, id: &BlockId) -> Result<bool, Error>;

    /// Update an existing memory block
    async fn update(&self, id: &BlockId, block: MemoryBlock) -> Result<MemoryBlock, Error>;

    /// Search for memory blocks based on criteria
    async fn query(&self, query: MemoryQuery) -> Result<Vec<MemoryBlock>, Error>;

    /// Clear all data for a specific user
    async fn clear_user_data(&self, user_id: &str) -> Result<u64, Error>;

    /// Get statistics about memory usage
    async fn get_stats(&self, user_id: &str) -> Result<MemoryStats, Error>;
}

/// A query for searching memory blocks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryQuery {
    /// User ID to search blocks for
    pub user_id: Option<String>,

    /// Session ID to search blocks for
    pub session_id: Option<String>,

    /// Types of blocks to search for
    pub block_types: Vec<BlockType>,

    /// Text to search for in block content
    pub content_contains: Option<String>,

    /// Time range filters
    pub created_after: Option<chrono::DateTime<chrono::Utc>>,
    pub created_before: Option<chrono::DateTime<chrono::Utc>>,

    /// Maximum number of blocks to return
    pub limit: Option<usize>,

    /// Sort order (newer first, older first, relevance)
    pub sort: Option<QuerySort>,

    /// Vector similarity search parameters
    pub vector_search: Option<VectorQuery>,
}

/// Vector similarity search query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorQuery {
    /// The query embedding vector to search for
    pub query_vector: Vec<f32>,
    
    /// Configuration for vector search
    pub search_config: VectorSearchConfig,
}

/// Sort order for memory queries
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[derive(Default)]
pub enum QuerySort {
    /// Sort by creation time, newest first
    #[default]
    NewestFirst,

    /// Sort by creation time, oldest first
    OldestFirst,

    /// Sort by relevance score
    Relevance,
}

/// Memory statistics for the MemoryStore trait
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStats {
    pub total_blocks: u64,
    pub blocks_by_type: HashMap<String, u64>,
    pub total_size_bytes: u64,
    pub last_updated: DateTime<Utc>,
}


impl Default for MemoryQuery {
    fn default() -> Self {
        MemoryQuery {
            user_id: None,
            session_id: None,
            block_types: Vec::new(),
            content_contains: None,
            created_after: None,
            created_before: None,
            limit: Some(100),
            sort: Some(QuerySort::default()),
            vector_search: None,
        }
    }
}

/// A memory manager that interfaces with a storage backend
pub struct MemoryManager {
    store: Arc<dyn MemoryStore>,
}

impl MemoryManager {
    /// Create a new memory manager with the given store
    pub fn new(store: impl MemoryStore + 'static) -> Self {
        MemoryManager {
            store: Arc::new(store),
        }
    }

    /// Store a memory block
    pub async fn store(&self, block: MemoryBlock) -> Result<BlockId, Error> {
        self.store.store(block).await
    }

    /// Retrieve a memory block by its ID
    pub async fn get(&self, id: &BlockId) -> Result<Option<MemoryBlock>, Error> {
        self.store.retrieve(id).await
    }

    /// Delete a memory block
    pub async fn delete(&self, id: &BlockId) -> Result<bool, Error> {
        self.store.delete(id).await
    }

    /// Update an existing memory block
    pub async fn update(&self, id: &BlockId, block: MemoryBlock) -> Result<MemoryBlock, Error> {
        self.store.update(id, block).await
    }

    /// Search for memory blocks based on criteria
    pub async fn search(&self, query: &MemoryQuery) -> Result<Vec<MemoryBlock>, Error> {
        self.store.query(query.clone()).await
    }

    /// List all memory blocks for a user
    pub async fn list(&self, user_id: &str) -> Result<Vec<MemoryBlock>, Error> {
        let query = MemoryQuery {
            user_id: Some(user_id.to_string()),
            ..Default::default()
        };
        self.store.query(query).await
    }

    /// Clear all data for a user
    pub async fn clear_user_data(&self, user_id: &str) -> Result<u64, Error> {
        self.store.clear_user_data(user_id).await
    }

    /// Get memory usage statistics
    pub async fn get_stats(&self, user_id: &str) -> Result<MemoryStats, Error> {
        self.store.get_stats(user_id).await
    }

    /// Perform semantic search using embeddings
    pub async fn semantic_search(
        &self,
        embedding_service: &dyn EmbeddingService,
        query_text: &str,
        user_id: &str,
        search_config: Option<VectorSearchConfig>,
    ) -> Result<Vec<MemoryBlock>, Error> {
        // Generate embedding for the query text
        let query_embedding = embedding_service.embed_text(query_text).await?;
        
        // Configure search parameters
        let search_config = search_config.unwrap_or_default();
        
        // Create vector query
        let vector_query = VectorQuery {
            query_vector: query_embedding,
            search_config: search_config.clone(),
        };
        
        // Create memory query with vector search
        let memory_query = MemoryQuery {
            user_id: Some(user_id.to_string()),
            vector_search: Some(vector_query),
            limit: Some(search_config.max_results),
            ..Default::default()
        };
        
        self.store.query(memory_query).await
    }

    /// Create a conversation summary block from a collection of message blocks
    pub async fn summarize_conversation(
        &self,
        session_id: &str,
        message_block_ids: &[BlockId],
    ) -> Result<BlockId, Error> {
        // In a real implementation, this would:
        // 1. Retrieve all the message blocks
        // 2. Use an LLM to generate a summary
        // 3. Create a new summary block
        // 4. Store the summary block

        let mut message_blocks = Vec::new();
        for id in message_block_ids {
            if let Some(block) = self.get(id).await? {
                message_blocks.push(block);
            }
        }

        // For now, just create a placeholder summary block
        let summary_content = format!("Summary of {} messages", message_blocks.len());

        let summary_block = MemoryBlockBuilder::new()
            .with_user_id("system")
            .with_session_id(session_id)
            .with_type(BlockType::Summary)
            .with_content(MemoryContent::Text(summary_content))
            .with_reference_ids(message_block_ids.to_vec())
            .build()?;

        self.store(summary_block).await
    }
}
