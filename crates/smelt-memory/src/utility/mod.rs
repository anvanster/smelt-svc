// Copyright 2024-2026 Andrey Vasilevsky <anvanster@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Utility calculation algorithms for episode ranking
//!
//! Ported from MemRL with adaptations for Smelt's needs.

mod bellman;
mod decay;
mod ranker;
mod wilson;

pub use bellman::{bellman_propagate, temporal_credit_assignment, PropagationResult};
pub use decay::{apply_decay, DecayParams};
pub use ranker::UtilityRanker;
pub use wilson::wilson_score;
