//! Memory block implementation for AI context management
//!
//! This module provides structures and traits for implementing memory blocks
//! that help manage and retrieve AI context, similar to Letta's approach.

mod block;
mod fjall_store;
mod types;

pub use block::{MemoryBlock, MemoryBlockBuilder, MemoryBlockMetadata};
pub use fjall_store::FjallMemoryStore;
pub use types::{BlockId, BlockType, MemoryContent, Relevance, TimeRange};

use anyhow::{Error, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// A trait defining operations for a memory storage system
#[async_trait]
pub trait MemoryStore: Send + Sync {
    /// Store a memory block
    async fn store_block(&self, block: MemoryBlock) -> Result<BlockId, Error>;

    /// Retrieve a memory block by its ID
    async fn get_block(&self, id: &BlockId) -> Result<Option<MemoryBlock>, Error>;

    /// Delete a memory block
    async fn delete_block(&self, id: &BlockId) -> Result<(), Error>;

    /// Search for memory blocks based on criteria
    async fn search_blocks(&self, query: &MemoryQuery) -> Result<Vec<MemoryBlock>, Error>;

    /// List all memory blocks for a user/session
    async fn list_blocks(&self, user_id: &str) -> Result<Vec<MemoryBlock>, Error>;
}

/// A query for searching memory blocks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryQuery {
    /// User ID to search blocks for
    pub user_id: Option<String>,

    /// Session ID to search blocks for
    pub session_id: Option<String>,

    /// Types of blocks to search for
    pub block_types: Option<Vec<BlockType>>,

    /// Time range to search in
    pub time_range: Option<TimeRange>,

    /// Text to search for in block content
    pub content_query: Option<String>,

    /// Maximum number of blocks to return
    pub limit: Option<usize>,

    /// Sort order (newer first, older first, relevance)
    pub sort: Option<QuerySort>,
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


impl Default for MemoryQuery {
    fn default() -> Self {
        MemoryQuery {
            user_id: None,
            session_id: None,
            block_types: None,
            time_range: None,
            content_query: None,
            limit: Some(100),
            sort: Some(QuerySort::default()),
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
        self.store.store_block(block).await
    }

    /// Retrieve a memory block by its ID
    pub async fn get(&self, id: &BlockId) -> Result<Option<MemoryBlock>, Error> {
        self.store.get_block(id).await
    }

    /// Delete a memory block
    pub async fn delete(&self, id: &BlockId) -> Result<(), Error> {
        self.store.delete_block(id).await
    }

    /// Search for memory blocks based on criteria
    pub async fn search(&self, query: &MemoryQuery) -> Result<Vec<MemoryBlock>, Error> {
        self.store.search_blocks(query).await
    }

    /// List all memory blocks for a user
    pub async fn list(&self, user_id: &str) -> Result<Vec<MemoryBlock>, Error> {
        self.store.list_blocks(user_id).await
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
