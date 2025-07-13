//! Context window manager for dynamic context management
//!
//! This module provides intelligent context window management, automatically
//! selecting and organizing memory blocks for optimal AI performance.

use crate::context::core_blocks::{CoreBlockManager, CoreBlockType, CoreBlockConfig, CoreBlockStats};
use crate::memory::{MemoryManager, MemoryBlock, MemoryQuery, QuerySort};
use crate::utils::tokens::TokenManager;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Configuration for context window management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextWindowConfig {
    /// Maximum total tokens for the context window
    pub max_total_tokens: u32,

    /// Tokens reserved for core blocks
    pub core_block_tokens: u32,

    /// Tokens reserved for recent conversation history
    pub conversation_tokens: u32,

    /// Tokens available for dynamic memory blocks
    pub dynamic_memory_tokens: u32,

    /// Maximum number of dynamic memory blocks to include
    pub max_dynamic_blocks: usize,

    /// Minimum relevance score for dynamic blocks
    pub min_relevance_score: f32,

    /// Whether to automatically manage the context window
    pub auto_manage: bool,

    /// Update frequency for context management (in seconds)
    pub update_interval: u64,
}

impl Default for ContextWindowConfig {
    fn default() -> Self {
        ContextWindowConfig {
            max_total_tokens: 8000,     // Reserve significant space for context
            core_block_tokens: 3000,    // Core blocks get priority
            conversation_tokens: 3000,  // Recent conversation history
            dynamic_memory_tokens: 2000, // Relevant memories
            max_dynamic_blocks: 10,
            min_relevance_score: 0.3,
            auto_manage: true,
            update_interval: 30, // Update every 30 seconds
        }
    }
}

/// Strategy for selecting dynamic memory blocks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelectionStrategy {
    /// Select by relevance score only
    ByRelevance,

    /// Select by recency (most recent first)
    ByRecency,

    /// Balanced selection (relevance + recency)
    Balanced,

    /// Select by access frequency
    ByFrequency,

    /// Diversified selection (try to include different types)
    Diversified,
}

impl Default for SelectionStrategy {
    fn default() -> Self {
        SelectionStrategy::Balanced
    }
}

/// A dynamic memory block selected for context inclusion
#[derive(Debug, Clone)]
pub struct ContextMemoryBlock {
    /// The memory block
    pub block: MemoryBlock,

    /// Relevance score for current context
    pub relevance_score: f32,

    /// Estimated token count
    pub estimated_tokens: u32,

    /// Last access time
    pub last_accessed: u64,

    /// Access frequency counter
    pub access_count: u32,
}

/// Context window state and contents
#[derive(Debug, Clone)]
pub struct ContextWindow {
    /// Core blocks currently in context
    pub core_blocks_content: String,

    /// Recent conversation history
    pub conversation_history: Vec<String>,

    /// Dynamic memory blocks in context
    pub dynamic_blocks: Vec<ContextMemoryBlock>,

    /// Total estimated token usage
    pub total_tokens: u32,

    /// Token breakdown by category
    pub token_breakdown: TokenBreakdown,

    /// Last update timestamp
    pub last_updated: u64,
}

/// Token usage breakdown for context window
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBreakdown {
    /// Tokens used by core blocks
    pub core_blocks: u32,

    /// Tokens used by conversation history
    pub conversation: u32,

    /// Tokens used by dynamic memory blocks
    pub dynamic_memory: u32,

    /// Total token usage
    pub total: u32,
}

/// Context window manager - the main interface for Letta-style memory management
pub struct ContextWindowManager {
    /// Core block manager
    core_manager: CoreBlockManager,

    /// Memory manager for dynamic blocks
    memory_manager: Arc<MemoryManager>,

    /// Token manager for estimation
    #[allow(dead_code)]
    token_manager: Arc<RwLock<TokenManager>>,

    /// Configuration
    config: ContextWindowConfig,

    /// Current context window state
    current_context: Arc<RwLock<Option<ContextWindow>>>,

    /// Memory block access tracking
    access_tracking: Arc<RwLock<HashMap<String, (u32, u64)>>>, // (access_count, last_accessed)

    /// Selection strategy
    strategy: SelectionStrategy,

    /// User ID
    user_id: String,

    /// Session ID
    #[allow(dead_code)]
    session_id: String,
}

impl ContextWindowManager {
    /// Create a new context window manager
    pub fn new(
        user_id: impl Into<String>,
        session_id: impl Into<String>,
        memory_manager: Arc<MemoryManager>,
        token_manager: Arc<RwLock<TokenManager>>,
        config: Option<ContextWindowConfig>,
        core_config: Option<CoreBlockConfig>,
    ) -> Self {
        let user_id = user_id.into();
        let session_id = session_id.into();
        let config = config.unwrap_or_default();

        let mut core_manager = CoreBlockManager::new(&user_id, core_config);
        core_manager.initialize().unwrap_or_else(|e| {
            warn!("Failed to initialize core blocks: {}", e);
        });

        ContextWindowManager {
            core_manager,
            memory_manager,
            token_manager,
            config,
            current_context: Arc::new(RwLock::new(None)),
            access_tracking: Arc::new(RwLock::new(HashMap::new())),
            strategy: SelectionStrategy::default(),
            user_id,
            session_id,
        }
    }

    /// Update the context window with current conversation and memory
    pub async fn update_context(&mut self, conversation_history: Vec<String>) -> Result<()> {
        info!("Updating context window for user: {}", self.user_id);

        // Get core blocks content
        let core_content = self.core_manager.format_for_context();
        let core_tokens = self.estimate_tokens(&core_content);

        // Calculate conversation tokens
        let conversation_tokens = conversation_history
            .iter()
            .map(|msg| self.estimate_tokens(msg))
            .sum::<u32>();

        // Determine available tokens for dynamic memory
        let used_tokens = core_tokens + conversation_tokens;
        let available_tokens = self.config.dynamic_memory_tokens
            .saturating_sub(used_tokens.saturating_sub(self.config.core_block_tokens + self.config.conversation_tokens));

        // Select dynamic memory blocks
        let dynamic_blocks = self.select_dynamic_blocks(available_tokens).await?;
        let dynamic_tokens = dynamic_blocks.iter().map(|b| b.estimated_tokens).sum::<u32>();

        // Create context window
        let context_window = ContextWindow {
            core_blocks_content: core_content,
            conversation_history,
            dynamic_blocks,
            total_tokens: core_tokens + conversation_tokens + dynamic_tokens,
            token_breakdown: TokenBreakdown {
                core_blocks: core_tokens,
                conversation: conversation_tokens,
                dynamic_memory: dynamic_tokens,
                total: core_tokens + conversation_tokens + dynamic_tokens,
            },
            last_updated: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        };

        // Update current context
        let mut current = self.current_context.write().await;
        *current = Some(context_window);

        debug!("Context window updated. Total tokens: {}", current.as_ref().unwrap().total_tokens);

        Ok(())
    }

    /// Select dynamic memory blocks based on strategy and available tokens
    async fn select_dynamic_blocks(&mut self, available_tokens: u32) -> Result<Vec<ContextMemoryBlock>> {
        let query = MemoryQuery {
            user_id: Some(self.user_id.clone()),
            session_id: None,
            block_types: Vec::new(),
            content_contains: None,
            created_after: None,
            created_before: None,
            limit: Some(self.config.max_dynamic_blocks * 2),
            sort: Some(QuerySort::Relevance),
            vector_search: None,
        };

        let candidate_blocks = self.memory_manager.search(&query).await?;
        let mut context_blocks = Vec::new();
        let mut used_tokens = 0u32;

        // Convert to context memory blocks and filter
        let mut candidates: Vec<ContextMemoryBlock> = candidate_blocks
            .into_iter()
            .filter_map(|block| {
                let content_len = block.content.as_text()?.len();
                let estimated_tokens = (content_len as f32 / 4.0).ceil() as u32;
                let relevance = block.metadata.relevance?.score();

                if relevance >= self.config.min_relevance_score {
                    Some(ContextMemoryBlock {
                        block,
                        relevance_score: relevance,
                        estimated_tokens,
                        last_accessed: 0, // Will be updated from tracking
                        access_count: 0,  // Will be updated from tracking
                    })
                } else {
                    None
                }
            })
            .collect();

        // Update access tracking info
        let tracking = self.access_tracking.read().await;
        for context_block in &mut candidates {
            if let Some((count, last_accessed)) = tracking.get(context_block.block.id().as_str()) {
                context_block.access_count = *count;
                context_block.last_accessed = *last_accessed;
            }
        }

        // Sort by strategy
        self.sort_candidates_by_strategy(&mut candidates);

        // Select blocks within token budget
        for candidate in candidates {
            if used_tokens + candidate.estimated_tokens <= available_tokens &&
               context_blocks.len() < self.config.max_dynamic_blocks {
                used_tokens += candidate.estimated_tokens;
                context_blocks.push(candidate);
            }
        }

        info!("Selected {} dynamic memory blocks using {} tokens",
              context_blocks.len(), used_tokens);

        Ok(context_blocks)
    }

    /// Sort candidate blocks based on selection strategy
    fn sort_candidates_by_strategy(&self, candidates: &mut Vec<ContextMemoryBlock>) {
        match self.strategy {
            SelectionStrategy::ByRelevance => {
                candidates.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap());
            },
            SelectionStrategy::ByRecency => {
                candidates.sort_by(|a, b| b.block.metadata.updated_at.cmp(&a.block.metadata.updated_at));
            },
            SelectionStrategy::ByFrequency => {
                candidates.sort_by(|a, b| b.access_count.cmp(&a.access_count));
            },
            SelectionStrategy::Balanced => {
                // Combine relevance and recency with weights
                candidates.sort_by(|a, b| {
                    let score_a = a.relevance_score * 0.7 +
                        (a.block.metadata.updated_at as f32 / 1_000_000_000.0) * 0.3;
                    let score_b = b.relevance_score * 0.7 +
                        (b.block.metadata.updated_at as f32 / 1_000_000_000.0) * 0.3;
                    score_b.partial_cmp(&score_a).unwrap()
                });
            },
            SelectionStrategy::Diversified => {
                // Sort by relevance first, then try to diversify by block type
                candidates.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap());

                // TODO: Implement type-based diversification
                // This would require tracking block types and ensuring variety
            },
        }
    }

    /// Get the current context formatted for AI input
    pub async fn get_formatted_context(&self) -> Result<String> {
        let context_guard = self.current_context.read().await;

        if let Some(context) = context_guard.as_ref() {
            let mut formatted = String::new();

            // Add core blocks
            formatted.push_str("# Core Context\n\n");
            formatted.push_str(&context.core_blocks_content);
            formatted.push_str("\n");

            // Add relevant memories
            if !context.dynamic_blocks.is_empty() {
                formatted.push_str("# Relevant Memories\n\n");
                for (i, memory_block) in context.dynamic_blocks.iter().enumerate() {
                    if let Some(content) = memory_block.block.content.as_text() {
                        formatted.push_str(&format!("## Memory {} (Relevance: {:.2})\n\n{}\n\n",
                            i + 1, memory_block.relevance_score, content));
                    }
                }
            }

            // Add recent conversation (this would typically be managed separately)
            if !context.conversation_history.is_empty() {
                formatted.push_str("# Recent Conversation\n\n");
                for (_i, message) in context.conversation_history.iter().rev().take(5).enumerate() {
                    formatted.push_str(&format!("{}\n\n", message));
                }
            }

            Ok(formatted)
        } else {
            Ok("# Context\n\nNo context available yet.".to_string())
        }
    }

    /// Update a core block
    pub fn update_core_block(&mut self, core_type: CoreBlockType, content: String) -> Result<()> {
        self.core_manager.update_block(core_type, content)
    }

    /// Get core block content
    pub fn get_core_block_content(&mut self, core_type: CoreBlockType) -> Option<String> {
        self.core_manager.get_block(core_type)
            .and_then(|block| block.get_text_content().map(|s| s.to_string()))
    }

    /// Add a memory block and mark it as accessed
    pub async fn access_memory_block(&self, block_id: &str) {
        let mut tracking = self.access_tracking.write().await;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let entry = tracking.entry(block_id.to_string()).or_insert((0, now));
        entry.0 += 1; // Increment access count
        entry.1 = now; // Update last accessed time
    }

    /// Set the selection strategy
    pub fn set_selection_strategy(&mut self, strategy: SelectionStrategy) {
        self.strategy = strategy;
        info!("Changed context selection strategy to: {:?}", strategy);
    }

    /// Get context window statistics
    pub async fn get_stats(&self) -> ContextWindowStats {
        let context_guard = self.current_context.read().await;
        let core_stats = self.core_manager.get_stats();

        if let Some(context) = context_guard.as_ref() {
            ContextWindowStats {
                core_block_stats: core_stats,
                total_tokens: context.total_tokens,
                token_breakdown: context.token_breakdown.clone(),
                dynamic_blocks_count: context.dynamic_blocks.len(),
                max_tokens: self.config.max_total_tokens,
                utilization: (context.total_tokens as f32 / self.config.max_total_tokens as f32) * 100.0,
                last_updated: context.last_updated,
            }
        } else {
            ContextWindowStats {
                core_block_stats: core_stats,
                total_tokens: 0,
                token_breakdown: TokenBreakdown {
                    core_blocks: 0,
                    conversation: 0,
                    dynamic_memory: 0,
                    total: 0,
                },
                dynamic_blocks_count: 0,
                max_tokens: self.config.max_total_tokens,
                utilization: 0.0,
                last_updated: 0,
            }
        }
    }

    /// Estimate tokens for text content
    fn estimate_tokens(&self, text: &str) -> u32 {
        // Simple token estimation: ~4 characters per token
        (text.len() as f32 / 4.0).ceil() as u32
    }

    /// Perform maintenance on the context window
    pub async fn maintenance(&mut self) -> Result<()> {
        // Auto-manage core blocks
        self.core_manager.auto_manage_blocks()?;

        // Clean up old access tracking entries (keep only last 1000)
        let mut tracking = self.access_tracking.write().await;
        if tracking.len() > 1000 {
            let mut entries: Vec<_> = tracking.iter().map(|(k, v)| (k.clone(), *v)).collect();
            entries.sort_by_key(|(_, (_, last_accessed))| *last_accessed);

            // Keep only the 800 most recently accessed
            tracking.clear();
            for (key, value) in entries.into_iter().rev().take(800) {
                tracking.insert(key, value);
            }
        }

        info!("Context window maintenance completed");
        Ok(())
    }
}

/// Statistics about context window usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextWindowStats {
    /// Core block statistics
    pub core_block_stats: CoreBlockStats,

    /// Total token usage
    pub total_tokens: u32,

    /// Token breakdown
    pub token_breakdown: TokenBreakdown,

    /// Number of dynamic blocks in context
    pub dynamic_blocks_count: usize,

    /// Maximum allowed tokens
    pub max_tokens: u32,

    /// Context window utilization percentage
    pub utilization: f32,

    /// Last update timestamp
    pub last_updated: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::{SurrealMemoryStore, SurrealConfig};
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_context_window_manager() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let config = SurrealConfig::File {
            path: db_path,
            namespace: "test".to_string(),
            database: "memory".to_string(),
        };
        let store = SurrealMemoryStore::new(config).await.unwrap();
        store.initialize_schema_with_dimensions(384).await.unwrap();
        let memory_manager = Arc::new(MemoryManager::new(store));
        let token_manager = Arc::new(RwLock::new(TokenManager::new(std::path::PathBuf::from("./data"))));

        let mut manager = ContextWindowManager::new(
            "test_user",
            "test_session",
            memory_manager,
            token_manager,
            None,
            None,
        );

        // Test core block update
        manager.update_core_block(
            CoreBlockType::UserPersona,
            "Test user who likes programming".to_string(),
        ).unwrap();

        // Test context update
        let conversation = vec!["Hello".to_string(), "How are you?".to_string()];
        manager.update_context(conversation).await.unwrap();

        // Test formatted context
        let formatted = manager.get_formatted_context().await.unwrap();
        assert!(formatted.contains("Core Context"));
        assert!(formatted.contains("programming"));
    }
}
