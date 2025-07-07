//! Embedding generation and vector similarity for memory blocks
//!
//! This module provides embedding services for generating vector representations
//! of memory block content, enabling semantic search and similarity matching.

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, warn};

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

/// OpenAI embedding service implementation
pub struct OpenAIEmbeddingService {
    config: EmbeddingConfig,
    client: reqwest::Client,
}

impl OpenAIEmbeddingService {
    pub fn new(config: EmbeddingConfig) -> Result<Self> {
        if config.api_key.is_none() {
            return Err(anyhow!("OpenAI API key is required"));
        }
        
        let client = reqwest::Client::new();
        Ok(Self { config, client })
    }
}

#[async_trait]
impl EmbeddingService for OpenAIEmbeddingService {
    async fn embed_text(&self, text: &str) -> Result<Vec<f32>> {
        let texts = vec![text.to_string()];
        let embeddings = self.embed_texts(&texts).await?;
        embeddings.into_iter().next()
            .ok_or_else(|| anyhow!("No embedding returned from OpenAI"))
    }
    
    async fn embed_texts(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let api_key = self.config.api_key.as_ref()
            .ok_or_else(|| anyhow!("OpenAI API key not configured"))?;
        
        // Truncate texts that are too long
        let truncated_texts: Vec<String> = texts.iter()
            .map(|text| {
                if text.len() > self.config.max_text_length {
                    warn!("Truncating text from {} to {} characters", text.len(), self.config.max_text_length);
                    text.chars().take(self.config.max_text_length).collect()
                } else {
                    text.clone()
                }
            })
            .collect();
        
        #[derive(Serialize)]
        struct EmbeddingRequest {
            input: Vec<String>,
            model: String,
            encoding_format: String,
        }
        
        #[derive(Deserialize)]
        struct EmbeddingResponse {
            data: Vec<EmbeddingData>,
        }
        
        #[derive(Deserialize)]
        struct EmbeddingData {
            embedding: Vec<f32>,
        }
        
        let request = EmbeddingRequest {
            input: truncated_texts,
            model: self.config.model.clone(),
            encoding_format: "float".to_string(),
        };
        
        let response = self.client
            .post("https://api.openai.com/v1/embeddings")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("OpenAI API error: {}", error_text));
        }
        
        let embedding_response: EmbeddingResponse = response.json().await?;
        let embeddings = embedding_response.data.into_iter()
            .map(|data| data.embedding)
            .collect();
        
        debug!("Generated {} embeddings using OpenAI", texts.len());
        Ok(embeddings)
    }
    
    fn dimensions(&self) -> usize {
        self.config.dimensions
    }
    
    fn max_text_length(&self) -> usize {
        self.config.max_text_length
    }
}

/// Local embedding service (placeholder for local models)
pub struct LocalEmbeddingService {
    config: EmbeddingConfig,
}

impl LocalEmbeddingService {
    pub fn new(config: EmbeddingConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl EmbeddingService for LocalEmbeddingService {
    async fn embed_text(&self, _text: &str) -> Result<Vec<f32>> {
        // In a real implementation, this would:
        // 1. Load a local sentence transformer model
        // 2. Tokenize and embed the text
        // 3. Return the embedding vector
        
        // For now, return a mock embedding
        warn!("Local embedding service not yet implemented, returning mock embedding");
        let mut embedding = vec![0.0; self.config.dimensions];
        // Simple hash-based mock embedding
        let hash = std::collections::hash_map::DefaultHasher::new();
        for (i, value) in embedding.iter_mut().enumerate() {
            *value = ((i * 7) % 100) as f32 / 100.0;
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
        // Generate deterministic mock embeddings based on text content
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        let hash = hasher.finish();
        
        let mut embedding = vec![0.0; self.config.dimensions];
        for (i, value) in embedding.iter_mut().enumerate() {
            let seed = hash.wrapping_add(i as u64);
            *value = ((seed % 1000) as f32 / 1000.0) * 2.0 - 1.0; // Range [-1, 1]
        }
        
        // Normalize the vector
        let magnitude: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if magnitude > 0.0 {
            for value in &mut embedding {
                *value /= magnitude;
            }
        }
        
        debug!("Generated mock embedding with {} dimensions for text: '{}'", 
               self.config.dimensions, 
               if text.len() > 50 { &text[..50] } else { text });
        
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

/// Factory for creating embedding services
pub struct EmbeddingServiceFactory;

impl EmbeddingServiceFactory {
    pub fn create(config: EmbeddingConfig) -> Result<Arc<dyn EmbeddingService>> {
        let service: Arc<dyn EmbeddingService> = match config.provider {
            EmbeddingProvider::OpenAI => {
                Arc::new(OpenAIEmbeddingService::new(config)?)
            }
            EmbeddingProvider::Local => {
                Arc::new(LocalEmbeddingService::new(config))
            }
            EmbeddingProvider::Ollama => {
                // TODO: Implement Ollama embedding service
                warn!("Ollama embedding service not yet implemented, using mock");
                Arc::new(MockEmbeddingService::new(config))
            }
            EmbeddingProvider::Mock => {
                Arc::new(MockEmbeddingService::new(config))
            }
        };
        
        Ok(service)
    }
}

/// Vector similarity utilities
pub struct VectorSimilarity;

impl VectorSimilarity {
    /// Calculate cosine similarity between two vectors
    pub fn cosine_similarity(a: &[f32], b: &[f32]) -> Result<f32> {
        if a.len() != b.len() {
            return Err(anyhow!("Vector dimensions don't match: {} vs {}", a.len(), b.len()));
        }
        
        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let magnitude_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let magnitude_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        
        if magnitude_a == 0.0 || magnitude_b == 0.0 {
            return Ok(0.0);
        }
        
        Ok(dot_product / (magnitude_a * magnitude_b))
    }
    
    /// Calculate Euclidean distance between two vectors
    pub fn euclidean_distance(a: &[f32], b: &[f32]) -> Result<f32> {
        if a.len() != b.len() {
            return Err(anyhow!("Vector dimensions don't match: {} vs {}", a.len(), b.len()));
        }
        
        let distance: f32 = a.iter()
            .zip(b.iter())
            .map(|(x, y)| (x - y).powi(2))
            .sum::<f32>()
            .sqrt();
        
        Ok(distance)
    }
    
    /// Calculate dot product similarity between two vectors
    pub fn dot_product(a: &[f32], b: &[f32]) -> Result<f32> {
        if a.len() != b.len() {
            return Err(anyhow!("Vector dimensions don't match: {} vs {}", a.len(), b.len()));
        }
        
        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        Ok(dot_product)
    }
}

/// Vector search configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorSearchConfig {
    /// Similarity threshold (0.0 to 1.0)
    pub similarity_threshold: f32,
    /// Maximum number of results to return
    pub max_results: usize,
    /// Similarity metric to use
    pub similarity_metric: SimilarityMetric,
}

impl Default for VectorSearchConfig {
    fn default() -> Self {
        Self {
            similarity_threshold: 0.7,
            max_results: 10,
            similarity_metric: SimilarityMetric::Cosine,
        }
    }
}

/// Available similarity metrics
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SimilarityMetric {
    /// Cosine similarity (recommended for text embeddings)
    Cosine,
    /// Euclidean distance (lower is more similar)
    Euclidean,
    /// Dot product similarity
    DotProduct,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_mock_embedding_service() {
        let config = EmbeddingConfig {
            provider: EmbeddingProvider::Mock,
            dimensions: 384,
            ..Default::default()
        };
        
        let service = MockEmbeddingService::new(config);
        
        let text = "This is a test sentence for embedding.";
        let embedding = service.embed_text(text).await.unwrap();
        
        assert_eq!(embedding.len(), 384);
        
        // Test that the same text produces the same embedding
        let embedding2 = service.embed_text(text).await.unwrap();
        assert_eq!(embedding, embedding2);
        
        // Test that different text produces different embeddings
        let different_text = "This is a completely different sentence.";
        let different_embedding = service.embed_text(different_text).await.unwrap();
        assert_ne!(embedding, different_embedding);
    }
    
    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        let c = vec![1.0, 0.0, 0.0];
        
        // Orthogonal vectors should have 0 similarity
        assert!((VectorSimilarity::cosine_similarity(&a, &b).unwrap() - 0.0).abs() < 1e-6);
        
        // Identical vectors should have 1.0 similarity
        assert!((VectorSimilarity::cosine_similarity(&a, &c).unwrap() - 1.0).abs() < 1e-6);
    }
    
    #[tokio::test]
    async fn test_batch_embedding() {
        let config = EmbeddingConfig {
            provider: EmbeddingProvider::Mock,
            dimensions: 128,
            ..Default::default()
        };
        
        let service = MockEmbeddingService::new(config);
        
        let texts = vec![
            "First sentence.".to_string(),
            "Second sentence.".to_string(),
            "Third sentence.".to_string(),
        ];
        
        let embeddings = service.embed_texts(&texts).await.unwrap();
        
        assert_eq!(embeddings.len(), 3);
        for embedding in &embeddings {
            assert_eq!(embedding.len(), 128);
        }
        
        // Each embedding should be different
        assert_ne!(embeddings[0], embeddings[1]);
        assert_ne!(embeddings[1], embeddings[2]);
        assert_ne!(embeddings[0], embeddings[2]);
    }
}