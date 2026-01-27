//! Embedding generation for episodes

mod fastembed_impl;
mod traits;

pub use fastembed_impl::FastEmbedder;
pub use traits::Embedder;

/// Default embedding dimension for BGE-Small
pub const DEFAULT_DIMENSION: usize = 384;
