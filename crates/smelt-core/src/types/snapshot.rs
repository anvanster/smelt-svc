// Copyright 2024-2026 Andrey Vasilevsky <anvanster@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Graph snapshot types - serialized graph state with metadata

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Unique identifier for a snapshot
pub type SnapshotId = Uuid;

/// A serialized snapshot of the code graph at a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphSnapshot {
    /// Unique identifier
    pub id: SnapshotId,

    /// When this snapshot was captured
    pub timestamp: DateTime<Utc>,

    /// Git commit SHA this snapshot corresponds to (if any)
    pub git_sha: Option<String>,

    /// Number of nodes in the graph
    pub node_count: usize,

    /// Number of edges in the graph
    pub edge_count: usize,

    /// Summary of nodes by kind
    pub nodes_by_kind: HashMap<String, usize>,

    /// File paths included in this snapshot
    pub files: Vec<String>,

    /// Checksum for integrity verification
    pub checksum: String,
}

impl GraphSnapshot {
    /// Create a new empty snapshot
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            git_sha: None,
            node_count: 0,
            edge_count: 0,
            nodes_by_kind: HashMap::new(),
            files: Vec::new(),
            checksum: String::new(),
        }
    }

    /// Create a snapshot with the given counts
    pub fn with_counts(node_count: usize, edge_count: usize) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            git_sha: None,
            node_count,
            edge_count,
            nodes_by_kind: HashMap::new(),
            files: Vec::new(),
            checksum: String::new(),
        }
    }
}

impl Default for GraphSnapshot {
    fn default() -> Self {
        Self::new()
    }
}

/// Metadata about a snapshot for quick lookup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMetadata {
    /// Snapshot ID
    pub id: SnapshotId,

    /// When captured
    pub timestamp: DateTime<Utc>,

    /// Associated git SHA
    pub git_sha: Option<String>,

    /// Node count
    pub node_count: usize,

    /// Edge count
    pub edge_count: usize,
}

impl From<&GraphSnapshot> for SnapshotMetadata {
    fn from(snapshot: &GraphSnapshot) -> Self {
        Self {
            id: snapshot.id,
            timestamp: snapshot.timestamp,
            git_sha: snapshot.git_sha.clone(),
            node_count: snapshot.node_count,
            edge_count: snapshot.edge_count,
        }
    }
}
