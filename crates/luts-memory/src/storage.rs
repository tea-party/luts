//! Storage implementation for memory blocks using SurrealDB
//!
//! This module provides the SurrealDB-based storage backend for memory blocks
//! with automatic embedding generation and vector similarity search.

use crate::{
    block::MemoryBlock,
    embeddings::{EmbeddingService, VectorSearchConfig},
    types::{BlockId, BlockType, MemoryContent},
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use luts_common::{LutsError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use surrealdb::{
    Surreal,
    engine::local::{Db, Mem, SurrealKv},
};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// A trait defining operations for a memory storage system
#[async_trait]
pub trait MemoryStore: Send + Sync {
    /// Store a memory block
    async fn store(&self, block: MemoryBlock) -> Result<BlockId>;

    /// Retrieve a memory block by its ID
    async fn retrieve(&self, id: &BlockId) -> Result<Option<MemoryBlock>>;

    /// Delete a memory block
    async fn delete(&self, id: &BlockId) -> Result<bool>;

    /// Update an existing memory block
    async fn update(&self, id: &BlockId, block: MemoryBlock) -> Result<MemoryBlock>;

    /// Search for memory blocks based on criteria
    async fn query(&self, query: MemoryQuery) -> Result<Vec<MemoryBlock>>;

    /// Clear all data for a specific user
    async fn clear_user_data(&self, user_id: &str) -> Result<u64>;

    /// Get statistics about memory usage
    async fn get_stats(&self, user_id: &str) -> Result<MemoryStats>;
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
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

/// SurrealDB configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SurrealConfig {
    /// File-based SurrealDB
    File {
        path: PathBuf,
        namespace: String,
        database: String,
    },
    /// Memory-based SurrealDB
    Memory { namespace: String, database: String },
}

impl Default for SurrealConfig {
    fn default() -> Self {
        SurrealConfig::File {
            path: PathBuf::from("./data/memory.db"),
            namespace: "luts".to_string(),
            database: "memory".to_string(),
        }
    }
}

/// Authentication configuration for SurrealDB
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub username: String,
    pub password: String,
}

/// Relationship types between memory blocks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationType {
    /// One block references another
    References,
    /// One block is derived from another
    DerivedFrom,
    /// Blocks are related in some way
    Related,
}

/// Enhanced memory block with embedding and metadata for SurrealDB
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedMemoryBlock {
    #[serde(default)]
    pub id: BlockId,
    pub user_id: String,
    pub session_id: Option<String>,
    pub block_type: String, // Store as string for SurrealDB compatibility
    pub content: String,    // Store content as JSON string
    pub tags: Vec<String>,
    pub embedding: Option<Vec<f32>>,  // For semantic search
    pub relevance_score: Option<f32>, // Dynamic relevance
    pub access_count: u64,            // Usage tracking
    pub last_accessed: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<MemoryBlock> for EnhancedMemoryBlock {
    fn from(block: MemoryBlock) -> Self {
        // Convert u64 timestamps to RFC3339 strings
        let created_at = DateTime::from_timestamp_millis(block.created_at() as i64)
            .unwrap_or_else(|| Utc::now())
            .to_rfc3339();
        let updated_at = DateTime::from_timestamp_millis(block.updated_at() as i64)
            .unwrap_or_else(|| Utc::now())
            .to_rfc3339();

        Self {
            id: block.id().clone(),
            user_id: block.user_id().to_string(),
            session_id: block.session_id().map(|s| s.to_string()),
            block_type: block.block_type().to_string(), // Use Display trait
            content: serde_json::to_string(block.content()).unwrap(), // Serialize content to JSON
            tags: block.tags().to_vec(),                // Convert &[String] to Vec<String>
            embedding: None,
            relevance_score: None,
            access_count: 0,
            last_accessed: Utc::now().to_rfc3339(),
            created_at,
            updated_at,
        }
    }
}

impl From<EnhancedMemoryBlock> for MemoryBlock {
    fn from(enhanced: EnhancedMemoryBlock) -> Self {
        use crate::block::MemoryBlockBuilder;

        // Parse block_type from string back to enum
        let block_type = match enhanced.block_type.as_str() {
            "message" => BlockType::Message,
            "summary" => BlockType::Summary,
            "fact" => BlockType::Fact,
            "preference" => BlockType::Preference,
            "personal_info" => BlockType::PersonalInfo,
            "goal" => BlockType::Goal,
            "task" => BlockType::Task,
            _ if enhanced.block_type.starts_with("custom_") => {
                let id_str = enhanced.block_type.strip_prefix("custom_").unwrap_or("0");
                let id = id_str.parse::<u8>().unwrap_or(0);
                BlockType::Custom(id)
            }
            _ => BlockType::Fact, // Default fallback
        };

        // Parse content from JSON string back to MemoryContent
        let content: MemoryContent = serde_json::from_str(&enhanced.content)
            .unwrap_or_else(|_| MemoryContent::Text(enhanced.content.clone()));

        let mut builder = MemoryBlockBuilder::new()
            .with_id(enhanced.id)
            .with_user_id(&enhanced.user_id)
            .with_type(block_type)
            .with_content(content)
            .with_tags(enhanced.tags);

        // Add session_id if present
        if let Some(session_id) = enhanced.session_id {
            builder = builder.with_session_id(&session_id);
        }

        builder
            .build()
            .expect("Enhanced block should always be valid")
    }
}

/// SurrealDB memory store implementation with automatic embedding generation
#[derive(Clone)]
pub struct SurrealMemoryStore {
    db: Surreal<Db>,
    _config: SurrealConfig,
    initialized: Arc<RwLock<bool>>,
    embedding_service: Option<Arc<dyn EmbeddingService>>,
}

impl SurrealMemoryStore {
    /// Create a new SurrealDB memory store
    pub async fn new(config: SurrealConfig) -> Result<Self> {
        Self::with_embedding_service(config, None).await
    }

    /// Create a new SurrealMemoryStore with optional embedding service
    pub async fn with_embedding_service(
        config: SurrealConfig,
        embedding_service: Option<Arc<dyn EmbeddingService>>,
    ) -> Result<Self> {
        let db = match &config {
            SurrealConfig::File { path, .. } => {
                debug!("Initializing SurrealDB in file mode at: {:?}", path);

                let db: Surreal<Db> =
                    Surreal::new::<SurrealKv>(path.clone()).await.map_err(|e| {
                        LutsError::Storage(format!(
                            "Failed to create SurrealDB file connection: {}",
                            e
                        ))
                    })?;

                let namespace = match &config {
                    SurrealConfig::File { namespace, .. } => namespace,
                    SurrealConfig::Memory { namespace, .. } => namespace,
                };
                let database = match &config {
                    SurrealConfig::File { database, .. } => database,
                    SurrealConfig::Memory { database, .. } => database,
                };

                db.use_ns(namespace).use_db(database).await.map_err(|e| {
                    LutsError::Storage(format!("Failed to set namespace/database: {}", e))
                })?;

                info!("SurrealDB initialized with file backend at: {:?}", path);
                db
            }
            SurrealConfig::Memory { .. } => {
                debug!("Initializing SurrealDB in memory mode");

                let db: Surreal<Db> = Surreal::new::<Mem>(()).await.map_err(|e| {
                    LutsError::Storage(format!("Failed to create SurrealDB connection: {}", e))
                })?;

                let namespace = match &config {
                    SurrealConfig::File { namespace, .. } => namespace,
                    SurrealConfig::Memory { namespace, .. } => namespace,
                };
                let database = match &config {
                    SurrealConfig::File { database, .. } => database,
                    SurrealConfig::Memory { database, .. } => database,
                };

                db.use_ns(namespace).use_db(database).await.map_err(|e| {
                    LutsError::Storage(format!("Failed to set namespace/database: {}", e))
                })?;

                info!("SurrealDB initialized with in-memory backend");
                db
            }
        };

        Ok(Self {
            db,
            _config: config,
            initialized: Arc::new(RwLock::new(false)),
            embedding_service,
        })
    }

    /// Get a clone of the underlying SurrealDB connection
    pub fn db(&self) -> Surreal<Db> {
        self.db.clone()
    }

    /// Initialize the database schema
    pub async fn initialize_schema(&self) -> Result<()> {
        self.initialize_schema_with_dimensions(1536).await
    }

    /// Initialize the database schema with custom embedding dimensions
    pub async fn initialize_schema_with_dimensions(
        &self,
        embedding_dimensions: usize,
    ) -> Result<()> {
        let mut initialized = self.initialized.write().await;
        if *initialized {
            return Ok(());
        }

        debug!(
            "Initializing SurrealDB schema with {} embedding dimensions...",
            embedding_dimensions
        );

        // Define the memory_blocks table
        self.db
            .query("DEFINE TABLE memory_blocks SCHEMALESS;")
            .await
            .map_err(|e| {
                LutsError::Storage(format!("Failed to define memory_blocks table: {}", e))
            })?;

        // Define indexes for performance with dynamic embedding dimensions
        let index_query = format!(
            "
            DEFINE INDEX user_blocks ON memory_blocks FIELDS user_id, block_type;
            DEFINE INDEX session_blocks ON memory_blocks FIELDS session_id, created_at;
            DEFINE INDEX tag_search ON memory_blocks FIELDS tags;
            DEFINE INDEX embedding_vector ON memory_blocks FIELDS embedding MTREE DIMENSION {};
        ",
            embedding_dimensions
        );

        self.db
            .query(&index_query)
            .await
            .map_err(|e| LutsError::Storage(format!("Failed to create indexes: {}", e)))?;

        *initialized = true;
        info!("SurrealDB schema initialized successfully");
        Ok(())
    }

    /// Update access count for a memory block (for usage tracking)
    async fn update_access_count(&self, id: &BlockId) -> Result<()> {
        let block_id_string = id.as_str().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        self.db
            .query("UPDATE type::thing('memory_blocks', $block_id) SET access_count += 1, last_accessed = $now")
            .bind(("block_id", block_id_string))
            .bind(("now", now))
            .await
            .map_err(|e| LutsError::Storage(format!("Failed to update access count: {}", e)))?;

        Ok(())
    }

    /// Perform vector similarity search using SurrealDB MTREE index
    async fn vector_similarity_search(
        &self,
        vector_query: &VectorQuery,
        query: &MemoryQuery,
    ) -> Result<Vec<MemoryBlock>> {
        let mut conditions = Vec::new();
        let mut bindings = Vec::new();

        // Add non-vector filters
        if let Some(user_id) = &query.user_id {
            conditions.push("user_id = $user_id".to_string());
            bindings.push(("user_id", user_id.clone()));
        }

        if let Some(session_id) = &query.session_id {
            conditions.push("session_id = $session_id".to_string());
            bindings.push(("session_id", session_id.clone()));
        }

        if !query.block_types.is_empty() {
            let types: Vec<String> = query.block_types.iter().map(|t| t.to_string()).collect();
            conditions.push("block_type IN $block_types".to_string());
            bindings.push(("block_types", serde_json::to_string(&types).unwrap()));
        }

        // Build the vector search query using SurrealDB's vector capabilities
        let where_clause = if conditions.is_empty() {
            "WHERE embedding IS NOT NULL".to_string()
        } else {
            format!(
                "WHERE {} AND embedding IS NOT NULL",
                conditions.join(" AND ")
            )
        };

        let max_results = vector_query.search_config.max_results.min(1000); // Cap at 1000 for performance
        let min_relevance = vector_query.search_config.min_relevance;

        // Use SurrealDB's vector similarity functions
        let sql_query = format!(
            "SELECT *, vector::similarity::cosine(embedding, $query_vector) AS similarity_score
             FROM memory_blocks
             {}
             ORDER BY similarity_score DESC
             LIMIT {}",
            where_clause, max_results
        );

        let mut db_query = self.db.query(&sql_query);
        db_query = db_query.bind(("query_vector", vector_query.query_vector.clone()));

        for (key, value) in bindings {
            db_query = db_query.bind((key, value));
        }

        let mut response = db_query
            .await
            .map_err(|e| LutsError::Storage(format!("Failed to perform vector search: {}", e)))?;

        let results: Vec<serde_json::Value> = response.take(0).map_err(|e| {
            LutsError::Storage(format!("Failed to parse vector search results: {}", e))
        })?;

        let mut memory_blocks = Vec::new();

        for result in results {
            // Extract similarity score
            let similarity_score = result
                .get("similarity_score")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0) as f32;

            // Filter by minimum relevance threshold
            if similarity_score < min_relevance {
                continue;
            }

            // Parse the enhanced memory block
            let mut enhanced_block: EnhancedMemoryBlock =
                serde_json::from_value(result).map_err(|e| {
                    LutsError::Storage(format!("Failed to parse enhanced memory block: {}", e))
                })?;

            // Set the relevance score
            enhanced_block.relevance_score = Some(similarity_score);

            // Convert to MemoryBlock and add to results
            memory_blocks.push(enhanced_block.into());
        }

        debug!(
            "🔍 Vector search found {} blocks with min_relevance >= {}",
            memory_blocks.len(),
            min_relevance
        );

        Ok(memory_blocks)
    }

    /// Semantic search by generating embeddings for query text and finding similar blocks
    pub async fn semantic_search(
        &self,
        query_text: &str,
        config: VectorSearchConfig,
        user_id: Option<&str>,
    ) -> Result<Vec<MemoryBlock>> {
        if let Some(embedding_service) = &self.embedding_service {
            // Generate embedding for the query text
            let query_embedding = embedding_service.embed_text(query_text).await?;

            // Build the search query
            let vector_query = VectorQuery {
                query_vector: query_embedding,
                search_config: config,
            };

            let memory_query = MemoryQuery {
                user_id: user_id.map(|s| s.to_string()),
                vector_search: Some(vector_query),
                ..Default::default()
            };

            self.query(memory_query).await
        } else {
            Err(LutsError::Memory(
                "No embedding service available for semantic search".to_string(),
            ))
        }
    }
}

#[async_trait]
impl MemoryStore for SurrealMemoryStore {
    async fn store(&self, block: MemoryBlock) -> Result<BlockId> {
        self.initialize_schema().await?;

        let mut enhanced_block = EnhancedMemoryBlock::from(block);
        let block_id = enhanced_block.id.clone();

        // 🚀 AUTOMATIC EMBEDDING GENERATION 🚀
        // Generate embedding if embedding service is available and block doesn't have one
        if enhanced_block.embedding.is_none() {
            if let Some(embedding_service) = &self.embedding_service {
                // Extract text content from the serialized JSON content
                let text_content = if let Ok(original_content) =
                    serde_json::from_str::<MemoryContent>(&enhanced_block.content)
                {
                    match original_content {
                        MemoryContent::Text(text) => text,
                        MemoryContent::Json(json) => json.to_string(),
                        MemoryContent::Binary { .. } => {
                            // Skip embedding for binary content
                            warn!(
                                "Skipping embedding generation for binary content in block {}",
                                block_id.as_str()
                            );
                            String::new()
                        }
                    }
                } else {
                    // Fallback: treat the content string as plain text
                    enhanced_block.content.clone()
                };

                if !text_content.is_empty() {
                    match embedding_service.embed_text(&text_content).await {
                        Ok(embedding) => {
                            enhanced_block.embedding = Some(embedding);
                            debug!(
                                "✅ Generated embedding for block {} (content: {}...)",
                                block_id.as_str(),
                                text_content.chars().take(50).collect::<String>()
                            );
                        }
                        Err(e) => {
                            warn!(
                                "❌ Failed to generate embedding for block {}: {}",
                                block_id.as_str(),
                                e
                            );
                            // Continue without embedding rather than failing the entire operation
                        }
                    }
                }
            } else {
                debug!(
                    "No embedding service available for block {}",
                    block_id.as_str()
                );
            }
        }

        info!(
            "📦 Stored memory block {} with {} embedding",
            block_id.as_str(),
            if enhanced_block.embedding.is_some() {
                "✅"
            } else {
                "❌"
            }
        );

        // Store the enhanced block with embedding in SurrealDB
        let block_id_string = block_id.as_str().to_string();
        self.db
            .query(
                "CREATE type::thing('memory_blocks', $block_id) SET
                    user_id = $user_id,
                    session_id = $session_id,
                    block_type = $block_type,
                    content = $content,
                    tags = $tags,
                    embedding = $embedding,
                    relevance_score = $relevance_score,
                    access_count = $access_count,
                    last_accessed = $last_accessed,
                    created_at = $created_at,
                    updated_at = $updated_at",
            )
            .bind(("block_id", block_id_string))
            .bind(("user_id", enhanced_block.user_id))
            .bind(("session_id", enhanced_block.session_id))
            .bind(("block_type", enhanced_block.block_type))
            .bind(("content", enhanced_block.content))
            .bind(("tags", enhanced_block.tags))
            .bind(("embedding", enhanced_block.embedding))
            .bind(("relevance_score", enhanced_block.relevance_score))
            .bind(("access_count", enhanced_block.access_count))
            .bind(("last_accessed", enhanced_block.last_accessed))
            .bind(("created_at", enhanced_block.created_at))
            .bind(("updated_at", enhanced_block.updated_at))
            .await
            .map_err(|e| LutsError::Storage(format!("Failed to store memory block: {}", e)))?;

        Ok(block_id)
    }

    async fn retrieve(&self, id: &BlockId) -> Result<Option<MemoryBlock>> {
        self.initialize_schema().await?;

        let block_id_string = id.as_str().to_string();
        let mut response = self
            .db
            .query("SELECT * FROM type::thing('memory_blocks', $block_id)")
            .bind(("block_id", block_id_string))
            .await
            .map_err(|e| LutsError::Storage(format!("Failed to retrieve memory block: {}", e)))?;

        let enhanced_blocks: Vec<EnhancedMemoryBlock> = response
            .take(0)
            .map_err(|e| LutsError::Storage(format!("Failed to parse memory block: {}", e)))?;

        match enhanced_blocks.into_iter().next() {
            Some(enhanced_block) => {
                // Update access tracking
                let _ = self.update_access_count(id).await;
                Ok(Some(enhanced_block.into()))
            }
            None => Ok(None),
        }
    }

    async fn delete(&self, _id: &BlockId) -> Result<bool> {
        // In real implementation, this would delete the block from SurrealDB
        Ok(false)
    }

    async fn update(&self, _id: &BlockId, block: MemoryBlock) -> Result<MemoryBlock> {
        // In real implementation, this would update the block in SurrealDB
        Ok(block)
    }

    async fn query(&self, query: MemoryQuery) -> Result<Vec<MemoryBlock>> {
        self.initialize_schema().await?;

        let mut conditions = Vec::new();
        let mut bindings = Vec::new();

        // Build WHERE conditions
        if let Some(user_id) = &query.user_id {
            conditions.push("user_id = $user_id".to_string());
            bindings.push(("user_id", user_id.clone()));
        }

        if let Some(session_id) = &query.session_id {
            conditions.push("session_id = $session_id".to_string());
            bindings.push(("session_id", session_id.clone()));
        }

        if !query.block_types.is_empty() {
            let types: Vec<String> = query.block_types.iter().map(|t| t.to_string()).collect();
            conditions.push("block_type IN $block_types".to_string());
            bindings.push(("block_types", serde_json::to_string(&types).unwrap()));
        }

        if let Some(content) = &query.content_contains {
            conditions.push("content CONTAINS $content".to_string());
            bindings.push(("content", content.clone()));
        }

        // Handle vector similarity search
        if let Some(vector_query) = &query.vector_search {
            return self.vector_similarity_search(vector_query, &query).await;
        }

        // Build the query
        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", conditions.join(" AND "))
        };

        let order_clause = match query.sort.unwrap_or_default() {
            QuerySort::NewestFirst => " ORDER BY created_at DESC",
            QuerySort::OldestFirst => " ORDER BY created_at ASC",
            QuerySort::Relevance => " ORDER BY relevance_score DESC",
        };

        let limit_clause = query
            .limit
            .map(|l| format!(" LIMIT {}", l))
            .unwrap_or_default();

        let sql_query = format!(
            "SELECT * FROM memory_blocks{}{}{}",
            where_clause, order_clause, limit_clause
        );

        let mut db_query = self.db.query(&sql_query);
        for (key, value) in bindings {
            db_query = db_query.bind((key, value));
        }

        let mut response = db_query
            .await
            .map_err(|e| LutsError::Storage(format!("Failed to query memory blocks: {}", e)))?;

        let enhanced_blocks: Vec<EnhancedMemoryBlock> = response
            .take(0)
            .map_err(|e| LutsError::Storage(format!("Failed to parse memory blocks: {}", e)))?;

        Ok(enhanced_blocks.into_iter().map(|eb| eb.into()).collect())
    }

    async fn clear_user_data(&self, _user_id: &str) -> Result<u64> {
        // In real implementation, this would delete all blocks for the user
        Ok(0)
    }

    async fn get_stats(&self, _user_id: &str) -> Result<MemoryStats> {
        // In real implementation, this would calculate statistics from SurrealDB
        Ok(MemoryStats {
            total_blocks: 0,
            blocks_by_type: HashMap::new(),
            total_size_bytes: 0,
            last_updated: Utc::now(),
        })
    }
}

/// A memory manager that interfaces with a storage backend
pub struct MemoryManager {
    store: Box<dyn MemoryStore>,
}

impl MemoryManager {
    /// Create a new memory manager with the given store
    pub fn new(store: impl MemoryStore + 'static) -> Self {
        MemoryManager {
            store: Box::new(store),
        }
    }

    /// Store a memory block
    pub async fn store(&self, block: MemoryBlock) -> Result<BlockId> {
        self.store.store(block).await
    }

    /// Retrieve a memory block by its ID
    pub async fn get(&self, id: &BlockId) -> Result<Option<MemoryBlock>> {
        self.store.retrieve(id).await
    }

    /// Delete a memory block
    pub async fn delete(&self, id: &BlockId) -> Result<bool> {
        self.store.delete(id).await
    }

    /// Update an existing memory block
    pub async fn update(&self, id: &BlockId, block: MemoryBlock) -> Result<MemoryBlock> {
        self.store.update(id, block).await
    }

    /// Search for memory blocks based on criteria
    pub async fn search(&self, query: &MemoryQuery) -> Result<Vec<MemoryBlock>> {
        self.store.query(query.clone()).await
    }

    /// List all memory blocks for a user
    pub async fn list(&self, user_id: &str) -> Result<Vec<MemoryBlock>> {
        let query = MemoryQuery {
            user_id: Some(user_id.to_string()),
            ..Default::default()
        };
        self.store.query(query).await
    }

    /// Clear all data for a user
    pub async fn clear_user_data(&self, user_id: &str) -> Result<u64> {
        self.store.clear_user_data(user_id).await
    }

    /// Get memory usage statistics
    pub async fn get_stats(&self, user_id: &str) -> Result<MemoryStats> {
        self.store.get_stats(user_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_surreal_memory_store_creation() {
        let config = SurrealConfig::Memory {
            namespace: "test".to_string(),
            database: "memory".to_string(),
        };

        let store = SurrealMemoryStore::new(config).await.unwrap();
        store.initialize_schema_with_dimensions(384).await.unwrap();
    }

    #[tokio::test]
    async fn test_store_and_retrieve_with_embeddings() {
        use crate::embeddings::{EmbeddingConfig, EmbeddingProvider};
        use crate::types::MemoryContent;

        let config = SurrealConfig::Memory {
            namespace: "test".to_string(),
            database: "memory".to_string(),
        };

        // Create embedding service for testing
        let embedding_config = EmbeddingConfig {
            provider: EmbeddingProvider::Mock,
            dimensions: 384,
            ..Default::default()
        };
        let embedding_service = crate::embeddings::MockEmbeddingService::new(embedding_config);

        let store =
            SurrealMemoryStore::with_embedding_service(config, Some(Arc::new(embedding_service)))
                .await
                .unwrap();
        store.initialize_schema_with_dimensions(384).await.unwrap();

        // Create a test memory block with text content
        let block = MemoryBlock::new(
            BlockType::Fact,
            "test_user",
            MemoryContent::Text("This is a test fact about machine learning".to_string()),
        );

        let block_id = store.store(block).await.unwrap();

        // Test retrieval
        let retrieved = store.retrieve(&block_id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id(), &block_id);
    }
}
