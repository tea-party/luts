//! Token pricing configuration and calculations

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::constants::{models, providers};

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

impl TokenPricing {
    /// Calculate cost for given token usage
    pub fn calculate_cost(&self, input_tokens: u32, output_tokens: u32) -> f64 {
        let input_cost = (input_tokens as f64 / 1000.0) * self.input_price_per_1k;
        let output_cost = (output_tokens as f64 / 1000.0) * self.output_price_per_1k;
        input_cost + output_cost
    }
}

/// Comprehensive pricing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingConfig {
    /// Pricing per provider/model combination
    pub pricing: HashMap<String, TokenPricing>,
}

impl Default for PricingConfig {
    fn default() -> Self {
        let mut pricing = HashMap::new();
        
        // OpenAI pricing (as of 2024)
        pricing.insert(format!("{}/{}", providers::OPENAI, models::GPT_4), TokenPricing {
            input_price_per_1k: 0.03,
            output_price_per_1k: 0.06,
        });
        pricing.insert(format!("{}/{}", providers::OPENAI, models::GPT_4_TURBO), TokenPricing {
            input_price_per_1k: 0.01,
            output_price_per_1k: 0.03,
        });
        pricing.insert(format!("{}/{}", providers::OPENAI, models::GPT_3_5_TURBO), TokenPricing {
            input_price_per_1k: 0.001,
            output_price_per_1k: 0.002,
        });
        
        // Anthropic pricing
        pricing.insert(format!("{}/{}", providers::ANTHROPIC, models::CLAUDE_3_OPUS), TokenPricing {
            input_price_per_1k: 0.015,
            output_price_per_1k: 0.075,
        });
        pricing.insert(format!("{}/{}", providers::ANTHROPIC, models::CLAUDE_3_SONNET), TokenPricing {
            input_price_per_1k: 0.003,
            output_price_per_1k: 0.015,
        });
        pricing.insert(format!("{}/{}", providers::ANTHROPIC, models::CLAUDE_3_HAIKU), TokenPricing {
            input_price_per_1k: 0.00025,
            output_price_per_1k: 0.00125,
        });
        
        // Google pricing
        pricing.insert(format!("{}/{}", providers::GOOGLE, models::GEMINI_PRO), TokenPricing {
            input_price_per_1k: 0.00025,
            output_price_per_1k: 0.0005,
        });
        
        Self { pricing }
    }
}

impl PricingConfig {
    /// Get pricing for a specific provider/model combination
    pub fn get_pricing(&self, provider: &str, model: &str) -> Option<&TokenPricing> {
        let key = format!("{}/{}", provider, model);
        self.pricing.get(&key)
    }
    
    /// Calculate cost for a provider/model combination
    pub fn calculate_cost(&self, provider: &str, model: &str, input_tokens: u32, output_tokens: u32) -> Option<f64> {
        self.get_pricing(provider, model)
            .map(|pricing| pricing.calculate_cost(input_tokens, output_tokens))
    }
    
    /// Add or update pricing for a provider/model
    pub fn set_pricing(&mut self, provider: &str, model: &str, pricing: TokenPricing) {
        let key = format!("{}/{}", provider, model);
        self.pricing.insert(key, pricing);
    }
}