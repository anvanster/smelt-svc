//! Core types for Smelt semantic version control

mod delta;
mod intent;
mod snapshot;

#[cfg(test)]
mod tests;

pub use delta::{DependencyType, ImpactSummary, SemanticChange, SemanticDelta, Visibility};
pub use intent::{
    Author, AuthorType, Constraint, ContextLinks, IntentId, IntentRecord, IntentStatus,
};
pub use snapshot::{GraphSnapshot, SnapshotId, SnapshotMetadata};
