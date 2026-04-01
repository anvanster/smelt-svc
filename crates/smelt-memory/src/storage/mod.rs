// Copyright 2024-2026 Andrey Vasilevsky <anvanster@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Storage layer for episodes and embeddings

mod sqlite;
mod vectors;

pub use sqlite::{EpisodeStorage, MemoryStats};
pub use vectors::VectorStore;
