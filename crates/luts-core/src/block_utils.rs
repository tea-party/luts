use crate::memory::{BlockId, MemoryBlock, MemoryManager, MemoryQuery};
use anyhow::Result;
use std::sync::Arc;

/// Utility struct for managing memory blocks via MemoryManager.
#[derive(Clone)]
pub struct BlockUtils {
    pub memory_manager: Arc<MemoryManager>,
}

impl BlockUtils {
    pub fn new(memory_manager: Arc<MemoryManager>) -> Self {
        Self { memory_manager }
    }

    /// Create a new memory block.
    pub async fn create_block(&self, block: MemoryBlock) -> Result<BlockId> {
        self.memory_manager.store(block).await
    }

    /// Retrieve a memory block by its ID.
    pub async fn get_block(&self, id: &BlockId) -> Result<Option<MemoryBlock>> {
        self.memory_manager.get(id).await
    }

    /// Delete a memory block by its ID.
    pub async fn delete_block(&self, id: &BlockId) -> Result<()> {
        self.memory_manager.delete(id).await
    }

    /// Update a memory block by deleting the old one and storing the new one.
    pub async fn update_block(&self, id: &BlockId, new_block: MemoryBlock) -> Result<BlockId> {
        self.memory_manager.delete(id).await?;
        self.memory_manager.store(new_block).await
    }

    /// Search for memory blocks using a MemoryQuery.
    pub async fn search_blocks(&self, query: &MemoryQuery) -> Result<Vec<MemoryBlock>> {
        self.memory_manager.search(query).await
    }

    /// List all memory blocks for a user.
    pub async fn list_blocks(&self, user_id: &str) -> Result<Vec<MemoryBlock>> {
        self.memory_manager.list(user_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::{BlockType, FjallMemoryStore, MemoryBlockBuilder, MemoryContent};
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_block_utils_crud() {
        let dir = tempdir().unwrap();
        let store = FjallMemoryStore::new(dir.path()).unwrap();
        let manager = Arc::new(MemoryManager::new(store));
        let utils = BlockUtils::new(manager.clone());

        // Create
        let block = MemoryBlockBuilder::default()
            .with_user_id("testuser")
            .with_type(BlockType::Fact)
            .with_content(MemoryContent::Text("test fact".to_string()))
            .build()
            .unwrap();
        let block_id = utils.create_block(block.clone()).await.unwrap();

        // Get
        let fetched = utils.get_block(&block_id).await.unwrap();
        assert_eq!(fetched, Some(block.clone()));

        // Update
        let updated_block = MemoryBlockBuilder::default()
            .with_user_id("testuser")
            .with_type(BlockType::Fact)
            .with_content(MemoryContent::Text("updated fact".to_string()))
            .build()
            .unwrap();
        let updated_id = utils
            .update_block(&block_id, updated_block.clone())
            .await
            .unwrap();
        let fetched_updated = utils.get_block(&updated_id).await.unwrap();
        assert_eq!(fetched_updated, Some(updated_block.clone()));

        // Delete
        utils.delete_block(&updated_id).await.unwrap();
        let after_delete = utils.get_block(&updated_id).await.unwrap();
        assert!(after_delete.is_none());
    }
}
