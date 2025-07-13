//! LUTS Tools - AI tools collection
//!
//! This crate provides agent-independent AI tools including
//! calculator, web search, website scraping, and semantic search.

pub mod base;
pub mod calc;
pub mod search;
pub mod website;
pub mod semantic_search;

// Re-export key tools for convenience
pub use calc::MathTool;
pub use search::DDGSearchTool;
pub use website::WebsiteTool;
pub use semantic_search::SemanticSearchTool;
pub use base::AiTool;