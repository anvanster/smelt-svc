// Copyright 2024-2026 Andrey Vasilevsky <anvanster@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Intent types - structured declaration of desired changes

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::SnapshotId;

/// Unique identifier for an intent
pub type IntentId = Uuid;

/// Structured declaration of a desired change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentRecord {
    /// Unique identifier
    pub id: IntentId,

    /// When the intent was created
    pub created_at: DateTime<Utc>,

    /// Who created this intent
    pub author: Author,

    /// Natural language description of the goal
    pub goal: String,

    /// Why this change is being made
    pub rationale: Option<String>,

    /// Constraints that must be satisfied
    pub constraints: Vec<Constraint>,

    /// Links to external context (issues, PRs, docs)
    pub context_links: ContextLinks,

    /// Current status of the intent
    pub status: IntentStatus,

    /// Snapshot ID of the codebase at intent creation
    pub baseline_snapshot_id: Option<SnapshotId>,
}

/// Author of an intent or change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Author {
    /// Display name
    pub name: String,

    /// Email address
    pub email: String,

    /// Type of author (human or AI)
    pub author_type: AuthorType,
}

/// Type of author
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthorType {
    /// Human developer
    Human,

    /// AI assistant (e.g., Claude)
    AI,

    /// Mixed human and AI collaboration
    Hybrid,
}

/// A constraint that must be satisfied
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraint {
    /// Constraint name/type
    pub name: String,

    /// Constraint value or description
    pub value: String,

    /// Whether this is a hard requirement
    pub required: bool,
}

/// Links to external context
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContextLinks {
    /// GitHub/GitLab issue URLs
    pub issues: Vec<String>,

    /// Pull request URLs
    pub pull_requests: Vec<String>,

    /// Documentation URLs
    pub documentation: Vec<String>,

    /// Other relevant URLs
    pub other: Vec<String>,
}

/// Status of an intent through its lifecycle
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum IntentStatus {
    /// Initial state, being drafted
    Draft,

    /// Actively being worked on
    InProgress,

    /// Awaiting validation
    PendingValidation,

    /// Passed all validation checks
    Validated,

    /// Successfully committed to git
    Committed {
        /// The git commit SHA
        git_sha: String,
    },

    /// Failed validation or was rejected
    Rejected {
        /// List of validation violations
        violations: Vec<String>,
    },

    /// Abandoned by the author
    Abandoned,
}

impl IntentStatus {
    /// Check if the intent is in a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            IntentStatus::Committed { .. }
                | IntentStatus::Rejected { .. }
                | IntentStatus::Abandoned
        )
    }

    /// Check if the intent is active (can be worked on)
    pub fn is_active(&self) -> bool {
        matches!(
            self,
            IntentStatus::Draft
                | IntentStatus::InProgress
                | IntentStatus::PendingValidation
                | IntentStatus::Validated
        )
    }
}
