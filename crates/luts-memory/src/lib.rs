//! LUTS Memory - Memory architecture and management
//!
//! This crate provides the core memory management system for LUTS,
//! including memory blocks, embeddings, context management, and storage providers.

pub mod block;
pub mod embeddings;
pub mod storage;
pub mod types;
pub mod utils;

// Re-export commonly used types
pub use block::{MemoryBlock, MemoryBlockBuilder, MemoryBlockMetadata};
pub use embeddings::{
    EmbeddingConfig, EmbeddingProvider, EmbeddingService, EmbeddingServiceFactory,
    VectorSearchConfig, VectorSimilarity, SimilarityMetric
};
pub use storage::{
    MemoryStore, MemoryManager, MemoryQuery, MemoryStats, QuerySort, VectorQuery,
    SurrealMemoryStore, SurrealConfig, AuthConfig, RelationType
};
pub use types::{BlockId, BlockType, MemoryContent, Relevance, TimeRange};
pub use utils::BlockUtils;

// Re-export from luts-common for convenience
pub use luts_common::{LutsError, Result};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_exports() {
        // Test that all major types are accessible
        let _block_id = BlockId::generate();
        let _block_type = BlockType::Message;
        let _content = MemoryContent::Text("test".to_string());
        let _relevance = Relevance::new(0.5);
        let _time_range = TimeRange::last_days(1);
        
        // Test that config types are accessible
        let _embedding_config = EmbeddingConfig::default();
        let _vector_config = VectorSearchConfig::default();
        let _surreal_config = SurrealConfig::default();
    }
}