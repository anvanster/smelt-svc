//! Smelt Validator - Semantic delta and architectural validation
//!
//! This crate provides validation capabilities for Smelt:
//! - Semantic delta validation (breaking changes, visibility, complexity)
//! - Architectural validation via Crucible integration
//! - Intent constraint validation

pub mod config;
pub mod crucible;
pub mod error;
pub mod rules;
pub mod semantic;
pub mod validator;

pub use crucible::CrucibleAdapter;
pub use error::{ValidationError, ValidationResult};
pub use validator::{SmeltValidator, ValidationOutcome, ValidationSeverity, Violation};
