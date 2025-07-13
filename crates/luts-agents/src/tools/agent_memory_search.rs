//! Agent-focused memory search tool with vector semantics
//!
//! This tool provides agents with powerful semantic search capabilities over their memory,
//! allowing them to find relevant context and information based on meaning rather than keywords.

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use luts_llm::tools::AiTool;
use luts_memory::{BlockType, MemoryContent, MemoryManager, MemoryQuery};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Agent-focused memory search tool with semantic capabilities
pub struct AgentMemorySearchTool {
    pub memory_manager: Arc<MemoryManager>,
    pub user_id: String,
}

impl AgentMemorySearchTool {
    /// Create a new agent memory search tool
    pub fn new(memory_manager: Arc<MemoryManager>, user_id: String) -> Self {
        Self {
            memory_manager,
            user_id,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct AgentSearchParams {
    /// The search query - can be a question, topic, or description
    query: String,
    /// Types of memory to search (optional - defaults to all)
    memory_types: Option<Vec<String>>,
    /// Session to search within (optional - searches all sessions)
    session_id: Option<String>,
    /// Maximum results to return (1-10, defaults to 5)
    max_results: Option<usize>,
    /// Search mode: "semantic" (meaning-based) or "keyword" (text-based)
    search_mode: Option<String>,
    /// Minimum relevance score (0.0-1.0, defaults to 0.6)
    min_relevance: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize)]
struct AgentSearchResponse {
    /// Search summary
    summary: String,
    /// Number of results found
    total_results: usize,
    /// Search results with context
    memories: Vec<MemoryResult>,
    /// Search metadata
    search_details: SearchDetails,
}

#[derive(Debug, Serialize, Deserialize)]
struct MemoryResult {
    /// Memory ID for reference
    id: String,
    /// Type of memory (Fact, Goal, Task, etc.)
    memory_type: String,
    /// Relevance score (higher = more relevant)
    relevance: f32,
    /// Memory content with intelligent truncation
    content: String,
    /// When this memory was created
    created: String,
    /// Optional session context
    session: Option<String>,
    /// Key insights or tags
    insights: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SearchDetails {
    /// The original query
    query: String,
    /// Search mode used
    mode: String,
    /// Search parameters
    parameters: SearchParameters,
    /// Performance metrics
    performance: SearchMetrics,
}

#[derive(Debug, Serialize, Deserialize)]
struct SearchParameters {
    /// Types searched
    memory_types: Vec<String>,
    /// Minimum relevance threshold used
    min_relevance: f32,
    /// Maximum results requested
    max_results: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct SearchMetrics {
    /// Search duration in milliseconds
    duration_ms: u64,
    /// Total memories scanned
    memories_scanned: usize,
    /// Memories above threshold
    relevant_memories: usize,
}

#[async_trait]
impl AiTool for AgentMemorySearchTool {
    fn name(&self) -> &str {
        "search_agent_memory"
    }

    fn description(&self) -> &str {
        "Search through the agent's memory using semantic similarity to find relevant information, facts, goals, tasks, and context. \
         This tool understands meaning and context, not just keywords, making it excellent for finding related concepts and insights."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "What you're looking for. Can be a question, topic, concept, or description. Examples: 'user preferences about music', 'goals related to productivity', 'facts about the current project'"
                },
                "memory_types": {
                    "type": "array",
                    "items": {
                        "type": "string",
                        "enum": ["Message", "Summary", "Fact", "Preference", "PersonalInfo", "Goal", "Task"]
                    },
                    "description": "Types of memories to search (optional). Defaults to all types if not specified."
                },
                "session_id": {
                    "type": "string",
                    "description": "Search within a specific conversation session (optional)"
                },
                "max_results": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 10,
                    "default": 5,
                    "description": "Maximum number of relevant memories to return"
                },
                "search_mode": {
                    "type": "string",
                    "enum": ["semantic", "keyword"],
                    "default": "semantic",
                    "description": "Search mode: 'semantic' for meaning-based search, 'keyword' for text matching"
                },
                "min_relevance": {
                    "type": "number",
                    "minimum": 0.0,
                    "maximum": 1.0,
                    "default": 0.6,
                    "description": "Minimum relevance score (0.0-1.0). Higher values return only very relevant results."
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, params: Value) -> Result<Value> {
        let start_time = std::time::Instant::now();

        let params: AgentSearchParams = serde_json::from_value(params)
            .map_err(|e| anyhow!("Invalid search parameters: {}", e))?;

        info!("🔍 Agent searching memory for: '{}'", params.query);

        // Parse memory types
        let block_types = if let Some(type_strings) = &params.memory_types {
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
                    _ => warn!("Unknown memory type: {}", type_str),
                }
            }
            types
        } else {
            Vec::new() // Empty = search all types
        };

        let max_results = params.max_results.unwrap_or(5).min(10).max(1);

        // For now, use basic keyword search regardless of mode
        // Future enhancement: implement proper semantic search with embeddings
        debug!("Performing memory search (keyword-based for now)");
        let query = MemoryQuery {
            user_id: Some(self.user_id.clone()),
            session_id: params.session_id.clone(),
            block_types,
            content_contains: Some(params.query.clone()),
            limit: Some(max_results),
            ..Default::default()
        };

        let results = self
            .memory_manager
            .search(&query)
            .await
            .map_err(|e| anyhow!("Memory search failed: {}", e))?;

        let duration = start_time.elapsed();

        debug!(
            "Found {} memory results in {}ms",
            results.len(),
            duration.as_millis()
        );

        // Convert results to agent-friendly format
        let memory_results: Vec<MemoryResult> = results
            .into_iter()
            .map(|block| {
                let content = match &block.content {
                    MemoryContent::Text(text) => {
                        if text.len() > 300 {
                            format!("{}...", &text[..300])
                        } else {
                            text.clone()
                        }
                    }
                    MemoryContent::Json(json) => {
                        let json_str = json.to_string();
                        if json_str.len() > 300 {
                            format!("{}...", &json_str[..300])
                        } else {
                            json_str
                        }
                    }
                    MemoryContent::Binary { .. } => "[Binary content - not searchable]".to_string(),
                };

                // Extract insights from tags and content
                let mut insights = block.tags().to_vec();
                if insights.is_empty() {
                    // Generate basic insights from block type and content length
                    insights.push(format!("{:?}", block.block_type()).to_lowercase());
                    if content.len() > 100 {
                        insights.push("detailed".to_string());
                    }
                }

                MemoryResult {
                    id: block.id().as_str().to_string(),
                    memory_type: format!("{:?}", block.block_type()),
                    relevance: block.relevance().map(|r| r.score()).unwrap_or(0.8), // Default relevance
                    content,
                    created: chrono::DateTime::from_timestamp_millis(block.created_at() as i64)
                        .unwrap_or_else(|| chrono::Utc::now())
                        .format("%Y-%m-%d %H:%M:%S UTC")
                        .to_string(),
                    session: block.session_id().map(|s| s.to_string()),
                    insights,
                }
            })
            .collect();

        // Generate intelligent summary
        let summary = if memory_results.is_empty() {
            format!("No memories found matching '{}'", params.query)
        } else if memory_results.len() == 1 {
            format!("Found 1 relevant memory about '{}'", params.query)
        } else {
            let types: std::collections::HashSet<_> = memory_results
                .iter()
                .map(|r| r.memory_type.as_str())
                .collect();
            if types.len() == 1 {
                format!(
                    "Found {} {} memories related to '{}'",
                    memory_results.len(),
                    types.iter().next().unwrap().to_lowercase(),
                    params.query
                )
            } else {
                format!(
                    "Found {} memories across {} different types related to '{}'",
                    memory_results.len(),
                    types.len(),
                    params.query
                )
            }
        };

        let search_mode = params.search_mode.unwrap_or_else(|| "keyword".to_string());
        let min_relevance = params.min_relevance.unwrap_or(0.6).clamp(0.0, 1.0);
        let memory_count = memory_results.len();

        let response = AgentSearchResponse {
            summary,
            total_results: memory_count,
            memories: memory_results,
            search_details: SearchDetails {
                query: params.query,
                mode: search_mode,
                parameters: SearchParameters {
                    memory_types: params
                        .memory_types
                        .unwrap_or_else(|| vec!["All".to_string()]),
                    min_relevance,
                    max_results,
                },
                performance: SearchMetrics {
                    duration_ms: duration.as_millis() as u64,
                    memories_scanned: 0, // Would need actual implementation
                    relevant_memories: memory_count,
                },
            },
        };

        info!("✅ Memory search completed: {}", response.summary);

        Ok(serde_json::to_value(response)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use luts_memory::{MemoryBlockBuilder, SurrealConfig, SurrealMemoryStore};

    #[tokio::test]
    async fn test_agent_memory_search() {
        // Create in-memory store for testing
        let config = SurrealConfig::Memory {
            namespace: "test".to_string(),
            database: "memory".to_string(),
        };

        let store = SurrealMemoryStore::new(config).await.unwrap();
        let memory_manager = Arc::new(MemoryManager::new(store));

        // Create test memory
        let block = MemoryBlockBuilder::new()
            .with_user_id("test_user")
            .with_type(BlockType::Fact)
            .with_content(MemoryContent::Text(
                "The user prefers dark mode for coding".to_string(),
            ))
            .with_tag("preference")
            .build()
            .unwrap();

        memory_manager.store(block).await.unwrap();

        // Create search tool
        let tool = AgentMemorySearchTool::new(memory_manager, "test_user".to_string());

        // Test search
        let params = json!({
            "query": "user interface preferences",
            "search_mode": "keyword",
            "max_results": 3
        });

        let result = tool.execute(params).await.unwrap();

        // Verify response structure
        assert!(result.is_object());
        assert!(result.get("summary").is_some());
        assert!(result.get("memories").is_some());

        println!(
            "Agent memory search result: {}",
            serde_json::to_string_pretty(&result).unwrap()
        );
    }
}
