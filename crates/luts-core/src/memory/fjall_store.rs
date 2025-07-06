#![allow(unused)]
//! Fjäll-based implementation of the MemoryStore
//!
//! This module provides a MemoryStore implementation using the fjall crate as the storage backend.

use crate::memory::{
    BlockId, BlockType, MemoryBlock, MemoryBlockBuilder, MemoryContent, MemoryQuery, MemoryStore,
    QuerySort, Relevance, TimeRange,
};
use anyhow::{Error, Result, anyhow};
use async_trait::async_trait;
use fjall::{Config, Keyspace, PartitionCreateOptions, PartitionHandle, PersistMode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

/// Partition names for different block indices
const PARTITION_BLOCKS: &str = "blocks";

/// A Fjäll-based implementation of the MemoryStore trait
pub struct FjallMemoryStore {
    keyspace: Arc<Keyspace>,
    blocks: PartitionHandle,
}

impl FjallMemoryStore {
    /// Create a new FjallMemoryStore with the given data directory
    pub fn new<P: AsRef<Path>>(data_dir: P) -> Result<Self> {
        let keyspace = Config::new(data_dir).open()?;
        let blocks =
            keyspace.open_partition(PARTITION_BLOCKS, PartitionCreateOptions::default())?;
        Ok(Self {
            keyspace: Arc::new(keyspace),
            blocks,
        })
    }

    /// Serialize a MemoryBlock to bytes
    fn serialize_block(block: &MemoryBlock) -> Result<Vec<u8>> {
        Ok(serde_cbor::to_vec(block)?)
    }
    fn deserialize_block(bytes: &[u8]) -> Result<MemoryBlock> {
        Ok(serde_cbor::from_slice(bytes)?)
    }

    /// Generate the key for a block (just the block ID as bytes)
    fn block_key(id: &BlockId) -> Vec<u8> {
        id.as_str().as_bytes().to_vec()
    }
}

#[async_trait]
impl MemoryStore for FjallMemoryStore {
    async fn store_block(&self, block: MemoryBlock) -> Result<BlockId, Error> {
        let key = Self::block_key(block.id());
        let value = Self::serialize_block(&block)?;
        self.blocks.insert(key, value)?;
        self.keyspace.persist(PersistMode::Buffer)?;
        Ok(block.id().clone())
    }

    async fn get_block(&self, id: &BlockId) -> Result<Option<MemoryBlock>> {
        let key = Self::block_key(id);
        if let Some(bytes) = self.blocks.get(key)? {
            let block = Self::deserialize_block(&bytes)?;
            Ok(Some(block))
        } else {
            Ok(None)
        }
    }

    async fn delete_block(&self, id: &BlockId) -> Result<()> {
        let key = Self::block_key(id);
        self.blocks.remove(key)?;
        self.keyspace.persist(PersistMode::Buffer)?;
        Ok(())
    }

    async fn search_blocks(&self, query: &MemoryQuery) -> Result<Vec<MemoryBlock>> {
        // For now, scan all blocks and filter in memory.
        // For large datasets, consider adding secondary indices/partitions.
        // Helper functions for cosine similarity
        fn text_to_vector(text: &str) -> HashMap<String, f32> {
            let mut vec = HashMap::new();
            for word in text.split_whitespace() {
                *vec.entry(word.to_lowercase()).or_insert(0.0) += 1.0;
            }
            vec
        }

        fn cosine_similarity(a: &HashMap<String, f32>, b: &HashMap<String, f32>) -> f32 {
            let mut dot = 0.0;
            let mut norm_a = 0.0;
            let mut norm_b = 0.0;
            for (k, v) in a {
                dot += v * b.get(k).unwrap_or(&0.0);
                norm_a += v * v;
            }
            for v in b.values() {
                norm_b += v * v;
            }
            if norm_a == 0.0 || norm_b == 0.0 {
                0.0
            } else {
                dot / (norm_a.sqrt() * norm_b.sqrt())
            }
        }

        let mut results = Vec::new();
        let content_query_vec = query.content_query.as_ref().map(|q| text_to_vector(q));
        for kv in self.blocks.iter() {
            let (_key, value) = kv?;
            let mut block = match Self::deserialize_block(&value) {
                Ok(b) => b,
                Err(_) => continue,
            };
            // Inline filtering
            if let Some(ref user_id) = query.user_id {
                if block.user_id() != user_id {
                    continue;
                }
            }
            if let Some(ref session_id) = query.session_id {
                if block.session_id() != Some(session_id.as_str()) {
                    continue;
                }
            }
            if let Some(ref block_types) = query.block_types {
                if !block_types.contains(&block.block_type()) {
                    continue;
                }
            }
            if let Some(ref time_range) = query.time_range {
                let created = block.created_at();
                if let Some(start) = time_range.start {
                    if created < start {
                        continue;
                    }
                }
                if let Some(end) = time_range.end {
                    if created > end {
                        continue;
                    }
                }
            }

            // Compute cosine similarity relevance if content_query is present
            if let Some(ref query_vec) = content_query_vec {
                let content_vec =
                    text_to_vector(block.content().as_text().expect("Text content expected"));
                let score = cosine_similarity(query_vec, &content_vec);
                // Always set the computed relevance, even if already set
                block.set_relevance(Relevance::from(score));
            }
            results.push(block);
        }
        // Sort if requested
        if let Some(sort) = &query.sort {
            match sort {
                QuerySort::NewestFirst => {
                    results.sort_by_key(|b| std::cmp::Reverse(b.created_at()))
                }
                QuerySort::OldestFirst => results.sort_by_key(|b| b.created_at()),
                QuerySort::Relevance => {
                    results.sort_by(|a, b| {
                        b.relevance()
                            .unwrap()
                            .score()
                            .partial_cmp(&a.relevance().unwrap().score())
                            .unwrap_or(std::cmp::Ordering::Equal)
                    });
                }
            }
        }
        // Limit if requested
        if let Some(limit) = query.limit {
            results.truncate(limit);
        }
        Ok(results)
    }

    async fn list_blocks(&self, user_id: &str) -> Result<Vec<MemoryBlock>, Error> {
        let mut results = Vec::new();
        for kv in self.blocks.iter() {
            let (_key, value) = kv?;
            let block = match Self::deserialize_block(&value) {
                Ok(b) => b,
                Err(_) => continue,
            };
            if block.user_id() == user_id {
                results.push(block);
            }
        }
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::{MemoryBlockBuilder, MemoryContent};
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_store_and_retrieve() {
        let dir = tempdir().unwrap();
        let store = FjallMemoryStore::new(dir.path()).unwrap();

        let block = MemoryBlockBuilder::default()
            .with_user_id("user1")
            .with_session_id("sess1")
            .with_type(BlockType::Message)
            .with_content(MemoryContent::Text("hello world".to_string()))
            .build()
            .unwrap();

        let blockid = block.id().clone();

        store.store_block(block.clone()).await.unwrap();
        let retrieved = store.get_block(&blockid).await.unwrap();
        assert_eq!(Some(block), retrieved);
    }

    #[tokio::test]
    async fn test_delete() {
        let dir = tempdir().unwrap();
        let store = FjallMemoryStore::new(dir.path()).unwrap();

        let block = MemoryBlockBuilder::default()
            .with_user_id("user2")
            .with_session_id("sess2")
            .with_type(BlockType::Fact)
            .with_content(MemoryContent::Text("fact".to_string()))
            .build()
            .unwrap();

        let blockid = block.id().clone();

        store.store_block(block).await.unwrap();
        store.delete_block(&blockid).await.unwrap();
        let retrieved = store.get_block(&blockid).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_search_blocks() {
        let dir = tempdir().unwrap();
        let store = FjallMemoryStore::new(dir.path()).unwrap();

        let block1 = MemoryBlockBuilder::default()
            .with_user_id("alice")
            .with_session_id("s1")
            .with_type(BlockType::Message)
            .with_content(MemoryContent::Text("hello".to_string()))
            .build()
            .unwrap();

        let block2 = MemoryBlockBuilder::default()
            .with_user_id("bob")
            .with_session_id("s2")
            .with_type(BlockType::Fact)
            .with_content(MemoryContent::Text("fact".to_string()))
            .build()
            .unwrap();

        store.store_block(block1).await.unwrap();
        store.store_block(block2).await.unwrap();

        let query = MemoryQuery {
            user_id: Some("alice".to_string()),
            ..Default::default()
        };
        let results = store.search_blocks(&query).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].user_id(), "alice");
    }
}
