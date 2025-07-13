//! Semantic search tool for AI agents
//!
//! This tool provides semantic search capabilities using vector embeddings
//! to find relevant memory blocks based on meaning rather than just keywords.

use crate::memory::{
    MemoryManager, VectorSearchConfig, EmbeddingService, EmbeddingServiceFactory, 
    EmbeddingConfig, EmbeddingProvider, BlockType,
};
use crate::tools::AiTool;
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::{debug, warn};

/// Semantic search tool that uses vector embeddings for similarity search
pub struct SemanticSearchTool {
    pub memory_manager: Arc<MemoryManager>,
    pub embedding_service: Arc<dyn EmbeddingService>,
}

impl SemanticSearchTool {
    /// Create a new semantic search tool with default embedding service
    pub fn new(memory_manager: Arc<MemoryManager>) -> Result<Self> {
        // Create a default embedding service (mock for now, can be configured)
        let embedding_config = EmbeddingConfig {
            provider: EmbeddingProvider::Mock,
            dimensions: 384, // Common dimension for many embedding models
            ..Default::default()
        };
        
        let embedding_service = EmbeddingServiceFactory::create(embedding_config)?;
        
        Ok(Self {
            memory_manager,
            embedding_service,
        })
    }
    
    /// Create a semantic search tool with a specific embedding service
    pub fn with_embedding_service(
        memory_manager: Arc<MemoryManager>,
        embedding_service: Arc<dyn EmbeddingService>,
    ) -> Self {
        Self {
            memory_manager,
            embedding_service,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct SemanticSearchParams {
    /// The search query text
    query: String,
    /// User ID to search within (defaults to current user)
    user_id: Option<String>,
    /// Session ID to search within (optional)
    session_id: Option<String>,
    /// Block types to search (optional - searches all if not specified)
    block_types: Option<Vec<String>>,
    /// Maximum number of results (defaults to 5)
    max_results: Option<usize>,
    /// Similarity threshold (0.0 to 1.0, defaults to 0.7)
    similarity_threshold: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SemanticSearchResult {
    /// Number of results found
    results_found: usize,
    /// The search results
    results: Vec<SearchResultItem>,
    /// Search metadata
    search_info: SearchMetadata,
}

#[derive(Debug, Serialize, Deserialize)]
struct SearchResultItem {
    /// Block ID
    block_id: String,
    /// Block type
    block_type: String,
    /// Similarity score (0.0 to 1.0)
    similarity_score: f32,
    /// Block content (truncated for display)
    content_preview: String,
    /// Block metadata
    created_at: String,
    /// User who created the block
    user_id: String,
    /// Session the block belongs to
    session_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SearchMetadata {
    /// The query that was searched
    query: String,
    /// Embedding dimensions used
    embedding_dimensions: usize,
    /// Search configuration used
    similarity_threshold: f32,
    /// Maximum results requested
    max_results: usize,
}

#[async_trait]
impl AiTool for SemanticSearchTool {
    fn name(&self) -> &str {
        "semantic_search"
    }

    fn description(&self) -> &str {
        "Search memory blocks using semantic similarity based on meaning rather than keywords. \
         This tool uses vector embeddings to find content that is conceptually similar to your query, \
         even if it doesn't contain the exact same words."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The search query text. Describe what you're looking for in natural language."
                },
                "user_id": {
                    "type": "string",
                    "description": "User ID to search within (optional - defaults to current user)",
                    "default": "current_user"
                },
                "session_id": {
                    "type": "string",
                    "description": "Session ID to search within (optional - searches all sessions if not specified)"
                },
                "block_types": {
                    "type": "array",
                    "items": {
                        "type": "string",
                        "enum": ["Message", "Summary", "Fact", "Preference", "PersonalInfo", "Goal", "Task"]
                    },
                    "description": "Types of memory blocks to search (optional - searches all types if not specified)"
                },
                "max_results": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 20,
                    "default": 5,
                    "description": "Maximum number of results to return (1-20, defaults to 5)"
                },
                "similarity_threshold": {
                    "type": "number",
                    "minimum": 0.0,
                    "maximum": 1.0,
                    "default": 0.7,
                    "description": "Minimum similarity score for results (0.0-1.0, defaults to 0.7)"
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, params: Value) -> Result<Value> {
        let params: SemanticSearchParams = serde_json::from_value(params)
            .map_err(|e| anyhow!("Invalid parameters for semantic search: {}", e))?;

        debug!("Performing semantic search for query: '{}'", params.query);

        // Default user_id if not provided
        let user_id = params.user_id.unwrap_or_else(|| "current_user".to_string());

        // Parse block types if provided
        let _block_types = if let Some(type_strings) = params.block_types {
            let mut types = Vec::new();
            for type_str in type_strings {
                match type_str.as_str() {
                    "Message" => types.push(BlockType::Message),
                    "Summary" => types.push(BlockType::Summary),
                    "Fact" => types.push(BlockType::Fact),
                    "Preference" => types.push(BlockType::Preference),
                    "PersonalInfo" => types.push(BlockType::PersonalInfo),
                    "Goal" => types.push(BlockType::Goal),
                    "Task" => types.push(BlockType::Task),
                    _ => {
                        warn!("Unknown block type: {}", type_str);
                        continue;
                    }
                }
            }
            types
        } else {
            Vec::new() // Empty means search all types
        };

        // Configure search parameters
        let search_config = VectorSearchConfig {
            similarity_threshold: params.similarity_threshold.unwrap_or(0.7),
            max_results: params.max_results.unwrap_or(5),
            ..Default::default()
        };

        // Perform semantic search
        let results = self.memory_manager
            .semantic_search(
                self.embedding_service.as_ref(),
                &params.query,
                &user_id,
                Some(search_config.clone()),
            )
            .await?;

        debug!("Semantic search found {} results", results.len());

        // Convert results to response format
        let search_results: Vec<SearchResultItem> = results
            .into_iter()
            .map(|block| {
                let content_preview = match &block.content {
                    crate::memory::MemoryContent::Text(text) => {
                        if text.len() > 200 {
                            format!("{}...", &text[..200])
                        } else {
                            text.clone()
                        }
                    }
                    crate::memory::MemoryContent::Json(json) => {
                        let json_str = json.to_string();
                        if json_str.len() > 200 {
                            format!("{}...", &json_str[..200])
                        } else {
                            json_str
                        }
                    }
                    crate::memory::MemoryContent::Binary { .. } => "[Binary content]".to_string(),
                };

                SearchResultItem {
                    block_id: block.id().as_str().to_string(),
                    block_type: format!("{:?}", block.block_type()),
                    similarity_score: block.metadata.relevance
                        .map(|r| r.score())
                        .unwrap_or(0.0),
                    content_preview,
                    created_at: chrono::DateTime::from_timestamp_millis(block.metadata.created_at as i64)
                        .unwrap_or_else(|| chrono::Utc::now())
                        .to_rfc3339(),
                    user_id: block.metadata.user_id.clone(),
                    session_id: block.metadata.session_id.clone(),
                }
            })
            .collect();

        let response = SemanticSearchResult {
            results_found: search_results.len(),
            results: search_results,
            search_info: SearchMetadata {
                query: params.query,
                embedding_dimensions: self.embedding_service.dimensions(),
                similarity_threshold: search_config.similarity_threshold,
                max_results: search_config.max_results,
            },
        };

        Ok(serde_json::to_value(response)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::{
        MemoryBlockBuilder, MemoryContent, BlockType, 
        SurrealMemoryStore, SurrealConfig, 
        EmbeddingServiceFactory, EmbeddingConfig, EmbeddingProvider,
    };
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_semantic_search_tool() {
        // Create temporary storage
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let config = SurrealConfig::File {
            path: db_path,
            namespace: "test".to_string(),
            database: "memory".to_string(),
        };

        // Create embedding service
        let embedding_config = EmbeddingConfig {
            provider: EmbeddingProvider::Mock,
            dimensions: 384,
            ..Default::default()
        };
        let embedding_service = EmbeddingServiceFactory::create(embedding_config).unwrap();

        // Create store with embedding service
        let store = SurrealMemoryStore::with_embedding_service(config, Some(embedding_service.clone()))
            .await
            .unwrap();
        
        // Initialize the schema with matching dimensions
        store.initialize_schema_with_dimensions(384).await.unwrap();
        
        let memory_manager = Arc::new(MemoryManager::new(store));

        // Create and store some test blocks
        let block1 = MemoryBlockBuilder::new()
            .with_user_id("test_user")
            .with_type(BlockType::Fact)
            .with_content(MemoryContent::Text("The capital of France is Paris".to_string()))
            .build()
            .unwrap();

        let block2 = MemoryBlockBuilder::new()
            .with_user_id("test_user")
            .with_type(BlockType::Fact)
            .with_content(MemoryContent::Text("Python is a programming language".to_string()))
            .build()
            .unwrap();

        memory_manager.store(block1).await.unwrap();
        memory_manager.store(block2).await.unwrap();

        // Create semantic search tool
        let tool = SemanticSearchTool::with_embedding_service(memory_manager, embedding_service);

        // Test semantic search
        let params = json!({
            "query": "European capitals",
            "user_id": "test_user",
            "max_results": 5
        });

        let result = tool.execute(params).await.unwrap();
        
        // Verify result structure
        assert!(result.is_object());
        assert!(result.get("results_found").is_some());
        assert!(result.get("results").is_some());
        assert!(result.get("search_info").is_some());

        println!("Semantic search result: {}", serde_json::to_string_pretty(&result).unwrap());
    }
}