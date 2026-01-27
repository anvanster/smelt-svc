//! Embedder trait definition

use crate::error::MemoryResult;

/// Trait for generating text embeddings
pub trait Embedder: Send + Sync {
    /// Get the embedding dimension
    fn dimension(&self) -> usize;

    /// Generate an embedding for a single text
    fn embed(&self, text: &str) -> MemoryResult<Vec<f32>>;

    /// Generate embeddings for multiple texts (may be more efficient)
    fn embed_batch(&self, texts: &[&str]) -> MemoryResult<Vec<Vec<f32>>> {
        texts.iter().map(|t| self.embed(t)).collect()
    }
}
