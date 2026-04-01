// Copyright 2024-2026 Andrey Vasilevsky <anvanster@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Embedding generation for episodes

mod fastembed_impl;
mod traits;

pub use fastembed_impl::FastEmbedder;
pub use traits::Embedder;

/// Default embedding dimension for BGE-Small
pub const DEFAULT_DIMENSION: usize = 384;
