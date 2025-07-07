//! Conversation summarization system
//!
//! This module provides intelligent conversation summarization capabilities,
//! automatically condensing long conversations while preserving key context.

use crate::llm::{AiService, InternalChatMessage};
use crate::memory::{MemoryBlock, MemoryBlockBuilder, MemoryContent, BlockType};
use crate::utils::tokens::{TokenManager, TokenUsage};
use anyhow::Result;
use chrono::{DateTime, Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Summarization strategy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummarizationConfig {
    /// Maximum conversation length before triggering summarization
    pub max_conversation_length: usize,
    /// Target summary length (approximate tokens)
    pub target_summary_length: usize,
    /// Minimum conversation length to consider for summarization
    pub min_conversation_length: usize,
    /// Summarization strategy
    pub strategy: SummarizationStrategy,
    /// Preserve recent messages count (won't be summarized)
    pub preserve_recent_count: usize,
    /// Auto-summarize when token budget is approaching limit
    pub auto_summarize_on_budget_limit: bool,
    /// Keep important messages (marked as important)
    pub preserve_important_messages: bool,
}

impl Default for SummarizationConfig {
    fn default() -> Self {
        Self {
            max_conversation_length: 50,     // Start summarizing after 50 messages
            target_summary_length: 500,      // Aim for ~500 token summaries
            min_conversation_length: 10,     // Don't summarize very short conversations
            strategy: SummarizationStrategy::Progressive,
            preserve_recent_count: 5,        // Always keep last 5 messages
            auto_summarize_on_budget_limit: true,
            preserve_important_messages: true,
        }
    }
}

/// Different summarization strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SummarizationStrategy {
    /// Summarize everything into a single block
    Single,
    /// Progressive summarization (older summaries get re-summarized)
    Progressive,
    /// Topic-based summarization (group by topics)
    TopicBased,
    /// Hierarchical summarization (multiple levels)
    Hierarchical,
}

/// Summary metadata and tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryInfo {
    /// Unique ID for this summary
    pub id: String,
    /// When this summary was created
    pub created_at: DateTime<Utc>,
    /// Number of original messages summarized
    pub original_message_count: usize,
    /// Compression ratio (original tokens / summary tokens)
    pub compression_ratio: f64,
    /// Summarization strategy used
    pub strategy: SummarizationStrategy,
    /// Token usage for this summarization
    pub token_usage: Option<TokenUsage>,
    /// Quality score (if available)
    pub quality_score: Option<f64>,
    /// Topics detected in the summarized content
    pub detected_topics: Vec<String>,
}

/// Represents a summarized conversation segment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationSummary {
    /// Summary information
    pub info: SummaryInfo,
    /// The actual summary content
    pub summary_text: String,
    /// Key topics covered
    pub topics: Vec<String>,
    /// Important facts extracted
    pub key_facts: Vec<String>,
    /// Participants mentioned
    pub participants: Vec<String>,
    /// Time range of summarized messages
    pub time_range: (DateTime<Utc>, DateTime<Utc>),
    /// Original message IDs that were summarized
    pub source_message_ids: Vec<String>,
}

/// Intelligent conversation summarizer
pub struct ConversationSummarizer {
    /// Configuration for summarization behavior
    config: RwLock<SummarizationConfig>,
    /// AI service for generating summaries
    ai_service: Arc<dyn AiService>,
    /// Token manager for tracking usage
    token_manager: Option<Arc<TokenManager>>,
    /// Storage for summaries
    summaries: RwLock<Vec<ConversationSummary>>,
    /// Storage path for persistence
    storage_path: std::path::PathBuf,
}

impl ConversationSummarizer {
    /// Create a new conversation summarizer
    pub fn new(
        ai_service: Arc<dyn AiService>,
        token_manager: Option<Arc<TokenManager>>,
        storage_path: std::path::PathBuf,
    ) -> Self {
        Self {
            config: RwLock::new(SummarizationConfig::default()),
            ai_service,
            token_manager,
            summaries: RwLock::new(Vec::new()),
            storage_path,
        }
    }

    /// Update summarization configuration
    pub async fn update_config(&self, config: SummarizationConfig) -> Result<()> {
        *self.config.write().await = config;
        self.save_to_storage().await?;
        info!("Updated summarization configuration");
        Ok(())
    }

    /// Check if conversation needs summarization
    pub async fn should_summarize(&self, messages: &[InternalChatMessage]) -> bool {
        let config = self.config.read().await;
        
        // Check message count threshold
        if messages.len() < config.min_conversation_length {
            return false;
        }
        
        if messages.len() >= config.max_conversation_length {
            return true;
        }
        
        // Check token budget if enabled and available
        if config.auto_summarize_on_budget_limit {
            if let Some(token_manager) = &self.token_manager {
                if let Ok(status) = token_manager.check_budget_status().await {
                    if status.should_auto_summarize {
                        info!("Auto-summarization triggered by budget limit");
                        return true;
                    }
                }
            }
        }
        
        false
    }

    /// Summarize a conversation
    pub async fn summarize_conversation(
        &self,
        messages: &[InternalChatMessage],
        user_id: &str,
        session_id: &str,
    ) -> Result<ConversationSummary> {
        let config = self.config.read().await.clone();
        
        info!("Starting conversation summarization for {} messages", messages.len());
        
        match config.strategy {
            SummarizationStrategy::Single => {
                self.single_summarization(messages, &config, user_id, session_id).await
            }
            SummarizationStrategy::Progressive => {
                self.progressive_summarization(messages, &config, user_id, session_id).await
            }
            SummarizationStrategy::TopicBased => {
                self.topic_based_summarization(messages, &config, user_id, session_id).await
            }
            SummarizationStrategy::Hierarchical => {
                self.hierarchical_summarization(messages, &config, user_id, session_id).await
            }
        }
    }

    /// Create memory blocks from conversation summary
    pub async fn create_memory_blocks(
        &self,
        summary: &ConversationSummary,
        user_id: &str,
        session_id: &str,
    ) -> Result<Vec<MemoryBlock>> {
        let mut blocks = Vec::new();
        
        // Create main summary block
        let summary_block = MemoryBlockBuilder::new()
            .with_type(BlockType::Summary)
            .with_user_id(user_id)
            .with_session_id(session_id)
            .with_content(MemoryContent::Text(summary.summary_text.clone()))
            .with_tag("conversation_summary")
            .with_property("original_message_count", summary.info.original_message_count.to_string())
            .with_property("compression_ratio", summary.info.compression_ratio.to_string())
            .with_property("summary_id", summary.info.id.clone())
            .build()?;
        
        blocks.push(summary_block);
        
        // Create fact blocks for key facts
        for (i, fact) in summary.key_facts.iter().enumerate() {
            let fact_block = MemoryBlockBuilder::new()
                .with_type(BlockType::Fact)
                .with_user_id(user_id)
                .with_session_id(session_id)
                .with_content(MemoryContent::Text(fact.clone()))
                .with_tag("extracted_fact")
                .with_property("fact_index", i.to_string())
                .with_property("source_summary", summary.info.id.clone())
                .build()?;
            
            blocks.push(fact_block);
        }
        
        info!("Created {} memory blocks from conversation summary", blocks.len());
        Ok(blocks)
    }

    /// Get all summaries
    pub async fn get_summaries(&self) -> Vec<ConversationSummary> {
        self.summaries.read().await.clone()
    }

    /// Get summaries by topic
    pub async fn get_summaries_by_topic(&self, topic: &str) -> Vec<ConversationSummary> {
        let summaries = self.summaries.read().await;
        summaries
            .iter()
            .filter(|s| s.topics.iter().any(|t| t.contains(topic)))
            .cloned()
            .collect()
    }

    /// Search summaries by content
    pub async fn search_summaries(&self, query: &str) -> Vec<ConversationSummary> {
        let summaries = self.summaries.read().await;
        let query_lower = query.to_lowercase();
        
        summaries
            .iter()
            .filter(|s| {
                s.summary_text.to_lowercase().contains(&query_lower) ||
                s.key_facts.iter().any(|f| f.to_lowercase().contains(&query_lower)) ||
                s.topics.iter().any(|t| t.to_lowercase().contains(&query_lower))
            })
            .cloned()
            .collect()
    }

    /// Get summarization analytics
    pub async fn get_analytics(&self) -> SummarizationAnalytics {
        let summaries = self.summaries.read().await;
        
        let total_summaries = summaries.len();
        let total_messages_summarized: usize = summaries.iter()
            .map(|s| s.info.original_message_count)
            .sum();
        
        let average_compression_ratio = if total_summaries > 0 {
            summaries.iter()
                .map(|s| s.info.compression_ratio)
                .sum::<f64>() / total_summaries as f64
        } else {
            0.0
        };
        
        let total_tokens_used: u32 = summaries.iter()
            .filter_map(|s| s.info.token_usage.as_ref())
            .map(|u| u.total_tokens)
            .sum();
        
        let topics_frequency = self.calculate_topic_frequency(&summaries);
        
        SummarizationAnalytics {
            total_summaries,
            total_messages_summarized,
            average_compression_ratio,
            total_tokens_used,
            topics_frequency,
            most_productive_hour: self.calculate_most_productive_hour(&summaries),
        }
    }

    // Private helper methods

    async fn single_summarization(
        &self,
        messages: &[InternalChatMessage],
        config: &SummarizationConfig,
        _user_id: &str,
        _session_id: &str,
    ) -> Result<ConversationSummary> {
        // Preserve recent messages
        let messages_to_summarize = if config.preserve_recent_count > 0 && messages.len() > config.preserve_recent_count {
            &messages[..messages.len() - config.preserve_recent_count]
        } else {
            messages
        };
        
        let conversation_text = self.format_messages_for_summarization(messages_to_summarize);
        
        let summary_prompt = format!(
            "Please provide a comprehensive summary of the following conversation. \
            Focus on key topics, important decisions, and factual information. \
            Aim for approximately {} tokens in your summary.\n\n\
            Conversation:\n{}",
            config.target_summary_length,
            conversation_text
        );
        
        let summary_messages = vec![
            InternalChatMessage::System {
                content: "You are an expert conversation summarizer. Create concise but comprehensive summaries.".to_string()
            },
            InternalChatMessage::User {
                content: summary_prompt
            }
        ];
        
        let start_time = Utc::now();
        let response = self.ai_service.generate_response(&summary_messages).await?;
        let end_time = Utc::now();
        
        let summary_text = match response {
            genai::chat::MessageContent::Text(text) => text,
            _ => return Err(anyhow::anyhow!("Expected text response from summarization")),
        };
        
        // Extract topics, facts, and participants (simplified for now)
        let topics = self.extract_topics(&summary_text);
        let key_facts = self.extract_key_facts(&summary_text);
        let participants = self.extract_participants(messages_to_summarize);
        
        let summary_id = format!("summary_{}", Utc::now().timestamp());
        
        let summary = ConversationSummary {
            info: SummaryInfo {
                id: summary_id,
                created_at: start_time,
                original_message_count: messages_to_summarize.len(),
                compression_ratio: self.calculate_compression_ratio(&conversation_text, &summary_text),
                strategy: config.strategy.clone(),
                token_usage: None, // Will be filled by token manager if available
                quality_score: None, // Could be implemented later
                detected_topics: topics.clone(),
            },
            summary_text,
            topics,
            key_facts,
            participants,
            time_range: (start_time, end_time),
            source_message_ids: self.extract_message_ids(messages_to_summarize),
        };
        
        // Store the summary
        self.summaries.write().await.push(summary.clone());
        self.save_to_storage().await?;
        
        info!("Successfully created conversation summary with {} compression ratio", 
              summary.info.compression_ratio);
        
        Ok(summary)
    }

    async fn progressive_summarization(
        &self,
        messages: &[InternalChatMessage],
        config: &SummarizationConfig,
        _user_id: &str,
        _session_id: &str,
    ) -> Result<ConversationSummary> {
        // For progressive summarization, we'd combine with existing summaries
        // For now, fall back to single summarization
        warn!("Progressive summarization not fully implemented, falling back to single");
        self.single_summarization(messages, config, "default_user", "default_session").await
    }

    async fn topic_based_summarization(
        &self,
        messages: &[InternalChatMessage],
        config: &SummarizationConfig,
        _user_id: &str,
        _session_id: &str,
    ) -> Result<ConversationSummary> {
        // For topic-based summarization, we'd group messages by topic first
        // For now, fall back to single summarization
        warn!("Topic-based summarization not fully implemented, falling back to single");
        self.single_summarization(messages, config, "default_user", "default_session").await
    }

    async fn hierarchical_summarization(
        &self,
        messages: &[InternalChatMessage],
        config: &SummarizationConfig,
        _user_id: &str,
        _session_id: &str,
    ) -> Result<ConversationSummary> {
        // For hierarchical summarization, we'd create multiple levels of summaries
        // For now, fall back to single summarization
        warn!("Hierarchical summarization not fully implemented, falling back to single");
        self.single_summarization(messages, config, "default_user", "default_session").await
    }

    fn format_messages_for_summarization(&self, messages: &[InternalChatMessage]) -> String {
        messages
            .iter()
            .map(|msg| match msg {
                InternalChatMessage::System { content } => format!("System: {}", content),
                InternalChatMessage::User { content } => format!("User: {}", content),
                InternalChatMessage::Assistant { content, .. } => format!("Assistant: {}", content),
                InternalChatMessage::Tool { tool_name, content, .. } => {
                    format!("Tool ({}): {}", tool_name, content)
                }
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    fn extract_topics(&self, summary_text: &str) -> Vec<String> {
        // Simple topic extraction (could be enhanced with NLP)
        let potential_topics = [
            "technology", "AI", "programming", "business", "science", "health",
            "education", "finance", "marketing", "design", "productivity", "learning"
        ];
        
        potential_topics
            .iter()
            .filter(|topic| summary_text.to_lowercase().contains(&topic.to_lowercase()))
            .map(|s| s.to_string())
            .collect()
    }

    fn extract_key_facts(&self, summary_text: &str) -> Vec<String> {
        // Simple fact extraction (split by sentences and filter)
        summary_text
            .split('.')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty() && s.len() > 20) // Filter out very short fragments
            .take(5) // Limit to top 5 facts
            .map(|s| s.to_string())
            .collect()
    }

    fn extract_participants(&self, messages: &[InternalChatMessage]) -> Vec<String> {
        let mut participants = std::collections::HashSet::new();
        
        for message in messages {
            match message {
                InternalChatMessage::User { .. } => {
                    participants.insert("User".to_string());
                }
                InternalChatMessage::Assistant { .. } => {
                    participants.insert("Assistant".to_string());
                }
                InternalChatMessage::System { .. } => {
                    participants.insert("System".to_string());
                }
                InternalChatMessage::Tool { tool_name, .. } => {
                    participants.insert(format!("Tool({})", tool_name));
                }
            }
        }
        
        participants.into_iter().collect()
    }

    fn extract_message_ids(&self, messages: &[InternalChatMessage]) -> Vec<String> {
        // For now, generate simple IDs based on content hash
        messages
            .iter()
            .enumerate()
            .map(|(i, msg)| {
                let content = match msg {
                    InternalChatMessage::System { content } => content,
                    InternalChatMessage::User { content } => content,
                    InternalChatMessage::Assistant { content, .. } => content,
                    InternalChatMessage::Tool { content, .. } => content,
                };
                format!("msg_{}_{}", i, content.len())
            })
            .collect()
    }

    fn calculate_compression_ratio(&self, original: &str, summary: &str) -> f64 {
        let original_tokens = original.split_whitespace().count() as f64;
        let summary_tokens = summary.split_whitespace().count() as f64;
        
        if summary_tokens > 0.0 {
            original_tokens / summary_tokens
        } else {
            1.0
        }
    }

    fn calculate_topic_frequency(&self, summaries: &[ConversationSummary]) -> std::collections::HashMap<String, usize> {
        let mut frequency = std::collections::HashMap::new();
        
        for summary in summaries {
            for topic in &summary.topics {
                *frequency.entry(topic.clone()).or_insert(0) += 1;
            }
        }
        
        frequency
    }

    fn calculate_most_productive_hour(&self, summaries: &[ConversationSummary]) -> Option<u32> {
        if summaries.is_empty() {
            return None;
        }
        
        let mut hour_count = std::collections::HashMap::new();
        
        for summary in summaries {
            let hour = summary.info.created_at.hour();
            *hour_count.entry(hour).or_insert(0) += 1;
        }
        
        hour_count
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(hour, _)| hour)
    }

    async fn save_to_storage(&self) -> Result<()> {
        // Create storage directory if it doesn't exist
        if let Some(parent) = self.storage_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let summaries = self.summaries.read().await;
        let config = self.config.read().await;

        let storage_data = SummarizationStorageData {
            summaries: summaries.clone(),
            config: config.clone(),
        };

        let json = serde_json::to_string_pretty(&storage_data)?;
        tokio::fs::write(&self.storage_path, json).await?;
        
        Ok(())
    }

    /// Load summarizer from storage
    pub async fn load_from_storage(
        storage_path: std::path::PathBuf,
        ai_service: Arc<dyn AiService>,
        token_manager: Option<Arc<TokenManager>>,
    ) -> Result<Self> {
        let summarizer = Self::new(ai_service, token_manager, storage_path.clone());
        
        if storage_path.exists() {
            let json = tokio::fs::read_to_string(&storage_path).await?;
            let storage_data: SummarizationStorageData = serde_json::from_str(&json)?;
            
            *summarizer.summaries.write().await = storage_data.summaries;
            *summarizer.config.write().await = storage_data.config;
            
            info!("Loaded conversation summarizer from storage");
        }
        
        Ok(summarizer)
    }
}

/// Analytics about summarization performance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummarizationAnalytics {
    /// Total number of summaries created
    pub total_summaries: usize,
    /// Total messages that have been summarized
    pub total_messages_summarized: usize,
    /// Average compression ratio across all summaries
    pub average_compression_ratio: f64,
    /// Total tokens used for summarization
    pub total_tokens_used: u32,
    /// Frequency of topics across summaries
    pub topics_frequency: std::collections::HashMap<String, usize>,
    /// Most productive hour (when most summaries are created)
    pub most_productive_hour: Option<u32>,
}

/// Storage data structure
#[derive(Debug, Serialize, Deserialize)]
struct SummarizationStorageData {
    summaries: Vec<ConversationSummary>,
    config: SummarizationConfig,
}