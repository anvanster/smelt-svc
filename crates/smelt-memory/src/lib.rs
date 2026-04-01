// Copyright 2024-2026 Andrey Vasilevsky <anvanster@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Smelt Memory - Contextual memory system for semantic version control
//!
//! This crate provides episodic memory capabilities inspired by MemRL:
//! - Episode capture from intent + outcome
//! - Semantic search via embeddings
//! - Utility-based ranking (Wilson score, Bellman propagation, decay)
//! - Feedback learning for retrieval improvement

pub mod embedder;
pub mod error;
pub mod storage;
pub mod types;
pub mod utility;

mod memory;

pub use error::{MemoryError, MemoryResult};
pub use memory::SmeltMemory;
pub use types::{Episode, EpisodeOutcome, Feedback, RankedEpisode};
