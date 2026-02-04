//! FastEmbed-based embedding implementation

use super::traits::Embedder;
use super::DEFAULT_DIMENSION;
use crate::error::{MemoryError, MemoryResult};
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use std::sync::Arc;

/// FastEmbed-based embedder using BGE-Small model
pub struct FastEmbedder {
    model: Arc<TextEmbedding>,
    dimension: usize,
}

impl FastEmbedder {
    /// Create a new FastEmbedder with the default BGE-Small model
    pub fn new() -> MemoryResult<Self> {
        Self::with_model(EmbeddingModel::BGESmallENV15)
    }

    /// Create a FastEmbedder with a specific model
    pub fn with_model(model: EmbeddingModel) -> MemoryResult<Self> {
        // Set cache directory to ~/.smelt/fastembed_cache to avoid polluting project directories
        if std::env::var("FASTEMBED_CACHE_PATH").is_err() {
            if let Some(home) = dirs::home_dir() {
                let cache_dir = home.join(".smelt").join("fastembed_cache");
                let _ = std::fs::create_dir_all(&cache_dir);
                unsafe { std::env::set_var("FASTEMBED_CACHE_PATH", &cache_dir) };
            }
        }

        let embedding =
            TextEmbedding::try_new(InitOptions::new(model).with_show_download_progress(true))
                .map_err(|e| {
                    MemoryError::Embedding(format!("Failed to initialize embedding model: {}", e))
                })?;

        // Get dimension from first test embedding
        let dimension = match embedding.embed(vec!["test"], None) {
            Ok(embeddings) if !embeddings.is_empty() => embeddings[0].len(),
            _ => DEFAULT_DIMENSION,
        };

        Ok(Self {
            model: Arc::new(embedding),
            dimension,
        })
    }

    /// Create a dummy embedder for testing (returns random vectors)
    #[cfg(test)]
    pub fn dummy() -> Self {
        Self {
            model: Arc::new(
                TextEmbedding::try_new(InitOptions::new(EmbeddingModel::BGESmallENV15))
                    .expect("Failed to create test model"),
            ),
            dimension: DEFAULT_DIMENSION,
        }
    }
}

impl Embedder for FastEmbedder {
    fn dimension(&self) -> usize {
        self.dimension
    }

    fn embed(&self, text: &str) -> MemoryResult<Vec<f32>> {
        let embeddings = self
            .model
            .embed(vec![text], None)
            .map_err(|e| MemoryError::Embedding(format!("Embedding failed: {}", e)))?;

        embeddings
            .into_iter()
            .next()
            .ok_or_else(|| MemoryError::Embedding("No embedding generated".to_string()))
    }

    fn embed_batch(&self, texts: &[&str]) -> MemoryResult<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let texts_owned: Vec<String> = texts.iter().map(|s| s.to_string()).collect();
        let texts_refs: Vec<&str> = texts_owned.iter().map(|s| s.as_str()).collect();

        self.model
            .embed(texts_refs, None)
            .map_err(|e| MemoryError::Embedding(format!("Batch embedding failed: {}", e)))
    }
}

impl Default for FastEmbedder {
    fn default() -> Self {
        Self::new().expect("Failed to create default FastEmbedder")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests require downloading the model on first run
    // They are marked as ignored for CI but can be run manually

    #[test]
    #[ignore = "Requires model download"]
    fn test_embed_single() {
        let embedder = FastEmbedder::new().unwrap();
        let embedding = embedder.embed("Hello, world!").unwrap();

        assert_eq!(embedding.len(), embedder.dimension());
        // Check that values are reasonable (normalized)
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.1); // Should be roughly normalized
    }

    #[test]
    #[ignore = "Requires model download"]
    fn test_embed_batch() {
        let embedder = FastEmbedder::new().unwrap();
        let embeddings = embedder
            .embed_batch(&["First text", "Second text", "Third text"])
            .unwrap();

        assert_eq!(embeddings.len(), 3);
        for emb in &embeddings {
            assert_eq!(emb.len(), embedder.dimension());
        }
    }

    #[test]
    #[ignore = "Requires model download"]
    fn test_similar_texts() {
        let embedder = FastEmbedder::new().unwrap();

        let e1 = embedder.embed("Fix authentication bug in login").unwrap();
        let e2 = embedder.embed("Repair auth issue in sign-in").unwrap();
        let e3 = embedder.embed("Add new database migration").unwrap();

        // Similar texts should have higher cosine similarity
        let sim_12 = cosine_sim(&e1, &e2);
        let sim_13 = cosine_sim(&e1, &e3);

        assert!(
            sim_12 > sim_13,
            "Similar texts should have higher similarity"
        );
    }

    #[cfg(test)]
    fn cosine_sim(a: &[f32], b: &[f32]) -> f32 {
        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        dot / (norm_a * norm_b)
    }
}
