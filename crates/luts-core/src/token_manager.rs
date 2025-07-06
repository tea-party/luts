//! Token usage tracking and management system
//!
//! This module provides comprehensive token tracking, budgeting, and analytics
//! for AI conversations and tool usage, integrating with genai's Usage struct.

use anyhow::Result;
use chrono::{DateTime, Datelike, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Token usage statistics for a single operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Input/prompt tokens consumed
    pub input_tokens: u32,
    /// Output/completion tokens generated
    pub output_tokens: u32,
    /// Total tokens (input + output)
    pub total_tokens: u32,
    /// Estimated cost in USD (if available)
    pub estimated_cost: Option<f64>,
    /// Timestamp of usage
    pub timestamp: DateTime<Utc>,
    /// Provider used (e.g., "openai", "anthropic", "gemini")
    pub provider: String,
    /// Model used (e.g., "gpt-4", "claude-3-opus")
    pub model: String,
    /// Operation type (chat, tool, summarization, etc.)
    pub operation_type: String,
    /// Session ID
    pub session_id: String,
    /// User ID
    pub user_id: String,
}

impl TokenUsage {
    /// Create TokenUsage from genai Usage struct
    pub fn from_genai_usage(
        usage: &genai::chat::Usage,
        provider: String,
        model: String,
        operation_type: String,
        session_id: String,
        user_id: String,
    ) -> Self {
        Self {
            input_tokens: usage.prompt_tokens.unwrap_or(0) as u32,
            output_tokens: usage.completion_tokens.unwrap_or(0) as u32,
            total_tokens: usage.total_tokens.unwrap_or(0) as u32,
            estimated_cost: None, // Will be calculated by TokenManager
            timestamp: Utc::now(),
            provider,
            model,
            operation_type,
            session_id,
            user_id,
        }
    }
}

/// Token budget configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBudget {
    /// Daily token limit
    pub daily_limit: Option<u32>,
    /// Monthly token limit  
    pub monthly_limit: Option<u32>,
    /// Daily cost limit in USD
    pub daily_cost_limit: Option<f64>,
    /// Monthly cost limit in USD
    pub monthly_cost_limit: Option<f64>,
    /// Warning threshold (percentage of limit)
    pub warning_threshold: f64,
    /// Auto-summarize when approaching limit
    pub auto_summarize_on_limit: bool,
}

impl Default for TokenBudget {
    fn default() -> Self {
        Self {
            daily_limit: Some(100_000),      // 100K tokens per day
            monthly_limit: Some(2_000_000),  // 2M tokens per month
            daily_cost_limit: Some(50.0),    // $50 per day
            monthly_cost_limit: Some(1000.0), // $1000 per month
            warning_threshold: 0.8,           // Warn at 80%
            auto_summarize_on_limit: true,
        }
    }
}

/// Token usage analytics and summaries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenAnalytics {
    /// Total tokens used today
    pub daily_tokens: u32,
    /// Total tokens used this month
    pub monthly_tokens: u32,
    /// Total cost today
    pub daily_cost: f64,
    /// Total cost this month
    pub monthly_cost: f64,
    /// Usage by provider
    pub provider_breakdown: HashMap<String, u32>,
    /// Usage by operation type
    pub operation_breakdown: HashMap<String, u32>,
    /// Average tokens per conversation
    pub avg_tokens_per_conversation: f64,
    /// Most expensive operations
    pub top_expensive_operations: Vec<TokenUsage>,
    /// Usage trend (tokens per hour for last 24h)
    pub hourly_trend: Vec<(DateTime<Utc>, u32)>,
}

/// Token pricing configuration per provider/model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPricing {
    /// Price per 1K input tokens in USD
    pub input_price_per_1k: f64,
    /// Price per 1K output tokens in USD
    pub output_price_per_1k: f64,
}

impl Default for TokenPricing {
    fn default() -> Self {
        Self {
            input_price_per_1k: 0.01,  // $0.01 per 1K input tokens
            output_price_per_1k: 0.03, // $0.03 per 1K output tokens
        }
    }
}

/// Comprehensive token management system
pub struct TokenManager {
    /// Usage history
    usage_history: RwLock<Vec<TokenUsage>>,
    /// Budget configuration
    budget: RwLock<TokenBudget>,
    /// Pricing configuration per provider/model
    pricing: RwLock<HashMap<String, TokenPricing>>,
    /// Current analytics cache
    analytics_cache: RwLock<Option<TokenAnalytics>>,
    /// Data storage path
    storage_path: std::path::PathBuf,
}

impl TokenManager {
    /// Create a new token manager
    pub fn new(storage_path: std::path::PathBuf) -> Self {
        let mut pricing = HashMap::new();
        
        // Default pricing for common providers
        pricing.insert("openai/gpt-4".to_string(), TokenPricing { 
            input_price_per_1k: 0.03, 
            output_price_per_1k: 0.06 
        });
        pricing.insert("openai/gpt-3.5-turbo".to_string(), TokenPricing { 
            input_price_per_1k: 0.001, 
            output_price_per_1k: 0.002 
        });
        pricing.insert("anthropic/claude-3-opus".to_string(), TokenPricing { 
            input_price_per_1k: 0.015, 
            output_price_per_1k: 0.075 
        });
        pricing.insert("anthropic/claude-3-sonnet".to_string(), TokenPricing { 
            input_price_per_1k: 0.003, 
            output_price_per_1k: 0.015 
        });
        pricing.insert("google/gemini-pro".to_string(), TokenPricing { 
            input_price_per_1k: 0.00025, 
            output_price_per_1k: 0.0005 
        });

        Self {
            usage_history: RwLock::new(Vec::new()),
            budget: RwLock::new(TokenBudget::default()),
            pricing: RwLock::new(pricing),
            analytics_cache: RwLock::new(None),
            storage_path,
        }
    }

    /// Record token usage for an operation
    pub async fn record_usage(&self, mut usage: TokenUsage) -> Result<()> {
        // Calculate estimated cost
        let pricing_key = format!("{}/{}", usage.provider, usage.model);
        let pricing = self.pricing.read().await;
        
        if let Some(price_config) = pricing.get(&pricing_key) {
            let input_cost = (usage.input_tokens as f64 / 1000.0) * price_config.input_price_per_1k;
            let output_cost = (usage.output_tokens as f64 / 1000.0) * price_config.output_price_per_1k;
            usage.estimated_cost = Some(input_cost + output_cost);
        }
        drop(pricing);

        // Record usage
        let mut history = self.usage_history.write().await;
        history.push(usage.clone());
        
        // Clear analytics cache to force recalculation
        *self.analytics_cache.write().await = None;

        // Check budget limits
        self.check_budget_limits().await?;

        // Persist to storage
        self.save_to_storage().await?;

        info!(
            "Recorded token usage: {} input, {} output, {} total tokens for {} (${:.4})",
            usage.input_tokens,
            usage.output_tokens, 
            usage.total_tokens,
            usage.operation_type,
            usage.estimated_cost.unwrap_or(0.0)
        );

        Ok(())
    }

    /// Get current budget configuration
    pub async fn get_budget(&self) -> TokenBudget {
        self.budget.read().await.clone()
    }

    /// Update budget configuration
    pub async fn update_budget(&self, budget: TokenBudget) -> Result<()> {
        *self.budget.write().await = budget;
        self.save_to_storage().await?;
        info!("Updated token budget configuration");
        Ok(())
    }

    /// Get comprehensive token analytics
    pub async fn get_analytics(&self) -> Result<TokenAnalytics> {
        // Check if we have cached analytics
        if let Some(cached) = self.analytics_cache.read().await.as_ref() {
            return Ok(cached.clone());
        }

        // Calculate fresh analytics
        let analytics = self.calculate_analytics().await?;
        
        // Cache the result
        *self.analytics_cache.write().await = Some(analytics.clone());
        
        Ok(analytics)
    }

    /// Check if current usage is within budget limits
    pub async fn check_budget_status(&self) -> Result<BudgetStatus> {
        let analytics = self.get_analytics().await?;
        let budget = self.budget.read().await;

        let mut warnings = Vec::new();
        let mut exceeded = Vec::new();

        // Check daily token limit
        if let Some(daily_limit) = budget.daily_limit {
            let usage_percent = analytics.daily_tokens as f64 / daily_limit as f64;
            if usage_percent >= 1.0 {
                exceeded.push(format!("Daily token limit exceeded: {} / {}", analytics.daily_tokens, daily_limit));
            } else if usage_percent >= budget.warning_threshold {
                warnings.push(format!("Daily token warning: {}% used ({} / {})", 
                    (usage_percent * 100.0) as u32, analytics.daily_tokens, daily_limit));
            }
        }

        // Check monthly token limit
        if let Some(monthly_limit) = budget.monthly_limit {
            let usage_percent = analytics.monthly_tokens as f64 / monthly_limit as f64;
            if usage_percent >= 1.0 {
                exceeded.push(format!("Monthly token limit exceeded: {} / {}", analytics.monthly_tokens, monthly_limit));
            } else if usage_percent >= budget.warning_threshold {
                warnings.push(format!("Monthly token warning: {}% used ({} / {})", 
                    (usage_percent * 100.0) as u32, analytics.monthly_tokens, monthly_limit));
            }
        }

        // Check daily cost limit
        if let Some(daily_cost_limit) = budget.daily_cost_limit {
            let usage_percent = analytics.daily_cost / daily_cost_limit;
            if usage_percent >= 1.0 {
                exceeded.push(format!("Daily cost limit exceeded: ${:.2} / ${:.2}", analytics.daily_cost, daily_cost_limit));
            } else if usage_percent >= budget.warning_threshold {
                warnings.push(format!("Daily cost warning: {}% used (${:.2} / ${:.2})", 
                    (usage_percent * 100.0) as u32, analytics.daily_cost, daily_cost_limit));
            }
        }

        // Check monthly cost limit
        if let Some(monthly_cost_limit) = budget.monthly_cost_limit {
            let usage_percent = analytics.monthly_cost / monthly_cost_limit;
            if usage_percent >= 1.0 {
                exceeded.push(format!("Monthly cost limit exceeded: ${:.2} / ${:.2}", analytics.monthly_cost, monthly_cost_limit));
            } else if usage_percent >= budget.warning_threshold {
                warnings.push(format!("Monthly cost warning: {}% used (${:.2} / ${:.2})", 
                    (usage_percent * 100.0) as u32, analytics.monthly_cost, monthly_cost_limit));
            }
        }

        Ok(BudgetStatus {
            within_limits: exceeded.is_empty(),
            warnings,
            exceeded: exceeded.clone(),
            should_auto_summarize: !exceeded.is_empty() && budget.auto_summarize_on_limit,
        })
    }

    /// Get usage history with optional filtering
    pub async fn get_usage_history(&self, filter: Option<UsageFilter>) -> Result<Vec<TokenUsage>> {
        let history = self.usage_history.read().await;
        
        if let Some(filter) = filter {
            Ok(history.iter()
                .filter(|usage| filter.matches(usage))
                .cloned()
                .collect())
        } else {
            Ok(history.clone())
        }
    }

    /// Clear usage history (with optional date range)
    pub async fn clear_history(&self, before_date: Option<DateTime<Utc>>) -> Result<()> {
        let mut history = self.usage_history.write().await;
        
        if let Some(before) = before_date {
            history.retain(|usage| usage.timestamp >= before);
            info!("Cleared token usage history before {}", before);
        } else {
            history.clear();
            info!("Cleared all token usage history");
        }

        // Clear analytics cache
        *self.analytics_cache.write().await = None;
        
        self.save_to_storage().await?;
        Ok(())
    }

    /// Export usage data to various formats
    pub async fn export_usage(&self, format: ExportFormat, path: &std::path::Path) -> Result<()> {
        let history = self.usage_history.read().await;
        
        match format {
            ExportFormat::Json => {
                let json = serde_json::to_string_pretty(&*history)?;
                tokio::fs::write(path, json).await?;
            },
            ExportFormat::Csv => {
                let mut csv = String::from("timestamp,provider,model,operation_type,input_tokens,output_tokens,total_tokens,estimated_cost,session_id,user_id\n");
                for usage in history.iter() {
                    csv.push_str(&format!(
                        "{},{},{},{},{},{},{},{},{},{}\n",
                        usage.timestamp.to_rfc3339(),
                        usage.provider,
                        usage.model,
                        usage.operation_type,
                        usage.input_tokens,
                        usage.output_tokens,
                        usage.total_tokens,
                        usage.estimated_cost.unwrap_or(0.0),
                        usage.session_id,
                        usage.user_id
                    ));
                }
                tokio::fs::write(path, csv).await?;
            },
        }

        info!("Exported {} usage records to {:?}", history.len(), path);
        Ok(())
    }

    // Private helper methods
    
    async fn calculate_analytics(&self) -> Result<TokenAnalytics> {
        let history = self.usage_history.read().await;
        let now = Utc::now();
        
        // Filter for today and this month
        let today_start = now.date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc();
        let month_start = now.date_naive().with_day(1).unwrap().and_hms_opt(0, 0, 0).unwrap().and_utc();
        
        let daily_usage: Vec<_> = history.iter()
            .filter(|usage| usage.timestamp >= today_start)
            .collect();
        
        let monthly_usage: Vec<_> = history.iter()
            .filter(|usage| usage.timestamp >= month_start)
            .collect();

        // Calculate basic stats
        let daily_tokens: u32 = daily_usage.iter().map(|u| u.total_tokens).sum();
        let monthly_tokens: u32 = monthly_usage.iter().map(|u| u.total_tokens).sum();
        let daily_cost: f64 = daily_usage.iter().map(|u| u.estimated_cost.unwrap_or(0.0)).sum();
        let monthly_cost: f64 = monthly_usage.iter().map(|u| u.estimated_cost.unwrap_or(0.0)).sum();

        // Provider breakdown
        let mut provider_breakdown = HashMap::new();
        for usage in monthly_usage.iter() {
            *provider_breakdown.entry(usage.provider.clone()).or_insert(0) += usage.total_tokens;
        }

        // Operation breakdown
        let mut operation_breakdown = HashMap::new();
        for usage in monthly_usage.iter() {
            *operation_breakdown.entry(usage.operation_type.clone()).or_insert(0) += usage.total_tokens;
        }

        // Average tokens per conversation
        let conversations: std::collections::HashSet<_> = monthly_usage.iter()
            .map(|u| &u.session_id)
            .collect();
        let avg_tokens_per_conversation = if conversations.is_empty() {
            0.0
        } else {
            monthly_tokens as f64 / conversations.len() as f64
        };

        // Top expensive operations (last 30 days)
        let mut expensive_ops = monthly_usage.clone();
        expensive_ops.sort_by(|a, b| {
            b.estimated_cost.unwrap_or(0.0).partial_cmp(&a.estimated_cost.unwrap_or(0.0)).unwrap()
        });
        let top_expensive_operations = expensive_ops.into_iter()
            .take(10)
            .cloned()
            .collect();

        // Hourly trend for last 24 hours
        let mut hourly_trend = Vec::new();
        for hour in 0..24 {
            let hour_start = today_start - chrono::Duration::hours(24 - hour);
            let hour_end = hour_start + chrono::Duration::hours(1);
            
            let hour_tokens: u32 = history.iter()
                .filter(|u| u.timestamp >= hour_start && u.timestamp < hour_end)
                .map(|u| u.total_tokens)
                .sum();
            
            hourly_trend.push((hour_start, hour_tokens));
        }

        Ok(TokenAnalytics {
            daily_tokens,
            monthly_tokens,
            daily_cost,
            monthly_cost,
            provider_breakdown,
            operation_breakdown,
            avg_tokens_per_conversation,
            top_expensive_operations,
            hourly_trend,
        })
    }

    async fn check_budget_limits(&self) -> Result<()> {
        let status = self.check_budget_status().await?;
        
        for warning in &status.warnings {
            warn!("Token budget warning: {}", warning);
        }
        
        for exceeded in &status.exceeded {
            warn!("Token budget exceeded: {}", exceeded);
        }

        Ok(())
    }

    async fn save_to_storage(&self) -> Result<()> {
        // Create storage directory if it doesn't exist
        if let Some(parent) = self.storage_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Save token usage history
        let history = self.usage_history.read().await;
        let budget = self.budget.read().await;
        let pricing = self.pricing.read().await;

        let storage_data = TokenStorageData {
            usage_history: history.clone(),
            budget: budget.clone(),
            pricing: pricing.clone(),
        };

        let json = serde_json::to_string_pretty(&storage_data)?;
        tokio::fs::write(&self.storage_path, json).await?;
        
        Ok(())
    }

    /// Load token manager from storage
    pub async fn load_from_storage(storage_path: std::path::PathBuf) -> Result<Self> {
        let manager = Self::new(storage_path.clone());
        
        if storage_path.exists() {
            let json = tokio::fs::read_to_string(&storage_path).await?;
            let storage_data: TokenStorageData = serde_json::from_str(&json)?;
            
            let usage_history_len = storage_data.usage_history.len();
            *manager.usage_history.write().await = storage_data.usage_history;
            *manager.budget.write().await = storage_data.budget;
            *manager.pricing.write().await = storage_data.pricing;
            
            info!("Loaded token manager from storage with {} usage records", usage_history_len);
        }
        
        Ok(manager)
    }
}

/// Budget status information
#[derive(Debug, Clone)]
pub struct BudgetStatus {
    pub within_limits: bool,
    pub warnings: Vec<String>,
    pub exceeded: Vec<String>,
    pub should_auto_summarize: bool,
}

/// Filter for usage history queries
#[derive(Debug, Clone)]
pub struct UsageFilter {
    pub provider: Option<String>,
    pub model: Option<String>,
    pub operation_type: Option<String>,
    pub session_id: Option<String>,
    pub user_id: Option<String>,
    pub date_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
    pub min_tokens: Option<u32>,
    pub max_tokens: Option<u32>,
}

impl UsageFilter {
    pub fn matches(&self, usage: &TokenUsage) -> bool {
        if let Some(ref provider) = self.provider {
            if &usage.provider != provider {
                return false;
            }
        }
        
        if let Some(ref model) = self.model {
            if &usage.model != model {
                return false;
            }
        }
        
        if let Some(ref operation_type) = self.operation_type {
            if &usage.operation_type != operation_type {
                return false;
            }
        }
        
        if let Some(ref session_id) = self.session_id {
            if &usage.session_id != session_id {
                return false;
            }
        }
        
        if let Some(ref user_id) = self.user_id {
            if &usage.user_id != user_id {
                return false;
            }
        }
        
        if let Some((start, end)) = self.date_range {
            if usage.timestamp < start || usage.timestamp > end {
                return false;
            }
        }
        
        if let Some(min_tokens) = self.min_tokens {
            if usage.total_tokens < min_tokens {
                return false;
            }
        }
        
        if let Some(max_tokens) = self.max_tokens {
            if usage.total_tokens > max_tokens {
                return false;
            }
        }
        
        true
    }
}

/// Export format options
#[derive(Debug, Clone)]
pub enum ExportFormat {
    Json,
    Csv,
}

/// Storage data structure
#[derive(Debug, Serialize, Deserialize)]
struct TokenStorageData {
    usage_history: Vec<TokenUsage>,
    budget: TokenBudget,
    pricing: HashMap<String, TokenPricing>,
}