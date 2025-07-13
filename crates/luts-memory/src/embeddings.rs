//! Embedding generation and vector similarity for memory blocks
//!
//! This module provides embedding services for generating vector representations
//! of memory block content, enabling semantic search and similarity matching.

use luts_common::{LutsError, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Configuration for embedding services
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    /// Which embedding service to use
    pub provider: EmbeddingProvider,
    /// Model name (e.g., "text-embedding-3-small", "nomic-embed-text")
    pub model: String,
    /// API key if using external service
    pub api_key: Option<String>,
    /// Base URL for self-hosted services
    pub base_url: Option<String>,
    /// Maximum text length to embed (longer text will be truncated)
    pub max_text_length: usize,
    /// Dimensions of the embedding vectors
    pub dimensions: usize,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            provider: EmbeddingProvider::OpenAI,
            model: "text-embedding-3-small".to_string(),
            api_key: None,
            base_url: None,
            max_text_length: 8192,
            dimensions: 1536, // OpenAI text-embedding-3-small
        }
    }
}

/// Available embedding providers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EmbeddingProvider {
    /// OpenAI embedding API
    OpenAI,
    /// Local embedding service (e.g., sentence-transformers)
    Local,
    /// Ollama with embedding models
    Ollama,
    /// Mock provider for testing
    Mock,
}

/// A trait for embedding services that can generate vector representations
#[async_trait]
pub trait EmbeddingService: Send + Sync {
    /// Generate embeddings for a single text
    async fn embed_text(&self, text: &str) -> Result<Vec<f32>>;
    
    /// Generate embeddings for multiple texts (more efficient for batch processing)
    async fn embed_texts(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>;
    
    /// Get the dimensions of embeddings produced by this service
    fn dimensions(&self) -> usize;
    
    /// Get the maximum text length this service can handle
    fn max_text_length(&self) -> usize;
}

/// Vector similarity search configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorSearchConfig {
    /// Maximum number of results to return
    pub max_results: usize,
    /// Minimum similarity score threshold (0.0 to 1.0)
    pub min_relevance: f32,
    /// Similarity metric to use
    pub metric: SimilarityMetric,
}

impl Default for VectorSearchConfig {
    fn default() -> Self {
        Self {
            max_results: luts_common::vector_search::DEFAULT_MAX_RESULTS,
            min_relevance: luts_common::vector_search::DEFAULT_MIN_RELEVANCE,
            metric: SimilarityMetric::Cosine,
        }
    }
}

/// Available similarity metrics
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SimilarityMetric {
    /// Cosine similarity (most common)
    Cosine,
    /// Euclidean distance
    Euclidean,
    /// Dot product
    DotProduct,
}

/// Vector similarity operations
pub struct VectorSimilarity;

impl VectorSimilarity {
    /// Calculate cosine similarity between two vectors
    pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0;
        }
        
        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        
        if norm_a == 0.0 || norm_b == 0.0 {
            0.0
        } else {
            dot_product / (norm_a * norm_b)
        }
    }
    
    /// Calculate euclidean distance between two vectors
    pub fn euclidean_distance(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return f32::INFINITY;
        }
        
        a.iter()
            .zip(b.iter())
            .map(|(x, y)| (x - y).powi(2))
            .sum::<f32>()
            .sqrt()
    }
}

/// Factory for creating embedding services
pub struct EmbeddingServiceFactory;

impl EmbeddingServiceFactory {
    /// Create an embedding service from configuration
    pub fn create(config: EmbeddingConfig) -> Result<Box<dyn EmbeddingService>> {
        match config.provider {
            EmbeddingProvider::Mock => Ok(Box::new(MockEmbeddingService::new(config))),
            _ => Err(LutsError::Memory("Only mock embedding service is implemented in this phase".to_string())),
        }
    }
}

/// Mock embedding service for testing
pub struct MockEmbeddingService {
    config: EmbeddingConfig,
}

impl MockEmbeddingService {
    pub fn new(config: EmbeddingConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl EmbeddingService for MockEmbeddingService {
    async fn embed_text(&self, text: &str) -> Result<Vec<f32>> {
        // Generate a deterministic "embedding" based on text hash
        let hash = luts_common::string_hash(text);
        let mut embedding = vec![0.0; self.config.dimensions];
        
        // Fill with pseudo-random values based on hash
        for (i, value) in embedding.iter_mut().enumerate() {
            let seed = hash.wrapping_add(i as u32);
            *value = ((seed % 1000) as f32 - 500.0) / 500.0; // Range: -1.0 to 1.0
        }
        
        Ok(embedding)
    }
    
    async fn embed_texts(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let mut embeddings = Vec::new();
        for text in texts {
            embeddings.push(self.embed_text(text).await?);
        }
        Ok(embeddings)
    }
    
    fn dimensions(&self) -> usize {
        self.config.dimensions
    }
    
    fn max_text_length(&self) -> usize {
        self.config.max_text_length
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((VectorSimilarity::cosine_similarity(&a, &b) - 1.0).abs() < 0.001);
        
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        assert!((VectorSimilarity::cosine_similarity(&a, &b) - 0.0).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_mock_embedding_service() {
        let config = EmbeddingConfig {
            provider: EmbeddingProvider::Mock,
            dimensions: 384,
            ..Default::default()
        };
        
        let service = MockEmbeddingService::new(config);
        let embedding = service.embed_text("test text").await.unwrap();
        
        assert_eq!(embedding.len(), 384);
        assert!(embedding.iter().all(|&x| x >= -1.0 && x <= 1.0));
    }
}