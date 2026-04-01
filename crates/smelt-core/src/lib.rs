// Copyright 2024-2026 Andrey Vasilevsky <anvanster@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Smelt Core - Semantic version control library
//!
//! This crate provides the core functionality for Smelt, including:
//! - Intent management and lifecycle
//! - Semantic delta computation
//! - Graph snapshot and diff capabilities
//! - Storage for intents, deltas, and snapshots
//! - Git integration

pub mod error;
pub mod git;
pub mod graph;
pub mod storage;
pub mod types;

pub use error::{Result, SmeltError};
pub use git::{CommitInfo, Git2Interface, GitInterface};
pub use graph::SmeltGraph;
pub use storage::SqliteStorage;
pub use types::{
    Author, AuthorType, Constraint, ContextLinks, DependencyType, GraphSnapshot, ImpactSummary,
    IntentId, IntentRecord, IntentStatus, SemanticChange, SemanticDelta, SnapshotId,
    SnapshotMetadata, Visibility,
};
