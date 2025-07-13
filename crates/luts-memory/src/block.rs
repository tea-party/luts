//! Memory block implementation
//!
//! This module provides the core `MemoryBlock` structure and related types.

use crate::types::{BlockId, BlockType, MemoryContent, Relevance};
use luts_common::{LutsError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Metadata for a memory block
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryBlockMetadata {
    /// Unique identifier for the block
    pub id: BlockId,

    /// Type of the block
    pub block_type: BlockType,

    /// ID of the user this block belongs to
    pub user_id: String,

    /// ID of the session this block belongs to (if applicable)
    pub session_id: Option<String>,

    /// Creation time as Unix timestamp (milliseconds)
    pub created_at: u64,

    /// Last modification time as Unix timestamp (milliseconds)
    pub updated_at: u64,

    /// IDs of blocks that this block references
    pub reference_ids: Vec<BlockId>,

    /// Custom tags for the block
    pub tags: Vec<String>,

    /// Custom properties for the block
    pub properties: HashMap<String, serde_json::Value>,

    /// Relevance score for the block (optional)
    pub relevance: Option<Relevance>,
}

/// A memory block that contains content and metadata
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryBlock {
    /// Metadata for the block
    pub metadata: MemoryBlockMetadata,

    /// Content of the block
    pub content: MemoryContent,
}

impl MemoryBlock {
    /// Create a new memory block
    pub fn new(block_type: BlockType, user_id: impl Into<String>, content: MemoryContent) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        MemoryBlock {
            metadata: MemoryBlockMetadata {
                id: BlockId::generate(),
                block_type,
                user_id: user_id.into(),
                session_id: None,
                created_at: now,
                updated_at: now,
                reference_ids: Vec::new(),
                tags: Vec::new(),
                properties: HashMap::new(),
                relevance: None,
            },
            content,
        }
    }

    /// Get the block ID
    pub fn id(&self) -> &BlockId {
        &self.metadata.id
    }

    /// Get the block type
    pub fn block_type(&self) -> BlockType {
        self.metadata.block_type
    }

    /// Get the user ID
    pub fn user_id(&self) -> &str {
        &self.metadata.user_id
    }

    /// Get the session ID if available
    pub fn session_id(&self) -> Option<&str> {
        self.metadata.session_id.as_deref()
    }

    /// Get the creation time
    pub fn created_at(&self) -> u64 {
        self.metadata.created_at
    }

    /// Get the last modification time
    pub fn updated_at(&self) -> u64 {
        self.metadata.updated_at
    }

    /// Get the reference block IDs
    pub fn reference_ids(&self) -> &[BlockId] {
        &self.metadata.reference_ids
    }

    /// Get the tags
    pub fn tags(&self) -> &[String] {
        &self.metadata.tags
    }

    /// Get the properties
    pub fn properties(&self) -> &HashMap<String, serde_json::Value> {
        &self.metadata.properties
    }

    /// Get the relevance score if available
    pub fn relevance(&self) -> Option<Relevance> {
        self.metadata.relevance
    }

    /// Get the content
    pub fn content(&self) -> &MemoryContent {
        &self.content
    }

    /// Set a new content and update the modification time
    pub fn set_content(&mut self, content: MemoryContent) {
        self.content = content;
        self.metadata.updated_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
    }

    /// Add a reference to another block
    pub fn add_reference(&mut self, id: BlockId) {
        if !self.metadata.reference_ids.contains(&id) {
            self.metadata.reference_ids.push(id);
            self.metadata.updated_at = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;
        }
    }

    /// Add a tag
    pub fn add_tag(&mut self, tag: impl Into<String>) {
        let tag = tag.into();
        if !self.metadata.tags.contains(&tag) {
            self.metadata.tags.push(tag);
            self.metadata.updated_at = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;
        }
    }

    /// Set a property
    pub fn set_property(&mut self, key: impl Into<String>, value: impl Into<serde_json::Value>) {
        self.metadata.properties.insert(key.into(), value.into());
        self.metadata.updated_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
    }

    /// Set the relevance score
    pub fn set_relevance(&mut self, relevance: Relevance) {
        self.metadata.relevance = Some(relevance);
        self.metadata.updated_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
    }

    /// Get a property value by key
    pub fn get_property(&self, key: &str) -> Option<&serde_json::Value> {
        self.metadata.properties.get(key)
    }

    /// Remove a tag if it exists
    pub fn remove_tag(&mut self, tag: &str) {
        if let Some(pos) = self.metadata.tags.iter().position(|t| t == tag) {
            self.metadata.tags.remove(pos);
            self.metadata.updated_at = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;
        }
    }
}

/// Builder for creating memory blocks
pub struct MemoryBlockBuilder {
    id: Option<BlockId>,
    block_type: Option<BlockType>,
    user_id: Option<String>,
    session_id: Option<String>,
    created_at: Option<u64>,
    reference_ids: Vec<BlockId>,
    tags: Vec<String>,
    properties: HashMap<String, serde_json::Value>,
    relevance: Option<Relevance>,
    content: Option<MemoryContent>,
}

impl MemoryBlockBuilder {
    /// Create a new memory block builder
    pub fn new() -> Self {
        MemoryBlockBuilder {
            id: None,
            block_type: None,
            user_id: None,
            session_id: None,
            created_at: None,
            reference_ids: Vec::new(),
            tags: Vec::new(),
            properties: HashMap::new(),
            relevance: None,
            content: None,
        }
    }

    /// Set a custom ID (otherwise one will be generated)
    pub fn with_id(mut self, id: impl Into<BlockId>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set the block type
    pub fn with_type(mut self, block_type: BlockType) -> Self {
        self.block_type = Some(block_type);
        self
    }

    /// Set the user ID
    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    /// Set the session ID
    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// Set the creation time (otherwise current time will be used)
    pub fn with_created_at(mut self, timestamp: u64) -> Self {
        self.created_at = Some(timestamp);
        self
    }

    /// Add reference IDs
    pub fn with_reference_ids(mut self, ids: Vec<BlockId>) -> Self {
        self.reference_ids.extend(ids);
        self
    }

    /// Add a reference ID
    pub fn with_reference_id(mut self, id: impl Into<BlockId>) -> Self {
        self.reference_ids.push(id.into());
        self
    }

    /// Add tags
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags.extend(tags);
        self
    }

    /// Add a tag
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Add a property
    pub fn with_property(
        mut self,
        key: impl Into<String>,
        value: impl Into<serde_json::Value>,
    ) -> Self {
        self.properties.insert(key.into(), value.into());
        self
    }

    /// Set the relevance score
    pub fn with_relevance(mut self, relevance: impl Into<Relevance>) -> Self {
        self.relevance = Some(relevance.into());
        self
    }

    /// Set the content
    pub fn with_content(mut self, content: MemoryContent) -> Self {
        self.content = Some(content);
        self
    }

    /// Build the memory block
    pub fn build(self) -> Result<MemoryBlock> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let block_type = self
            .block_type
            .ok_or_else(|| LutsError::Memory("Block type is required".to_string()))?;
        let user_id = self
            .user_id
            .ok_or_else(|| LutsError::Memory("User ID is required".to_string()))?;
        let content = self
            .content
            .ok_or_else(|| LutsError::Memory("Content is required".to_string()))?;

        let created_at = self.created_at.unwrap_or(now);

        Ok(MemoryBlock {
            metadata: MemoryBlockMetadata {
                id: self.id.unwrap_or_else(BlockId::generate),
                block_type,
                user_id,
                session_id: self.session_id,
                created_at,
                updated_at: now,
                reference_ids: self.reference_ids,
                tags: self.tags,
                properties: self.properties,
                relevance: self.relevance,
            },
            content,
        })
    }
}

impl Default for MemoryBlockBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_block_builder() {
        let block = MemoryBlockBuilder::new()
            .with_type(BlockType::Message)
            .with_user_id("user123")
            .with_session_id("session456")
            .with_content(MemoryContent::Text("Hello, world!".to_string()))
            .with_tag("greeting")
            .with_reference_id("block_prev")
            .with_property("importance", "high")
            .build()
            .unwrap();

        assert_eq!(block.block_type(), BlockType::Message);
        assert_eq!(block.user_id(), "user123");
        assert_eq!(block.session_id(), Some("session456"));
        assert_eq!(block.content().as_text().unwrap(), "Hello, world!");
        assert!(block.tags().contains(&"greeting".to_string()));
        assert_eq!(block.reference_ids().len(), 1);
        assert_eq!(block.reference_ids()[0].as_str(), "block_prev");
        assert_eq!(
            block
                .properties()
                .get("importance")
                .and_then(|v| v.as_str()),
            Some("high")
        );
    }

    #[test]
    fn test_memory_block_modifications() {
        let mut block = MemoryBlock::new(
            BlockType::Fact,
            "user123",
            MemoryContent::Text("Original content".to_string()),
        );

        let original_updated_at = block.updated_at();
        std::thread::sleep(std::time::Duration::from_millis(10));

        block.set_content(MemoryContent::Text("Updated content".to_string()));
        assert_ne!(block.updated_at(), original_updated_at);
        assert_eq!(block.content().as_text().unwrap(), "Updated content");

        let updated_at = block.updated_at();
        std::thread::sleep(std::time::Duration::from_millis(10));

        block.add_tag("important");
        assert_ne!(block.updated_at(), updated_at);
        assert!(block.tags().contains(&"important".to_string()));
    }

    #[test]
    fn test_memory_block_builder_failures() {
        // Missing type should fail
        let result = MemoryBlockBuilder::new()
            .with_user_id("user123")
            .with_content(MemoryContent::Text("Hello".to_string()))
            .build();
        assert!(result.is_err());

        // Missing user_id should fail
        let result = MemoryBlockBuilder::new()
            .with_type(BlockType::Message)
            .with_content(MemoryContent::Text("Hello".to_string()))
            .build();
        assert!(result.is_err());

        // Missing content should fail
        let result = MemoryBlockBuilder::new()
            .with_type(BlockType::Message)
            .with_user_id("user123")
            .build();
        assert!(result.is_err());
    }
}
