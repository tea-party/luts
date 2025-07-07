//! Conversation search and filtering system
//!
//! This module provides advanced search and filtering capabilities for conversations,
//! supporting full-text search, semantic search, and complex filtering criteria.

use crate::memory::{MemoryManager, BlockType};
use crate::conversation::export::{ExportableConversation, MessageType, ConversationMetadata};
use crate::utils::tokens::TokenManager;
use anyhow::Result;
use chrono::{DateTime, Utc, Duration, Timelike};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Advanced search query for conversations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationSearchQuery {
    /// Full-text search terms
    pub text_query: Option<String>,
    /// Semantic search query (for embedding-based search)
    pub semantic_query: Option<String>,
    /// Filters to apply
    pub filters: SearchFilters,
    /// Search scope
    pub scope: SearchScope,
    /// Result sorting options
    pub sort: SearchSortOptions,
    /// Maximum number of results
    pub limit: Option<usize>,
    /// Result offset for pagination
    pub offset: Option<usize>,
    /// Include match highlights
    pub include_highlights: bool,
    /// Search explanation/debugging
    pub explain: bool,
}

impl Default for ConversationSearchQuery {
    fn default() -> Self {
        Self {
            text_query: None,
            semantic_query: None,
            filters: SearchFilters::default(),
            scope: SearchScope::All,
            sort: SearchSortOptions::default(),
            limit: Some(20),
            offset: None,
            include_highlights: true,
            explain: false,
        }
    }
}

/// Search filters for conversations
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SearchFilters {
    /// Filter by user IDs
    pub user_ids: Option<Vec<String>>,
    /// Filter by session IDs
    pub session_ids: Option<Vec<String>>,
    /// Filter by conversation status
    pub status: Option<Vec<ConversationStatus>>,
    /// Date range filter
    pub date_range: Option<DateRangeFilter>,
    /// Message count range
    pub message_count_range: Option<RangeFilter<usize>>,
    /// Token usage range
    pub token_range: Option<RangeFilter<u32>>,
    /// Filter by tags
    pub tags: Option<TagFilter>,
    /// Filter by message types
    pub message_types: Option<Vec<MessageType>>,
    /// Filter by participants
    pub participants: Option<Vec<String>>,
    /// Filter by language
    pub language: Option<String>,
    /// Custom property filters
    pub properties: Option<HashMap<String, PropertyFilter>>,
    /// Filter by conversation duration
    pub duration_range: Option<RangeFilter<i64>>, // Duration in seconds
    /// Filter by importance/relevance
    pub importance: Option<ImportanceFilter>,
    /// Advanced content filters
    pub content_filters: Option<ContentFilters>,
}

/// Status of a conversation for filtering
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConversationStatus {
    Active,
    Archived,
    Completed,
    Paused,
    Deleted,
    Bookmarked,
    Important,
}

/// Date range filter options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateRangeFilter {
    /// Start date (inclusive)
    pub start: Option<DateTime<Utc>>,
    /// End date (inclusive)
    pub end: Option<DateTime<Utc>>,
    /// Relative date shortcuts
    pub relative: Option<RelativeDateRange>,
}

/// Relative date range options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RelativeDateRange {
    Today,
    Yesterday,
    LastWeek,
    LastMonth,
    LastThreeMonths,
    LastYear,
    Custom(Duration),
}

/// Generic range filter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RangeFilter<T> {
    pub min: Option<T>,
    pub max: Option<T>,
}

/// Tag filtering options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagFilter {
    /// Tags that must be present (AND)
    pub required: Option<Vec<String>>,
    /// Tags where at least one must be present (OR)
    pub any_of: Option<Vec<String>>,
    /// Tags that must not be present (NOT)
    pub excluded: Option<Vec<String>>,
}

/// Property filtering for custom conversation properties
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PropertyFilter {
    /// Exact string match
    Equals(String),
    /// String contains
    Contains(String),
    /// Regex pattern match
    Regex(String),
    /// Numeric comparison
    Numeric(RangeFilter<f64>),
    /// Boolean value
    Boolean(bool),
    /// Value exists
    Exists,
    /// Value doesn't exist
    NotExists,
}

/// Importance/relevance filtering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportanceFilter {
    /// Minimum relevance score
    pub min_relevance: Option<f64>,
    /// Conversations marked as important
    pub important_only: Option<bool>,
    /// Conversations with bookmarks
    pub bookmarked_only: Option<bool>,
}

/// Advanced content filtering options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentFilters {
    /// Filter by sentiment
    pub sentiment: Option<SentimentFilter>,
    /// Filter by detected topics
    pub topics: Option<Vec<String>>,
    /// Filter by detected entities
    pub entities: Option<Vec<String>>,
    /// Filter by conversation complexity
    pub complexity: Option<ComplexityFilter>,
    /// Filter by response quality
    pub quality: Option<QualityFilter>,
}

/// Sentiment filtering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SentimentFilter {
    Positive,
    Negative,
    Neutral,
    Mixed,
    Range(f64, f64), // min, max sentiment score
}

/// Complexity filtering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComplexityFilter {
    Simple,
    Moderate,
    Complex,
    VeryComplex,
    Range(f64, f64), // min, max complexity score
}

/// Quality filtering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QualityFilter {
    High,
    Medium,
    Low,
    Range(f64, f64), // min, max quality score
}

/// Search scope options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SearchScope {
    /// Search all conversations
    All,
    /// Search only message content
    MessageContent,
    /// Search only conversation metadata
    Metadata,
    /// Search memory blocks
    MemoryBlocks,
    /// Search summaries
    Summaries,
    /// Search specific fields
    Fields(Vec<String>),
}

/// Search result sorting options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchSortOptions {
    /// Primary sort field
    pub primary: SortField,
    /// Secondary sort field
    pub secondary: Option<SortField>,
    /// Sort direction
    pub direction: SortDirection,
}

impl Default for SearchSortOptions {
    fn default() -> Self {
        Self {
            primary: SortField::Relevance,
            secondary: Some(SortField::LastMessageTime),
            direction: SortDirection::Descending,
        }
    }
}

/// Available sort fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SortField {
    /// Search relevance score
    Relevance,
    /// Conversation creation time
    CreatedAt,
    /// Last message time
    LastMessageTime,
    /// Message count
    MessageCount,
    /// Token usage
    TokenCount,
    /// Conversation duration
    Duration,
    /// Conversation title
    Title,
    /// User ID
    UserId,
    /// Custom property
    Property(String),
}

/// Sort direction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SortDirection {
    Ascending,
    Descending,
}

/// Search result with relevance information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationSearchResult {
    /// Conversation metadata
    pub conversation: ConversationMetadata,
    /// Search relevance score (0.0 to 1.0)
    pub relevance_score: f64,
    /// Highlighted matches in content
    pub highlights: Vec<SearchHighlight>,
    /// Match explanations (if requested)
    pub explanation: Option<SearchExplanation>,
    /// Matching messages (summary)
    pub matching_messages: Vec<MessageMatch>,
    /// Associated memory blocks that matched
    pub matching_blocks: Vec<MemoryBlockMatch>,
}

/// Search highlight information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHighlight {
    /// Field that contained the match
    pub field: String,
    /// Original text with highlights
    pub highlighted_text: String,
    /// Match positions
    pub positions: Vec<HighlightPosition>,
}

/// Highlight position information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HighlightPosition {
    /// Start character position
    pub start: usize,
    /// End character position
    pub end: usize,
    /// Matched term
    pub term: String,
}

/// Search explanation for debugging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchExplanation {
    /// Query analysis
    pub query_analysis: String,
    /// Filters applied
    pub filters_applied: Vec<String>,
    /// Scoring breakdown
    pub score_breakdown: HashMap<String, f64>,
    /// Processing time in milliseconds
    pub processing_time_ms: u64,
}

/// Message match information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageMatch {
    /// Message ID
    pub message_id: String,
    /// Message type
    pub message_type: MessageType,
    /// Message timestamp
    pub timestamp: DateTime<Utc>,
    /// Snippet of matching content
    pub snippet: String,
    /// Match score for this message
    pub score: f64,
}

/// Memory block match information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryBlockMatch {
    /// Block ID
    pub block_id: String,
    /// Block type
    pub block_type: BlockType,
    /// Snippet of matching content
    pub snippet: String,
    /// Match score for this block
    pub score: f64,
}

/// Search result summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResultSummary {
    /// Total number of results found
    pub total_results: usize,
    /// Number of results returned
    pub returned_results: usize,
    /// Search processing time in milliseconds
    pub processing_time_ms: u64,
    /// Query terms used
    pub query_terms: Vec<String>,
    /// Filters applied
    pub filters_applied: usize,
    /// Search suggestions
    pub suggestions: Vec<SearchSuggestion>,
}

/// Search suggestion for improving queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchSuggestion {
    /// Type of suggestion
    pub suggestion_type: SuggestionType,
    /// Suggestion text
    pub text: String,
    /// Modified query if user accepts suggestion
    pub modified_query: Option<ConversationSearchQuery>,
}

/// Types of search suggestions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SuggestionType {
    /// Spelling correction
    SpellingCorrection,
    /// Query expansion
    QueryExpansion,
    /// Filter suggestion
    FilterSuggestion,
    /// Scope suggestion
    ScopeSuggestion,
    /// Related terms
    RelatedTerms,
}

/// Saved search query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedSearch {
    /// Unique ID for the saved search
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Description of the search
    pub description: Option<String>,
    /// The search query
    pub query: ConversationSearchQuery,
    /// When this search was created
    pub created_at: DateTime<Utc>,
    /// When this search was last used
    pub last_used: Option<DateTime<Utc>>,
    /// How many times this search has been used
    pub usage_count: usize,
    /// Whether this is a favorite search
    pub is_favorite: bool,
    /// Tags for organizing searches
    pub tags: Vec<String>,
}

/// Search analytics data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchAnalytics {
    /// Total searches performed
    pub total_searches: usize,
    /// Most common search terms
    pub popular_terms: Vec<(String, usize)>,
    /// Most used filters
    pub popular_filters: Vec<(String, usize)>,
    /// Average search processing time
    pub avg_processing_time_ms: f64,
    /// Search success rate (searches with results)
    pub success_rate: f64,
    /// Most active search users
    pub active_users: Vec<(String, usize)>,
    /// Search patterns by time of day
    pub hourly_patterns: Vec<usize>,
}

/// Conversation search and filtering engine
pub struct ConversationSearchEngine {
    /// Memory manager for searching memory blocks
    memory_manager: Option<Arc<MemoryManager>>,
    /// Token manager for token-based filtering
    token_manager: Option<Arc<TokenManager>>,
    /// Saved searches
    saved_searches: RwLock<HashMap<String, SavedSearch>>,
    /// Search analytics
    analytics: RwLock<SearchAnalytics>,
    /// Search index for fast text search
    search_index: RwLock<SearchIndex>,
    /// Configuration
    config: RwLock<SearchConfig>,
}

/// Search configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    /// Maximum results per search
    pub max_results: usize,
    /// Default search timeout in seconds
    pub search_timeout_seconds: u64,
    /// Enable fuzzy matching
    pub enable_fuzzy_search: bool,
    /// Fuzzy search threshold
    pub fuzzy_threshold: f64,
    /// Enable semantic search
    pub enable_semantic_search: bool,
    /// Search result caching duration
    pub cache_duration_seconds: u64,
    /// Enable search analytics
    pub enable_analytics: bool,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            max_results: 100,
            search_timeout_seconds: 30,
            enable_fuzzy_search: true,
            fuzzy_threshold: 0.8,
            enable_semantic_search: false, // Requires additional embedding models
            cache_duration_seconds: 300, // 5 minutes
            enable_analytics: true,
        }
    }
}

/// Search index for fast text search
#[derive(Debug, Default)]
struct SearchIndex {
    /// Conversation text index
    conversations: HashMap<String, ConversationIndex>,
    /// Global term frequency
    term_frequencies: HashMap<String, usize>,
    /// Last index update time
    last_updated: Option<DateTime<Utc>>,
}

/// Individual conversation index
#[derive(Debug)]
struct ConversationIndex {
    /// Conversation metadata
    metadata: ConversationMetadata,
    /// Indexed terms and their positions
    terms: HashMap<String, Vec<TermPosition>>,
    /// Message content
    messages: Vec<IndexedMessage>,
}

/// Term position in conversation
#[derive(Debug)]
struct TermPosition {
    /// Message index
    message_index: usize,
    /// Character position in message
    position: usize,
    /// Term frequency in this position
    frequency: usize,
}

/// Indexed message for search
#[derive(Debug)]
struct IndexedMessage {
    /// Message ID
    id: String,
    /// Message type
    message_type: MessageType,
    /// Message content (lowercased for search)
    content: String,
    /// Original content
    original_content: String,
    /// Timestamp
    timestamp: DateTime<Utc>,
    /// Author
    author: String,
}

impl ConversationSearchEngine {
    /// Create a new search engine
    pub fn new() -> Self {
        Self {
            memory_manager: None,
            token_manager: None,
            saved_searches: RwLock::new(HashMap::new()),
            analytics: RwLock::new(SearchAnalytics {
                total_searches: 0,
                popular_terms: Vec::new(),
                popular_filters: Vec::new(),
                avg_processing_time_ms: 0.0,
                success_rate: 0.0,
                active_users: Vec::new(),
                hourly_patterns: vec![0; 24],
            }),
            search_index: RwLock::new(SearchIndex::default()),
            config: RwLock::new(SearchConfig::default()),
        }
    }

    /// Create search engine with components
    pub fn new_with_components(
        memory_manager: Option<Arc<MemoryManager>>,
        token_manager: Option<Arc<TokenManager>>,
    ) -> Self {
        let mut engine = Self::new();
        engine.memory_manager = memory_manager;
        engine.token_manager = token_manager;
        engine
    }

    /// Search conversations
    pub async fn search_conversations(
        &self,
        query: ConversationSearchQuery,
    ) -> Result<(Vec<ConversationSearchResult>, SearchResultSummary)> {
        let start_time = std::time::Instant::now();
        info!("Starting conversation search with query: {:?}", query);

        // Update analytics
        if self.config.read().await.enable_analytics {
            self.update_search_analytics(&query).await;
        }

        // Build search results
        let mut results = Vec::new();
        let search_index = self.search_index.read().await;

        // Perform text search if query specified
        if let Some(ref text_query) = query.text_query {
            let text_results = self.perform_text_search(text_query, &query, &search_index).await?;
            results.extend(text_results);
        }

        // Perform semantic search if enabled and query specified
        if let Some(ref _semantic_query) = query.semantic_query {
            if self.config.read().await.enable_semantic_search {
                warn!("Semantic search requested but not yet implemented");
                // Would implement semantic search here
            }
        }

        // Apply filters
        results = self.apply_filters(results, &query.filters).await?;

        // Sort results
        results = self.sort_results(results, &query.sort).await;

        // Apply pagination
        let total_results = results.len();
        if let Some(offset) = query.offset {
            if offset < results.len() {
                results.drain(0..offset);
            } else {
                results.clear();
            }
        }
        if let Some(limit) = query.limit {
            results.truncate(limit);
        }

        let processing_time = start_time.elapsed().as_millis() as u64;

        let summary = SearchResultSummary {
            total_results,
            returned_results: results.len(),
            processing_time_ms: processing_time,
            query_terms: self.extract_query_terms(&query),
            filters_applied: self.count_active_filters(&query.filters),
            suggestions: self.generate_suggestions(&query, &results).await,
        };

        info!("Search completed: {} results in {}ms", results.len(), processing_time);
        Ok((results, summary))
    }

    /// Index a conversation for searching
    pub async fn index_conversation(
        &self,
        conversation: &ExportableConversation,
    ) -> Result<()> {
        let mut search_index = self.search_index.write().await;
        
        let mut indexed_messages = Vec::new();
        let mut terms = HashMap::new();

        for (msg_idx, message) in conversation.messages.iter().enumerate() {
            let indexed_message = IndexedMessage {
                id: message.id.clone(),
                message_type: message.message_type.clone(),
                content: message.content.to_lowercase(),
                original_content: message.content.clone(),
                timestamp: message.timestamp,
                author: message.author.clone(),
            };

            // Extract and index terms
            let words: Vec<&str> = indexed_message.content.split_whitespace().collect();
            for (pos, word) in words.iter().enumerate() {
                let term = word.trim_matches(|c: char| !c.is_alphanumeric()).to_string();
                if !term.is_empty() && term.len() > 2 {
                    terms.entry(term.clone())
                        .or_insert_with(Vec::new)
                        .push(TermPosition {
                            message_index: msg_idx,
                            position: pos,
                            frequency: 1,
                        });
                    
                    // Update global term frequency
                    *search_index.term_frequencies.entry(term).or_insert(0) += 1;
                }
            }

            indexed_messages.push(indexed_message);
        }

        let conversation_index = ConversationIndex {
            metadata: conversation.metadata.clone(),
            terms,
            messages: indexed_messages,
        };

        search_index.conversations.insert(conversation.metadata.id.clone(), conversation_index);
        search_index.last_updated = Some(Utc::now());

        info!("Indexed conversation: {}", conversation.metadata.id);
        Ok(())
    }

    /// Save a search query for later use
    pub async fn save_search(
        &self,
        name: String,
        description: Option<String>,
        query: ConversationSearchQuery,
        tags: Vec<String>,
    ) -> Result<String> {
        let search_id = format!("search_{}_{}", Utc::now().timestamp(), uuid::Uuid::new_v4().to_string()[..8].to_string());
        
        let saved_search = SavedSearch {
            id: search_id.clone(),
            name,
            description,
            query,
            created_at: Utc::now(),
            last_used: None,
            usage_count: 0,
            is_favorite: false,
            tags,
        };

        self.saved_searches.write().await.insert(search_id.clone(), saved_search);
        info!("Saved search: {}", search_id);
        Ok(search_id)
    }

    /// Load a saved search
    pub async fn load_saved_search(&self, search_id: &str) -> Option<SavedSearch> {
        let mut saved_searches = self.saved_searches.write().await;
        if let Some(saved_search) = saved_searches.get_mut(search_id) {
            saved_search.last_used = Some(Utc::now());
            saved_search.usage_count += 1;
            Some(saved_search.clone())
        } else {
            None
        }
    }

    /// List saved searches
    pub async fn list_saved_searches(&self, _user_id: Option<&str>) -> Vec<SavedSearch> {
        let saved_searches = self.saved_searches.read().await;
        saved_searches.values().cloned().collect()
    }

    /// Get search analytics
    pub async fn get_search_analytics(&self) -> SearchAnalytics {
        self.analytics.read().await.clone()
    }

    // Private helper methods

    async fn perform_text_search(
        &self,
        text_query: &str,
        query: &ConversationSearchQuery,
        search_index: &SearchIndex,
    ) -> Result<Vec<ConversationSearchResult>> {
        let mut results = Vec::new();
        let query_terms: Vec<&str> = text_query.split_whitespace().collect();

        for (_conv_id, conv_index) in &search_index.conversations {
            let mut relevance_score = 0.0;
            let highlights = Vec::new();
            let mut matching_messages = Vec::new();

            // Calculate relevance based on term matches
            for term in &query_terms {
                let term_lower = term.to_lowercase();
                if let Some(positions) = conv_index.terms.get(&term_lower) {
                    relevance_score += positions.len() as f64 * 0.1;
                    
                    // Create highlights and matching messages
                    for position in positions {
                        if let Some(message) = conv_index.messages.get(position.message_index) {
                            matching_messages.push(MessageMatch {
                                message_id: message.id.clone(),
                                message_type: message.message_type.clone(),
                                timestamp: message.timestamp,
                                snippet: self.create_snippet(&message.original_content, term, 100),
                                score: 0.5, // Simplified scoring
                            });
                        }
                    }
                }
            }

            if relevance_score > 0.0 {
                results.push(ConversationSearchResult {
                    conversation: conv_index.metadata.clone(),
                    relevance_score: relevance_score.min(1.0),
                    highlights,
                    explanation: if query.explain { 
                        Some(SearchExplanation {
                            query_analysis: format!("Matched {} terms", query_terms.len()),
                            filters_applied: Vec::new(),
                            score_breakdown: HashMap::new(),
                            processing_time_ms: 0,
                        })
                    } else { 
                        None 
                    },
                    matching_messages,
                    matching_blocks: Vec::new(),
                });
            }
        }

        Ok(results)
    }

    async fn apply_filters(
        &self,
        mut results: Vec<ConversationSearchResult>,
        filters: &SearchFilters,
    ) -> Result<Vec<ConversationSearchResult>> {
        // Apply user ID filter
        if let Some(ref user_ids) = filters.user_ids {
            results.retain(|r| user_ids.contains(&r.conversation.user_id));
        }

        // Apply date range filter
        if let Some(ref date_range) = filters.date_range {
            results.retain(|r| self.matches_date_range(&r.conversation.started_at, date_range));
        }

        // Apply message count filter
        if let Some(ref msg_range) = filters.message_count_range {
            results.retain(|r| {
                let count = r.conversation.message_count;
                msg_range.min.map_or(true, |min| count >= min) &&
                msg_range.max.map_or(true, |max| count <= max)
            });
        }

        // Apply tag filter
        if let Some(ref tag_filter) = filters.tags {
            results.retain(|r| self.matches_tag_filter(&r.conversation.tags, tag_filter));
        }

        Ok(results)
    }

    async fn sort_results(
        &self,
        mut results: Vec<ConversationSearchResult>,
        sort_options: &SearchSortOptions,
    ) -> Vec<ConversationSearchResult> {
        results.sort_by(|a, b| {
            let primary_cmp = self.compare_by_field(a, b, &sort_options.primary);
            if primary_cmp != std::cmp::Ordering::Equal {
                match sort_options.direction {
                    SortDirection::Ascending => primary_cmp,
                    SortDirection::Descending => primary_cmp.reverse(),
                }
            } else if let Some(ref secondary) = sort_options.secondary {
                let secondary_cmp = self.compare_by_field(a, b, secondary);
                match sort_options.direction {
                    SortDirection::Ascending => secondary_cmp,
                    SortDirection::Descending => secondary_cmp.reverse(),
                }
            } else {
                std::cmp::Ordering::Equal
            }
        });

        results
    }

    fn compare_by_field(
        &self,
        a: &ConversationSearchResult,
        b: &ConversationSearchResult,
        field: &SortField,
    ) -> std::cmp::Ordering {
        match field {
            SortField::Relevance => a.relevance_score.partial_cmp(&b.relevance_score).unwrap_or(std::cmp::Ordering::Equal),
            SortField::CreatedAt => a.conversation.started_at.cmp(&b.conversation.started_at),
            SortField::LastMessageTime => a.conversation.last_message_at.cmp(&b.conversation.last_message_at),
            SortField::MessageCount => a.conversation.message_count.cmp(&b.conversation.message_count),
            SortField::Title => a.conversation.title.cmp(&b.conversation.title),
            SortField::UserId => a.conversation.user_id.cmp(&b.conversation.user_id),
            _ => std::cmp::Ordering::Equal, // Simplified for other fields
        }
    }

    async fn update_search_analytics(&self, query: &ConversationSearchQuery) {
        let mut analytics = self.analytics.write().await;
        analytics.total_searches += 1;

        // Update hourly patterns
        let hour = Utc::now().hour() as usize;
        if hour < 24 {
            analytics.hourly_patterns[hour] += 1;
        }

        // Extract and track query terms
        if let Some(ref text_query) = query.text_query {
            for term in text_query.split_whitespace() {
                let entry = analytics.popular_terms.iter_mut()
                    .find(|(t, _)| t == term);
                if let Some((_, count)) = entry {
                    *count += 1;
                } else {
                    analytics.popular_terms.push((term.to_string(), 1));
                }
            }
        }
    }

    fn extract_query_terms(&self, query: &ConversationSearchQuery) -> Vec<String> {
        let mut terms = Vec::new();
        if let Some(ref text_query) = query.text_query {
            terms.extend(text_query.split_whitespace().map(|s| s.to_string()));
        }
        if let Some(ref semantic_query) = query.semantic_query {
            terms.extend(semantic_query.split_whitespace().map(|s| s.to_string()));
        }
        terms
    }

    fn count_active_filters(&self, filters: &SearchFilters) -> usize {
        let mut count = 0;
        if filters.user_ids.is_some() { count += 1; }
        if filters.session_ids.is_some() { count += 1; }
        if filters.status.is_some() { count += 1; }
        if filters.date_range.is_some() { count += 1; }
        if filters.message_count_range.is_some() { count += 1; }
        if filters.token_range.is_some() { count += 1; }
        if filters.tags.is_some() { count += 1; }
        if filters.message_types.is_some() { count += 1; }
        if filters.participants.is_some() { count += 1; }
        if filters.language.is_some() { count += 1; }
        if filters.properties.is_some() { count += 1; }
        if filters.duration_range.is_some() { count += 1; }
        if filters.importance.is_some() { count += 1; }
        if filters.content_filters.is_some() { count += 1; }
        count
    }

    async fn generate_suggestions(
        &self,
        query: &ConversationSearchQuery,
        results: &[ConversationSearchResult],
    ) -> Vec<SearchSuggestion> {
        let mut suggestions = Vec::new();

        // Suggest query expansion if few results
        if results.len() < 3 && query.text_query.is_some() {
            suggestions.push(SearchSuggestion {
                suggestion_type: SuggestionType::QueryExpansion,
                text: "Try broader search terms or remove some filters".to_string(),
                modified_query: None,
            });
        }

        // Suggest filters if too many results
        if results.len() > 50 {
            suggestions.push(SearchSuggestion {
                suggestion_type: SuggestionType::FilterSuggestion,
                text: "Try adding date or user filters to narrow results".to_string(),
                modified_query: None,
            });
        }

        suggestions
    }

    fn matches_date_range(&self, date: &DateTime<Utc>, filter: &DateRangeFilter) -> bool {
        if let Some(start) = filter.start {
            if *date < start {
                return false;
            }
        }
        if let Some(end) = filter.end {
            if *date > end {
                return false;
            }
        }
        if let Some(ref relative) = filter.relative {
            let now = Utc::now();
            let threshold = match relative {
                RelativeDateRange::Today => now - Duration::days(1),
                RelativeDateRange::Yesterday => now - Duration::days(2),
                RelativeDateRange::LastWeek => now - Duration::weeks(1),
                RelativeDateRange::LastMonth => now - Duration::days(30),
                RelativeDateRange::LastThreeMonths => now - Duration::days(90),
                RelativeDateRange::LastYear => now - Duration::days(365),
                RelativeDateRange::Custom(duration) => now - *duration,
            };
            return *date >= threshold;
        }
        true
    }

    fn matches_tag_filter(&self, tags: &[String], filter: &TagFilter) -> bool {
        if let Some(ref required) = filter.required {
            if !required.iter().all(|tag| tags.contains(tag)) {
                return false;
            }
        }
        if let Some(ref any_of) = filter.any_of {
            if !any_of.iter().any(|tag| tags.contains(tag)) {
                return false;
            }
        }
        if let Some(ref excluded) = filter.excluded {
            if excluded.iter().any(|tag| tags.contains(tag)) {
                return false;
            }
        }
        true
    }

    fn create_snippet(&self, content: &str, term: &str, max_length: usize) -> String {
        let term_lower = term.to_lowercase();
        let content_lower = content.to_lowercase();
        
        if let Some(pos) = content_lower.find(&term_lower) {
            let start = pos.saturating_sub(max_length / 2);
            let end = (pos + term.len() + max_length / 2).min(content.len());
            
            let mut snippet = content[start..end].to_string();
            if start > 0 {
                snippet = format!("...{}", snippet);
            }
            if end < content.len() {
                snippet = format!("{}...", snippet);
            }
            snippet
        } else {
            content.chars().take(max_length).collect()
        }
    }
}