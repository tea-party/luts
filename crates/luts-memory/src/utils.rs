//! Utility functions for managing memory blocks
//!
//! This module provides high-level utility functions for common memory block operations.

use crate::{
    storage::{MemoryManager, MemoryQuery},
    block::MemoryBlock,
    types::BlockId,
};
use anyhow::Result;
use std::sync::Arc;

/// Utility struct for managing memory blocks via MemoryManager
#[derive(Clone)]
pub struct BlockUtils {
    pub memory_manager: Arc<MemoryManager>,
}

impl BlockUtils {
    /// Create a new BlockUtils instance
    pub fn new(memory_manager: Arc<MemoryManager>) -> Self {
        Self { memory_manager }
    }

    /// Create a new memory block
    pub async fn create_block(&self, block: MemoryBlock) -> Result<BlockId> {
        Ok(self.memory_manager.store(block).await?)
    }

    /// Retrieve a memory block by its ID
    pub async fn get_block(&self, id: &BlockId) -> Result<Option<MemoryBlock>> {
        Ok(self.memory_manager.get(id).await?)
    }

    /// Delete a memory block by its ID
    pub async fn delete_block(&self, id: &BlockId) -> Result<()> {
        self.memory_manager.delete(id).await?;
        Ok(())
    }

    /// Update a memory block by deleting the old one and storing the new one
    pub async fn update_block(&self, id: &BlockId, new_block: MemoryBlock) -> Result<BlockId> {
        self.memory_manager.delete(id).await?;
        Ok(self.memory_manager.store(new_block).await?)
    }

    /// Search for memory blocks using a MemoryQuery
    pub async fn search_blocks(&self, query: &MemoryQuery) -> Result<Vec<MemoryBlock>> {
        Ok(self.memory_manager.search(query).await?)
    }

    /// List all memory blocks for a user
    pub async fn list_blocks(&self, user_id: &str) -> Result<Vec<MemoryBlock>> {
        Ok(self.memory_manager.list(user_id).await?)
    }

    /// Clear all data for a user
    pub async fn clear_user_data(&self, user_id: &str) -> Result<u64> {
        Ok(self.memory_manager.clear_user_data(user_id).await?)
    }

    /// Get memory usage statistics for a user
    pub async fn get_stats(&self, user_id: &str) -> Result<crate::storage::MemoryStats> {
        Ok(self.memory_manager.get_stats(user_id).await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        block::MemoryBlockBuilder,
        types::{BlockType, MemoryContent},
        storage::{SurrealMemoryStore, SurrealConfig},
    };

    #[tokio::test]
    async fn test_block_utils() {
        // Create in-memory store for testing
        let config = SurrealConfig::Memory {
            namespace: "test".to_string(),
            database: "memory".to_string(),
        };
        
        let store = SurrealMemoryStore::new(config).await.unwrap();
        let memory_manager = Arc::new(MemoryManager::new(store));
        let utils = BlockUtils::new(memory_manager);
        
        // Create a test block
        let block = MemoryBlockBuilder::new()
            .with_user_id("test_user")
            .with_type(BlockType::Fact)
            .with_content(MemoryContent::Text("Test fact".to_string()))
            .build()
            .unwrap();
        
        // Test create
        let block_id = utils.create_block(block).await.unwrap();
        
        // Test retrieve
        let retrieved = utils.get_block(&block_id).await.unwrap();
        assert!(retrieved.is_some());
        
        // Test list
        let blocks = utils.list_blocks("test_user").await.unwrap();
        assert_eq!(blocks.len(), 1);
    }
}