//! Validation rules

mod breaking;
mod complexity;
mod visibility;

pub use breaking::BreakingChangeChecker;
pub use complexity::ComplexityChecker;
pub use visibility::VisibilityChecker;

use crate::validator::Violation;
use smelt_core::{IntentRecord, SemanticDelta};

/// Trait for validation rules
pub trait ValidationRule: Send + Sync {
    /// Rule name
    fn name(&self) -> &'static str;

    /// Validate a semantic delta
    fn validate(&self, delta: &SemanticDelta, intent: Option<&IntentRecord>) -> Vec<Violation>;
}
