// Copyright 2024-2026 Andrey Vasilevsky <anvanster@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Semantic delta types - machine-readable representation of code changes

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{IntentId, SnapshotId};

/// Unique identifier for a semantic delta
pub type DeltaId = Uuid;

/// Machine-readable representation of code meaning changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticDelta {
    /// Unique identifier
    pub id: DeltaId,

    /// The intent this delta fulfills
    pub intent_id: IntentId,

    /// When this delta was computed
    pub timestamp: DateTime<Utc>,

    /// Snapshot before changes
    pub from_snapshot: SnapshotId,

    /// Snapshot after changes
    pub to_snapshot: SnapshotId,

    /// List of semantic changes
    pub changes: Vec<SemanticChange>,

    /// Summary of the delta's impact
    pub impact_summary: ImpactSummary,
}

/// A single semantic change between snapshots
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SemanticChange {
    /// A function was added
    FunctionAdded {
        /// Fully qualified name
        name: String,
        /// File path
        file: String,
        /// Function signature
        signature: String,
        /// Whether it's public
        is_public: bool,
    },

    /// A function was removed
    FunctionRemoved {
        /// Fully qualified name
        name: String,
        /// File path
        file: String,
        /// Whether it was public (breaking if so)
        was_public: bool,
    },

    /// A function's signature changed
    SignatureChanged {
        /// Fully qualified name
        name: String,
        /// File path
        file: String,
        /// Old signature
        old_signature: String,
        /// New signature
        new_signature: String,
        /// Whether this is a breaking change
        is_breaking: bool,
    },

    /// A function's body changed
    BodyModified {
        /// Fully qualified name
        name: String,
        /// File path
        file: String,
        /// Complexity change (positive = more complex)
        complexity_delta: i32,
    },

    /// A type/struct/class was added
    TypeAdded {
        /// Type name
        name: String,
        /// File path
        file: String,
        /// Kind (struct, class, enum, etc.)
        kind: String,
        /// Whether it's public
        is_public: bool,
    },

    /// A type was removed
    TypeRemoved {
        /// Type name
        name: String,
        /// File path
        file: String,
        /// Whether it was public
        was_public: bool,
    },

    /// A type's definition changed
    TypeModified {
        /// Type name
        name: String,
        /// File path
        file: String,
        /// Fields/variants added
        fields_added: Vec<String>,
        /// Fields/variants removed
        fields_removed: Vec<String>,
        /// Whether this is a breaking change
        is_breaking: bool,
    },

    /// A dependency was added
    DependencyAdded {
        /// From node (caller)
        from: String,
        /// To node (callee)
        to: String,
        /// Type of dependency
        dependency_type: DependencyType,
    },

    /// A dependency was removed
    DependencyRemoved {
        /// From node
        from: String,
        /// To node
        to: String,
        /// Type of dependency
        dependency_type: DependencyType,
    },

    /// Visibility changed (public <-> private)
    VisibilityChanged {
        /// Node name
        name: String,
        /// File path
        file: String,
        /// Old visibility
        old_visibility: Visibility,
        /// New visibility
        new_visibility: Visibility,
    },

    /// A file was added
    FileAdded {
        /// File path
        path: String,
        /// Number of symbols defined
        symbol_count: usize,
    },

    /// A file was removed
    FileRemoved {
        /// File path
        path: String,
        /// Number of symbols that were defined
        symbol_count: usize,
    },

    /// A module/namespace structure changed
    ModuleReorganized {
        /// Description of the reorganization
        description: String,
        /// Affected paths
        affected_paths: Vec<String>,
    },
}

/// Type of dependency relationship
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DependencyType {
    /// Function call
    Call,
    /// Import/use statement
    Import,
    /// Type reference
    TypeReference,
    /// Inheritance/implementation
    Inheritance,
    /// Composition (field of type)
    Composition,
}

/// Visibility level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Visibility {
    /// Public API
    Public,
    /// Crate/module internal
    Internal,
    /// Private
    Private,
}

/// Summary of a delta's impact
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ImpactSummary {
    /// Number of files affected
    pub files_affected: usize,

    /// Functions added
    pub functions_added: usize,

    /// Functions removed
    pub functions_removed: usize,

    /// Functions modified (body or signature)
    pub functions_modified: usize,

    /// Types added
    pub types_added: usize,

    /// Types removed
    pub types_removed: usize,

    /// Types modified
    pub types_modified: usize,

    /// Dependencies added
    pub dependencies_added: usize,

    /// Dependencies removed
    pub dependencies_removed: usize,

    /// Number of breaking changes
    pub breaking_changes: usize,

    /// New public API surface (count of new public symbols)
    pub new_public_api: usize,

    /// Complexity delta (sum of all complexity changes)
    pub complexity_delta: i32,
}

impl ImpactSummary {
    /// Check if this delta has any breaking changes
    pub fn has_breaking_changes(&self) -> bool {
        self.breaking_changes > 0
    }

    /// Check if this delta adds to the public API
    pub fn expands_public_api(&self) -> bool {
        self.new_public_api > 0
    }

    /// Get a risk score (0.0 - 1.0) based on impact
    pub fn risk_score(&self) -> f64 {
        let mut score = 0.0;

        // Breaking changes are highest risk
        score += (self.breaking_changes as f64) * 0.3;

        // Removed functions/types are risky
        score += (self.functions_removed as f64) * 0.15;
        score += (self.types_removed as f64) * 0.15;

        // Large changes are riskier
        let total_changes = self.functions_added
            + self.functions_removed
            + self.functions_modified
            + self.types_added
            + self.types_removed
            + self.types_modified;

        score += (total_changes as f64) * 0.02;

        // Complexity increase is risky
        if self.complexity_delta > 0 {
            score += (self.complexity_delta as f64) * 0.01;
        }

        // Cap at 1.0
        score.min(1.0)
    }
}
