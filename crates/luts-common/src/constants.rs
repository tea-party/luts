//! Common constants used across LUTS

/// Default maximum memory blocks to retrieve in a query
pub const DEFAULT_MAX_BLOCKS: usize = 100;

/// Default context window size in tokens
pub const DEFAULT_CONTEXT_TOKENS: usize = 8000;

/// Default embedding dimension for OpenAI text-embedding-ada-002
pub const DEFAULT_EMBEDDING_DIM: usize = 1536;

/// Default token budget limits
pub const DEFAULT_DAILY_TOKEN_LIMIT: u32 = 100_000;
pub const DEFAULT_MONTHLY_TOKEN_LIMIT: u32 = 2_000_000;

/// Default cost budget limits
pub const DEFAULT_DAILY_COST_LIMIT: f64 = 50.0;
pub const DEFAULT_MONTHLY_COST_LIMIT: f64 = 1000.0;

/// Default warning threshold (percentage)
pub const DEFAULT_WARNING_THRESHOLD: f64 = 0.8;

/// Default file size limits
pub const MAX_EXPORT_FILE_SIZE: usize = 100 * 1024 * 1024; // 100MB
pub const MAX_IMPORT_FILE_SIZE: usize = 50 * 1024 * 1024;  // 50MB

/// Default pagination limits
pub const DEFAULT_PAGE_SIZE: usize = 50;
pub const MAX_PAGE_SIZE: usize = 1000;

/// Common model identifiers
pub mod models {
    // OpenAI models
    pub const GPT_4: &str = "gpt-4";
    pub const GPT_4_TURBO: &str = "gpt-4-turbo";
    pub const GPT_3_5_TURBO: &str = "gpt-3.5-turbo";
    
    // Anthropic models
    pub const CLAUDE_3_OPUS: &str = "claude-3-opus";
    pub const CLAUDE_3_SONNET: &str = "claude-3-sonnet";
    pub const CLAUDE_3_HAIKU: &str = "claude-3-haiku";
    
    // Google models
    pub const GEMINI_PRO: &str = "gemini-pro";
    pub const GEMINI_PRO_VISION: &str = "gemini-pro-vision";
}

/// Common provider identifiers
pub mod providers {
    pub const OPENAI: &str = "openai";
    pub const ANTHROPIC: &str = "anthropic";
    pub const GOOGLE: &str = "google";
    pub const AZURE: &str = "azure";
}

/// Default timeout values in seconds
pub mod timeouts {
    pub const DEFAULT_HTTP_TIMEOUT: u64 = 30;
    pub const DEFAULT_LLM_TIMEOUT: u64 = 120;
    pub const DEFAULT_TOOL_TIMEOUT: u64 = 60;
    pub const DEFAULT_EMBEDDING_TIMEOUT: u64 = 30;
}

/// Vector search defaults
pub mod vector_search {
    pub const DEFAULT_MAX_RESULTS: usize = 10;
    pub const DEFAULT_MIN_RELEVANCE: f32 = 0.7;
    pub const DEFAULT_EMBEDDING_BATCH_SIZE: usize = 100;
}