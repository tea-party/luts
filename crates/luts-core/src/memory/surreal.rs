//! SurrealDB-based memory storage implementation
//!
//! This module provides a SurrealDB backend for memory storage, offering
//! enhanced querying, relationships, vector search, and real-time features 
//! compared to the basic FjallMemoryStore.

use crate::memory::{
    BlockId, BlockType, MemoryBlock, MemoryBlockBuilder, MemoryBlockMetadata, MemoryContent,
    MemoryQuery, MemoryStore, Relevance, VectorQuery,
    EmbeddingService, EmbeddingConfig, EmbeddingServiceFactory, VectorSimilarity,
};
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use surrealdb::sql::Thing;
use surrealdb::{
    Surreal,
    engine::local::{Db, Mem},
};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Configuration for SurrealDB connection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SurrealConfig {
    /// Embedded file mode (default for local usage)
    File {
        path: PathBuf,
        namespace: String,
        database: String,
    },
    /// Local server mode (for development with SurrealDB server)
    Local {
        host: String,
        port: u16,
        namespace: String,
        database: String,
    },
    /// Remote server mode (for production deployments)
    Remote {
        url: String,
        namespace: String,
        database: String,
        auth: AuthConfig,
    },
}

/// Authentication configuration for remote SurrealDB
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthConfig {
    RootAuth {
        username: String,
        password: String,
    },
    NamespaceAuth {
        namespace: String,
        username: String,
        password: String,
    },
    DatabaseAuth {
        namespace: String,
        database: String,
        username: String,
        password: String,
    },
}

impl Default for SurrealConfig {
    fn default() -> Self {
        Self::File {
            path: PathBuf::from("./data/memory.db"),
            namespace: "luts".to_string(),
            database: "memory".to_string(),
        }
    }
}

/// Enhanced memory block with relationship support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedMemoryBlock {
    #[serde(default)] // Use default value if not present during deserialization
    pub id: BlockId,
    pub user_id: String,
    pub session_id: Option<String>,
    pub block_type: BlockType,
    pub content: MemoryContent,
    pub metadata: MemoryBlockMetadata,
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
        let metadata = block.metadata.clone();
        Self {
            id: metadata.id.clone(),
            user_id: metadata.user_id.clone(),
            session_id: metadata.session_id.clone(),
            block_type: metadata.block_type,
            content: block.content,
            metadata: metadata.clone(),
            tags: metadata.tags.clone(),
            embedding: None,
            relevance_score: None,
            access_count: 0,
            last_accessed: chrono::DateTime::from_timestamp_millis(metadata.updated_at as i64)
                .unwrap_or_else(|| chrono::Utc::now())
                .to_rfc3339(),
            created_at: chrono::DateTime::from_timestamp_millis(metadata.created_at as i64)
                .unwrap_or_else(|| chrono::Utc::now())
                .to_rfc3339(),
            updated_at: chrono::DateTime::from_timestamp_millis(metadata.updated_at as i64)
                .unwrap_or_else(|| chrono::Utc::now())
                .to_rfc3339(),
        }
    }
}

impl From<EnhancedMemoryBlock> for MemoryBlock {
    fn from(enhanced: EnhancedMemoryBlock) -> Self {
        Self {
            metadata: MemoryBlockMetadata {
                id: enhanced.id,
                block_type: enhanced.block_type,
                user_id: enhanced.user_id,
                session_id: enhanced.session_id,
                created_at: chrono::DateTime::parse_from_rfc3339(&enhanced.created_at)
                    .map(|dt| dt.timestamp_millis() as u64)
                    .unwrap_or_else(|_| chrono::Utc::now().timestamp_millis() as u64),
                updated_at: chrono::DateTime::parse_from_rfc3339(&enhanced.updated_at)
                    .map(|dt| dt.timestamp_millis() as u64)
                    .unwrap_or_else(|_| chrono::Utc::now().timestamp_millis() as u64),
                reference_ids: enhanced.metadata.reference_ids,
                tags: enhanced.tags,
                properties: enhanced.metadata.properties,
                relevance: enhanced.relevance_score.map(|s| Relevance::from(s)),
            },
            content: enhanced.content,
        }
    }
}

/// Relationship types between memory blocks
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RelationType {
    References,  // Block A references information in Block B
    Contradicts, // Block A contradicts Block B
    Supports,    // Block A supports/confirms Block B
    FollowsFrom, // Block A is a logical consequence of Block B
    Generalizes, // Block A is a generalization of Block B
    Specializes, // Block A is a specialization of Block B
    Temporal,    // Block A happens before/after Block B
    Similarity,  // Block A is semantically similar to Block B
}

impl std::fmt::Display for RelationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RelationType::References => write!(f, "references"),
            RelationType::Contradicts => write!(f, "contradicts"),
            RelationType::Supports => write!(f, "supports"),
            RelationType::FollowsFrom => write!(f, "follows_from"),
            RelationType::Generalizes => write!(f, "generalizes"),
            RelationType::Specializes => write!(f, "specializes"),
            RelationType::Temporal => write!(f, "temporal"),
            RelationType::Similarity => write!(f, "similarity"),
        }
    }
}

/// Raw database representation with string-serialized fields
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RawMemoryBlock {
    #[serde(rename = "id")]
    pub record_id: surrealdb::sql::Thing, // SurrealDB returns Thing objects
    pub user_id: String,
    pub session_id: Option<String>,
    pub block_type: String,
    pub content: String,           // JSON serialized MemoryContent
    pub metadata: String,          // JSON serialized MemoryBlockMetadata
    pub tags: String,              // JSON serialized Vec<String>
    pub embedding: Option<Vec<f32>>, // Vector embedding for similarity search
    pub relevance_score: Option<f32>,
    pub access_count: u64,
    pub last_accessed: String,
    pub created_at: String,
    pub updated_at: String,
}

impl RawMemoryBlock {
    pub fn to_enhanced(self) -> Result<EnhancedMemoryBlock> {
        let content: MemoryContent = serde_json::from_str(&self.content)
            .map_err(|e| anyhow!("Failed to deserialize content: {}", e))?;
        let metadata: MemoryBlockMetadata = serde_json::from_str(&self.metadata)
            .map_err(|e| anyhow!("Failed to deserialize metadata: {}", e))?;
        let tags: Vec<String> = serde_json::from_str(&self.tags)
            .map_err(|e| anyhow!("Failed to deserialize tags: {}", e))?;
        // Embedding is now stored directly as Vec<f32>, not as JSON string
        let embedding = self.embedding;

        // Parse block type from debug string
        let block_type = match self.block_type.as_str() {
            "Message" => BlockType::Message,
            "Summary" => BlockType::Summary,
            "Fact" => BlockType::Fact,
            "Preference" => BlockType::Preference,
            "PersonalInfo" => BlockType::PersonalInfo,
            "Goal" => BlockType::Goal,
            "Task" => BlockType::Task,
            s if s.starts_with("Custom(") => {
                let num_str = s.trim_start_matches("Custom(").trim_end_matches(")");
                let num = num_str
                    .parse::<u8>()
                    .map_err(|e| anyhow!("Failed to parse custom block type: {}", e))?;
                BlockType::Custom(num)
            }
            _ => return Err(anyhow!("Unknown block type: {}", self.block_type)),
        };

        // Extract the string ID from the Thing
        let id_string = match &self.record_id.id {
            surrealdb::sql::Id::String(s) => s.clone(),
            surrealdb::sql::Id::Number(n) => n.to_string(),
            surrealdb::sql::Id::Array(a) => format!("{:?}", a),
            surrealdb::sql::Id::Object(o) => format!("{:?}", o),
            surrealdb::sql::Id::Generate(_) => return Err(anyhow!("Cannot handle generated IDs")),
            _ => return Err(anyhow!("Unsupported ID type")),
        };

        Ok(EnhancedMemoryBlock {
            id: BlockId::from(id_string),
            user_id: self.user_id,
            session_id: self.session_id,
            block_type,
            content,
            metadata,
            tags,
            embedding,
            relevance_score: self.relevance_score,
            access_count: self.access_count,
            last_accessed: self.last_accessed,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

/// Memory statistics aggregated from the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStats {
    pub total_blocks: u64,
    pub blocks_by_type: HashMap<String, u64>,
    pub total_size_bytes: u64,
    pub average_access_count: f64,
    pub most_accessed_blocks: Vec<(BlockId, u64)>,
}

/// SurrealDB implementation of MemoryStore
pub struct SurrealMemoryStore {
    db: Surreal<Db>,
    config: SurrealConfig,
    initialized: Arc<RwLock<bool>>,
    embedding_service: Option<Arc<dyn EmbeddingService>>,
}

impl SurrealMemoryStore {
    /// Create a new SurrealMemoryStore with the given configuration
    pub async fn new(config: SurrealConfig) -> Result<Self> {
        Self::with_embedding_service(config, None).await
    }

    /// Create a new SurrealMemoryStore with optional embedding service
    pub async fn with_embedding_service(
        config: SurrealConfig,
        embedding_service: Option<Arc<dyn EmbeddingService>>,
    ) -> Result<Self> {
        let db = match &config {
            SurrealConfig::File {
                path,
                namespace,
                database,
            } => {
                debug!("Initializing SurrealDB in memory mode (avoiding surrealkv vector issues)");

                // Use in-memory storage to avoid surrealkv vector deserialization issues
                // This is suitable for testing and development
                let db: Surreal<Db> = Surreal::new::<Mem>(())
                    .await
                    .map_err(|e| anyhow!("Failed to create SurrealDB memory connection: {}", e))?;

                db.use_ns(namespace)
                    .use_db(database)
                    .await
                    .map_err(|e| anyhow!("Failed to set namespace/database: {}", e))?;

                info!("SurrealDB initialized with in-memory backend (avoiding surrealkv vector issues)");
                db
            }
            SurrealConfig::Local { .. } => {
                return Err(anyhow!("Local server mode not yet implemented"));
            }
            SurrealConfig::Remote { .. } => {
                return Err(anyhow!("Remote server mode not yet implemented"));
            }
        };

        Ok(Self {
            db,
            config,
            initialized: Arc::new(RwLock::new(false)),
            embedding_service,
        })
    }

    /// Initialize the database schema and tables
    pub async fn initialize_schema(&self) -> Result<()> {
        self.initialize_schema_with_dimensions(1536).await
    }

    /// Initialize the database schema with custom embedding dimensions
    pub async fn initialize_schema_with_dimensions(&self, embedding_dimensions: usize) -> Result<()> {
        let mut initialized = self.initialized.write().await;
        if *initialized {
            return Ok(());
        }

        debug!("Initializing SurrealDB schema with {} embedding dimensions...", embedding_dimensions);

        // Define the memory_blocks table
        self.db
            .query(
                "
            DEFINE TABLE memory_blocks SCHEMALESS;
        ",
            )
            .await
            .map_err(|e| anyhow!("Failed to define memory_blocks table: {}", e))?;

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
            .map_err(|e| anyhow!("Failed to create indexes: {}", e))?;

        // Define the block_relations table for relationships
        self.db
            .query(
                "
            DEFINE TABLE block_relations SCHEMAFULL;
            DEFINE FIELD in ON block_relations TYPE record<memory_blocks>;
            DEFINE FIELD out ON block_relations TYPE record<memory_blocks>;
            DEFINE FIELD relation_type ON block_relations TYPE string;
            DEFINE FIELD strength ON block_relations TYPE float DEFAULT 1.0;
            DEFINE FIELD created_at ON block_relations TYPE datetime;
        ",
            )
            .await
            .map_err(|e| anyhow!("Failed to define block_relations table: {}", e))?;

        *initialized = true;
        info!("SurrealDB schema initialized successfully");
        Ok(())
    }

    /// Convert a BlockId to a SurrealDB Thing identifier
    fn block_id_to_thing(&self, id: &BlockId) -> Thing {
        Thing::from(("memory_blocks", id.as_str()))
    }

    /// Create a relationship between two blocks
    pub async fn create_relationship(
        &self,
        from: &BlockId,
        to: &BlockId,
        relation_type: RelationType,
        strength: Option<f32>,
    ) -> Result<()> {
        let strength = strength.unwrap_or(1.0);

        // Use string-based approach to avoid Thing serialization issues
        let relation_type_str = relation_type.to_string();
        self.db
            .query(
                "
            CREATE block_relations SET
                in = type::thing('memory_blocks', $from_id),
                out = type::thing('memory_blocks', $to_id),
                relation_type = $relation_type,
                strength = $strength,
                created_at = time::now()
        ",
            )
            .bind(("from_id", from.as_str().to_string()))
            .bind(("to_id", to.as_str().to_string()))
            .bind(("relation_type", relation_type_str))
            .bind(("strength", strength))
            .await
            .map_err(|e| anyhow!("Failed to create relationship: {}", e))?
            .check()?;

        debug!(
            "Created {} relationship from {} to {} with strength {}",
            relation_type,
            from.as_str(),
            to.as_str(),
            strength
        );
        Ok(())
    }

    /// Find blocks related to the given block
    pub async fn find_related(
        &self,
        block_id: &BlockId,
        relation_type: RelationType,
    ) -> Result<Vec<MemoryBlock>> {
        // Use a simpler approach: join the relationship table with the memory blocks table
        let relation_type_str = relation_type.to_string();
        let mut response = self.db
            .query("
                SELECT * FROM memory_blocks
                WHERE id IN (
                    SELECT VALUE out FROM block_relations
                    WHERE in = type::thing('memory_blocks', $block_id) AND relation_type = $relation_type
                )
            ")
            .bind(("block_id", block_id.as_str().to_string()))
            .bind(("relation_type", relation_type_str))
            .await
            .map_err(|e| anyhow!("Failed to find related blocks: {}", e))?;

        // Use RawMemoryBlock for deserialization
        let raw_blocks: Vec<RawMemoryBlock> = response.take(0)?;

        // Convert to MemoryBlock
        let mut memory_blocks = Vec::new();
        for raw_block in raw_blocks {
            let enhanced_block = raw_block.to_enhanced()?;
            memory_blocks.push(enhanced_block.into());
        }

        Ok(memory_blocks)
    }

    /// Get aggregate statistics about memory usage
    pub async fn aggregate_stats(&self, user_id: &str) -> Result<MemoryStats> {
        // Get total block count
        let total_blocks: Option<i64> = self
            .db
            .query(
                "
            SELECT VALUE count() FROM memory_blocks WHERE user_id = $user_id
        ",
            )
            .bind(("user_id", user_id.to_string()))
            .await
            .map_err(|e| anyhow!("Failed to get total block count: {}", e))?
            .take(0)?;

        // Get blocks by type
        let type_counts: Vec<(String, i64)> = self
            .db
            .query(
                "
            SELECT block_type, count() AS count
            FROM memory_blocks
            WHERE user_id = $user_id
            GROUP BY block_type
        ",
            )
            .bind(("user_id", user_id.to_string()))
            .await
            .map_err(|e| anyhow!("Failed to get block type counts: {}", e))?
            .take(0)?;

        let blocks_by_type: HashMap<String, u64> = type_counts
            .into_iter()
            .map(|(block_type, count)| (block_type, count as u64))
            .collect();

        // Get average access count
        let avg_access: Option<f64> = self
            .db
            .query(
                "
            SELECT VALUE math::mean(access_count) FROM memory_blocks WHERE user_id = $user_id
        ",
            )
            .bind(("user_id", user_id.to_string()))
            .await
            .map_err(|e| anyhow!("Failed to get average access count: {}", e))?
            .take(0)?;

        // Get most accessed blocks
        let most_accessed: Vec<(String, i64)> = self
            .db
            .query(
                "
            SELECT id, access_count
            FROM memory_blocks
            WHERE user_id = $user_id
            ORDER BY access_count DESC
            LIMIT 10
        ",
            )
            .bind(("user_id", user_id.to_string()))
            .await
            .map_err(|e| anyhow!("Failed to get most accessed blocks: {}", e))?
            .take(0)?;

        let most_accessed_blocks: Vec<(BlockId, u64)> = most_accessed
            .into_iter()
            .map(|(id, count)| (BlockId::from(id), count as u64))
            .collect();

        Ok(MemoryStats {
            total_blocks: total_blocks.unwrap_or(0) as u64,
            blocks_by_type,
            total_size_bytes: 0, // TODO: Calculate actual size
            average_access_count: avg_access.unwrap_or(0.0),
            most_accessed_blocks,
        })
    }

    /// Perform vector similarity search using SurrealDB's vector functions
    async fn vector_similarity_search(&self, query: &MemoryQuery, vector_query: &VectorQuery) -> Result<Vec<MemoryBlock>> {
        let query_vector = &vector_query.query_vector;
        let search_config = &vector_query.search_config;

        // Build the complete SQL query with all filters
        let mut sql = "SELECT *, vector::similarity::cosine(embedding, $query_vector) AS similarity_score 
                       FROM memory_blocks 
                       WHERE embedding IS NOT NONE".to_string();
        
        let mut bindings = vec![("query_vector", serde_json::Value::Array(
            query_vector.iter().map(|f| serde_json::Value::Number(serde_json::Number::from_f64(*f as f64).unwrap())).collect()
        ))];

        // Add user_id filter
        if let Some(user_id) = &query.user_id {
            sql.push_str(" AND user_id = $user_id");
            bindings.push(("user_id", serde_json::Value::String(user_id.clone())));
        }

        // Add session_id filter
        if let Some(session_id) = &query.session_id {
            sql.push_str(" AND session_id = $session_id");
            bindings.push(("session_id", serde_json::Value::String(session_id.clone())));
        }

        // Add block_type filter
        if !query.block_types.is_empty() {
            sql.push_str(" AND block_type IN $block_types");
            let types: Vec<String> = query.block_types.iter().map(|t| format!("{:?}", t)).collect();
            bindings.push(("block_types", serde_json::Value::Array(
                types.into_iter().map(serde_json::Value::String).collect()
            )));
        }

        // Add similarity threshold filter
        sql.push_str(&format!(" AND vector::similarity::cosine(embedding, $query_vector) >= {}", 
                             search_config.similarity_threshold));

        // Order by similarity score descending
        sql.push_str(" ORDER BY similarity_score DESC");

        // Add limit
        let limit = query.limit.unwrap_or(search_config.max_results);
        sql.push_str(&format!(" LIMIT {}", limit));

        debug!("Vector similarity search SQL: {}", sql);

        // Create the query and bind all parameters
        let mut db_query = self.db.query(&sql);
        for (key, value) in bindings {
            db_query = db_query.bind((key, value));
        }

        let mut response = db_query
            .await
            .map_err(|e| anyhow!("Failed to execute vector similarity search: {}", e))?;

        // Use RawMemoryBlock for deserialization and add similarity scores
        let results: Vec<serde_json::Value> = response.take(0)?;

        let mut memory_blocks = Vec::new();
        for result in results {
            // Extract the similarity score
            let similarity_score = result.get("similarity_score")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0) as f32;

            // Parse the RawMemoryBlock from the result (excluding similarity_score)
            let raw_block: RawMemoryBlock = serde_json::from_value(result)?;
            let mut enhanced_block = raw_block.to_enhanced()?;
            
            // Set the relevance score based on similarity
            enhanced_block.relevance_score = Some(similarity_score);
            
            memory_blocks.push(enhanced_block.into());
        }

        debug!("Vector similarity search returned {} blocks", memory_blocks.len());
        Ok(memory_blocks)
    }

    /// Perform traditional text-based search
    async fn text_based_search(&self, query: &MemoryQuery) -> Result<Vec<MemoryBlock>> {
        // Use SurrealDB's select with conditions
        let mut sql = "SELECT * FROM memory_blocks WHERE true".to_string();
        let mut bindings = Vec::new();

        // Add user_id filter
        if let Some(user_id) = &query.user_id {
            sql.push_str(" AND user_id = $user_id");
            bindings.push(("user_id", user_id.clone()));
        }

        // Add session_id filter
        if let Some(session_id) = &query.session_id {
            sql.push_str(" AND session_id = $session_id");
            bindings.push(("session_id", session_id.clone()));
        }

        // Add block_type filter
        if !query.block_types.is_empty() {
            sql.push_str(" AND block_type IN $block_types");
            // Convert block types to strings for comparison
            let types: Vec<String> = query
                .block_types
                .iter()
                .map(|t| format!("{:?}", t))
                .collect();
            bindings.push(("block_types", serde_json::to_string(&types).unwrap()));
        }

        // Add content search
        if let Some(content_contains) = &query.content_contains {
            sql.push_str(
                " AND string::contains(string::lowercase(content), string::lowercase($content))",
            );
            bindings.push(("content", content_contains.clone()));
        }

        // Add ordering
        sql.push_str(" ORDER BY created_at DESC");

        // Add limit
        if let Some(limit) = query.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }

        // Execute query with proper binding
        let mut db_query = self.db.query(&sql);
        for (key, value) in bindings {
            db_query = db_query.bind((key, value));
        }

        let mut response = db_query
            .await
            .map_err(|e| anyhow!("Failed to execute query: {}", e))?;

        // Use RawMemoryBlock for deserialization
        let results: Vec<RawMemoryBlock> = response.take(0)?;

        // Convert RawMemoryBlock to MemoryBlock
        let mut memory_blocks = Vec::new();
        for raw_block in results {
            let enhanced_block = raw_block.to_enhanced()?;
            memory_blocks.push(enhanced_block.into());
        }

        Ok(memory_blocks)
    }
}

#[async_trait]
impl MemoryStore for SurrealMemoryStore {
    async fn store(&self, block: MemoryBlock) -> Result<BlockId> {
        self.initialize_schema().await?;

        let mut enhanced_block = EnhancedMemoryBlock::from(block);
        let block_id = enhanced_block.id.clone();

        // Generate embedding if embedding service is available and block doesn't have one
        if enhanced_block.embedding.is_none() {
            if let Some(embedding_service) = &self.embedding_service {
                // Extract text content for embedding
                let text_content = match &enhanced_block.content {
                    MemoryContent::Text(text) => text.clone(),
                    MemoryContent::Json(json) => json.to_string(),
                    MemoryContent::Binary { .. } => {
                        // Skip embedding for binary content
                        warn!("Skipping embedding generation for binary content in block {}", block_id.as_str());
                        String::new()
                    }
                };

                if !text_content.is_empty() {
                    match embedding_service.embed_text(&text_content).await {
                        Ok(embedding) => {
                            enhanced_block.embedding = Some(embedding);
                            debug!("Generated embedding for block {}", block_id.as_str());
                        }
                        Err(e) => {
                            warn!("Failed to generate embedding for block {}: {}", block_id.as_str(), e);
                            // Continue without embedding rather than failing the entire operation
                        }
                    }
                }
            }
        }

        // Use string-based approach to avoid SurrealDB enum serialization issues
        let content_json = serde_json::to_string(&enhanced_block.content)
            .map_err(|e| anyhow!("Failed to serialize content: {}", e))?;
        let metadata_json = serde_json::to_string(&enhanced_block.metadata)
            .map_err(|e| anyhow!("Failed to serialize metadata: {}", e))?;
        let tags_json = serde_json::to_string(&enhanced_block.tags)
            .map_err(|e| anyhow!("Failed to serialize tags: {}", e))?;
        let embedding_vec = enhanced_block.embedding.clone();

        // Create record with direct embedding vector
        let block_id_string = block_id.as_str().to_string();
        self.db
            .query(
                "
                CREATE type::thing('memory_blocks', $block_id) SET
                    user_id = $user_id,
                    session_id = $session_id,
                    block_type = $block_type,
                    content = $content,
                    metadata = $metadata,
                    tags = $tags,
                    embedding = $embedding,
                    relevance_score = $relevance_score,
                    access_count = $access_count,
                    last_accessed = $last_accessed,
                    created_at = $created_at,
                    updated_at = $updated_at
            ",
            )
            .bind(("block_id", block_id_string))
            .bind(("user_id", enhanced_block.user_id))
            .bind(("session_id", enhanced_block.session_id))
            .bind(("block_type", format!("{:?}", enhanced_block.block_type)))
            .bind(("content", content_json))
            .bind(("metadata", metadata_json))
            .bind(("tags", tags_json))
            .bind(("embedding", embedding_vec))
            .bind(("relevance_score", enhanced_block.relevance_score))
            .bind(("access_count", enhanced_block.access_count))
            .bind(("last_accessed", enhanced_block.last_accessed))
            .bind(("created_at", enhanced_block.created_at))
            .bind(("updated_at", enhanced_block.updated_at))
            .await
            .map_err(|e| anyhow!("Failed to store memory block: {}", e))?
            .check()?;

        debug!("Stored memory block with ID: {}", block_id.as_str());

        Ok(block_id)
    }

    async fn retrieve(&self, id: &BlockId) -> Result<Option<MemoryBlock>> {
        self.initialize_schema().await?;

        // Use SurrealDB query to get the record as raw strings
        let block_id_string = id.as_str().to_string();
        let mut response = self
            .db
            .query("SELECT * FROM type::thing('memory_blocks', $block_id)")
            .bind(("block_id", block_id_string.clone()))
            .await
            .map_err(|e| anyhow!("Failed to retrieve memory block: {}", e))?;

        // Use RawMemoryBlock for deserialization which handles string fields
        let result: Option<RawMemoryBlock> = response.take(0)?;

        if let Some(raw_block) = result {
            // Convert raw block to enhanced block using the to_enhanced method
            let mut enhanced_block = raw_block.to_enhanced()?;

            // Manually set the ID since raw block has Thing ID
            enhanced_block.id = id.clone();

            // Update access count when retrieving
            self.db
                .query("UPDATE type::thing('memory_blocks', $block_id) SET access_count += 1, last_accessed = time::now()")
                .bind(("block_id", block_id_string))
                .await
                .map_err(|e| anyhow!("Failed to update access count: {}", e))?
                .check()?;

            Ok(Some(enhanced_block.into()))
        } else {
            Ok(None)
        }
    }

    async fn query(&self, query: MemoryQuery) -> Result<Vec<MemoryBlock>> {
        self.initialize_schema().await?;

        // Handle vector similarity search
        if let Some(vector_query) = &query.vector_search {
            return self.vector_similarity_search(&query, vector_query).await;
        }

        // Regular text-based search (existing implementation)
        self.text_based_search(&query).await
    }

    async fn delete(&self, id: &BlockId) -> Result<bool> {
        self.initialize_schema().await?;

        let block_id_string = id.as_str().to_string();
        let mut response = self
            .db
            .query("DELETE type::thing('memory_blocks', $block_id) RETURN BEFORE")
            .bind(("block_id", block_id_string))
            .await
            .map_err(|e| anyhow!("Failed to delete memory block: {}", e))?;

        let result: Option<RawMemoryBlock> = response.take(0)?;
        Ok(result.is_some())
    }

    async fn update(&self, id: &BlockId, block: MemoryBlock) -> Result<MemoryBlock> {
        self.initialize_schema().await?;

        let enhanced_block = EnhancedMemoryBlock::from(block);

        // Use string-based approach to avoid SurrealDB enum serialization issues
        let content_json = serde_json::to_string(&enhanced_block.content)
            .map_err(|e| anyhow!("Failed to serialize content: {}", e))?;
        let metadata_json = serde_json::to_string(&enhanced_block.metadata)
            .map_err(|e| anyhow!("Failed to serialize metadata: {}", e))?;
        let tags_json = serde_json::to_string(&enhanced_block.tags)
            .map_err(|e| anyhow!("Failed to serialize tags: {}", e))?;
        let embedding_vec = enhanced_block.embedding.clone();

        // Update record with string fields
        let block_id_string = id.as_str().to_string();
        let mut response = self
            .db
            .query(
                "
                UPDATE type::thing('memory_blocks', $block_id) SET
                    user_id = $user_id,
                    session_id = $session_id,
                    block_type = $block_type,
                    content = $content,
                    metadata = $metadata,
                    tags = $tags,
                    embedding = $embedding,
                    relevance_score = $relevance_score,
                    access_count = $access_count,
                    last_accessed = $last_accessed,
                    created_at = $created_at,
                    updated_at = $updated_at
                RETURN AFTER
            ",
            )
            .bind(("block_id", block_id_string))
            .bind(("user_id", enhanced_block.user_id))
            .bind(("session_id", enhanced_block.session_id))
            .bind(("block_type", format!("{:?}", enhanced_block.block_type)))
            .bind(("content", content_json))
            .bind(("metadata", metadata_json))
            .bind(("tags", tags_json))
            .bind(("embedding", embedding_vec))
            .bind(("relevance_score", enhanced_block.relevance_score))
            .bind(("access_count", enhanced_block.access_count))
            .bind(("last_accessed", enhanced_block.last_accessed))
            .bind(("created_at", enhanced_block.created_at))
            .bind(("updated_at", enhanced_block.updated_at))
            .await
            .map_err(|e| anyhow!("Failed to update memory block: {}", e))?;

        let result: Option<RawMemoryBlock> = response.take(0)?;

        if let Some(raw_block) = result {
            let mut updated_block = raw_block.to_enhanced()?;
            // Manually set the ID since raw block has Thing ID
            updated_block.id = id.clone();
            Ok(updated_block.into())
        } else {
            Err(anyhow!("Memory block with ID {} not found", id.as_str()))
        }
    }

    async fn clear_user_data(&self, user_id: &str) -> Result<u64> {
        self.initialize_schema().await?;

        // Use SurrealDB delete with condition
        let mut response = self
            .db
            .query("DELETE memory_blocks WHERE user_id = $user_id RETURN BEFORE")
            .bind(("user_id", user_id.to_string()))
            .await
            .map_err(|e| anyhow!("Failed to clear user data: {}", e))?;

        let deleted: Vec<RawMemoryBlock> = response.take(0)?;
        Ok(deleted.len() as u64)
    }

    async fn get_stats(&self, user_id: &str) -> Result<crate::memory::MemoryStats> {
        self.initialize_schema().await?;

        let surreal_stats = self.aggregate_stats(user_id).await?;

        Ok(crate::memory::MemoryStats {
            total_blocks: surreal_stats.total_blocks,
            blocks_by_type: surreal_stats.blocks_by_type,
            total_size_bytes: surreal_stats.total_size_bytes,
            last_updated: Utc::now(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn create_test_store() -> (SurrealMemoryStore, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let config = SurrealConfig::File {
            path: db_path,
            namespace: "test".to_string(),
            database: "memory".to_string(),
        };

        let store = SurrealMemoryStore::new(config).await.unwrap();
        store.initialize_schema().await.unwrap();

        (store, temp_dir)
    }

    #[tokio::test]
    async fn test_store_and_retrieve() {
        let (store, _temp_dir) = create_test_store().await;

        let block = MemoryBlockBuilder::new()
            .with_user_id("test_user")
            .with_type(BlockType::Fact)
            .with_content(MemoryContent::Text("Test fact".to_string()))
            .build()
            .unwrap();

        println!("Original block ID: {}", block.id());
        let block_id = store.store(block.clone()).await.unwrap();
        println!("Stored block ID: {}", block_id.as_str());

        let retrieved = store.retrieve(&block_id).await.unwrap();
        println!("Retrieved: {:?}", retrieved.is_some());

        assert!(retrieved.is_some());
        let retrieved_block = retrieved.unwrap();
        assert_eq!(retrieved_block.user_id(), "test_user");
        assert_eq!(retrieved_block.block_type(), BlockType::Fact);
    }

    #[tokio::test]
    async fn test_relationships() {
        let (store, _temp_dir) = create_test_store().await;

        // Create two blocks
        let block1 = MemoryBlockBuilder::new()
            .with_user_id("test_user")
            .with_type(BlockType::Fact)
            .with_content(MemoryContent::Text("Fact 1".to_string()))
            .build()
            .unwrap();

        let block2 = MemoryBlockBuilder::new()
            .with_user_id("test_user")
            .with_type(BlockType::Fact)
            .with_content(MemoryContent::Text("Fact 2".to_string()))
            .build()
            .unwrap();

        let id1 = store.store(block1).await.unwrap();
        let id2 = store.store(block2).await.unwrap();

        // Verify we can retrieve both blocks before creating relationships
        let retrieved1 = store.retrieve(&id1).await.unwrap();
        let retrieved2 = store.retrieve(&id2).await.unwrap();
        assert!(retrieved1.is_some());
        assert!(retrieved2.is_some());
        println!("Both blocks retrieved successfully");

        // Create a relationship
        println!("Creating relationship...");
        match store
            .create_relationship(&id1, &id2, RelationType::References, Some(0.8))
            .await
        {
            Ok(_) => println!("Relationship created successfully"),
            Err(e) => {
                println!("Failed to create relationship: {}", e);
                panic!("Relationship creation failed");
            }
        }

        // Find related blocks
        println!("Finding related blocks...");
        match store.find_related(&id1, RelationType::References).await {
            Ok(related) => {
                println!("Found {} related blocks", related.len());
                assert_eq!(related.len(), 1);
                assert_eq!(related[0].id(), &id2);
            }
            Err(e) => {
                println!("Failed to find related blocks: {}", e);
                panic!("Find related failed: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_query_filtering() {
        let (store, _temp_dir) = create_test_store().await;

        // Store multiple blocks
        for i in 0..5 {
            let block = MemoryBlockBuilder::new()
                .with_user_id("test_user")
                .with_type(if i % 2 == 0 {
                    BlockType::Fact
                } else {
                    BlockType::Message
                })
                .with_content(MemoryContent::Text(format!("Content {}", i)))
                .build()
                .unwrap();

            store.store(block).await.unwrap();
        }

        // Query for facts only
        let query = MemoryQuery {
            user_id: Some("test_user".to_string()),
            block_types: vec![BlockType::Fact],
            ..Default::default()
        };

        let results = store.query(query).await.unwrap();
        assert_eq!(results.len(), 3); // Should get 3 facts (0, 2, 4)

        for result in results {
            assert_eq!(result.block_type(), BlockType::Fact);
        }
    }
}
