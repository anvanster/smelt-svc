// Copyright 2024-2026 Andrey Vasilevsky <anvanster@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Adapter for integrating Crucible's architectural validation

use crate::rules::ValidationRule;
use crate::validator::{ValidationSeverity, Violation};
use crucible_core::validator::{ValidationIssue, ValidationResult, Validator};
use crucible_core::{Parser, Project};
use smelt_core::{IntentRecord, SemanticDelta};
use std::path::{Path, PathBuf};

/// Adapter that bridges Crucible's validation with SmeltValidator
pub struct CrucibleAdapter {
    /// Root path for the project
    project_root: PathBuf,
    /// Whether architectural validation is enabled
    enabled: bool,
    /// Whether to check for circular dependencies
    check_circular_deps: bool,
    /// Whether to validate type references
    check_types: bool,
    /// Whether to validate call targets
    check_calls: bool,
}

impl CrucibleAdapter {
    /// Create a new Crucible adapter
    pub fn new(project_root: &Path) -> Self {
        Self {
            project_root: project_root.to_path_buf(),
            enabled: true,
            check_circular_deps: true,
            check_types: true,
            check_calls: true,
        }
    }

    /// Disable the adapter
    pub fn disabled() -> Self {
        Self {
            project_root: PathBuf::new(),
            enabled: false,
            check_circular_deps: false,
            check_types: false,
            check_calls: false,
        }
    }

    /// Set whether to check circular dependencies
    pub fn with_circular_deps(mut self, check: bool) -> Self {
        self.check_circular_deps = check;
        self
    }

    /// Set whether to check type references
    pub fn with_type_checks(mut self, check: bool) -> Self {
        self.check_types = check;
        self
    }

    /// Set whether to check call targets
    pub fn with_call_checks(mut self, check: bool) -> Self {
        self.check_calls = check;
        self
    }

    /// Check if a Crucible project exists at the root
    fn has_crucible_project(&self) -> bool {
        self.project_root.join("crucible.json").exists()
            || self.project_root.join("crucible.yaml").exists()
    }

    /// Parse the Crucible project
    fn parse_project(&self) -> Option<Project> {
        if !self.has_crucible_project() {
            return None;
        }

        let parser = Parser::new(&self.project_root);
        parser.parse_project().ok()
    }

    /// Run Crucible validation and convert results
    fn run_crucible_validation(&self) -> Vec<Violation> {
        let Some(project) = self.parse_project() else {
            return Vec::new();
        };

        let validator = Validator::new(project);
        let result = validator.validate();

        self.convert_result(&result)
    }

    /// Convert Crucible ValidationResult to our Violations
    fn convert_result(&self, result: &ValidationResult) -> Vec<Violation> {
        let mut violations = Vec::new();

        // Convert errors
        for issue in &result.errors {
            if self.should_include_issue(issue) {
                violations.push(self.convert_issue(issue, ValidationSeverity::Error));
            }
        }

        // Convert warnings
        for issue in &result.warnings {
            if self.should_include_issue(issue) {
                violations.push(self.convert_issue(issue, ValidationSeverity::Warning));
            }
        }

        // Convert info
        for issue in &result.info {
            if self.should_include_issue(issue) {
                violations.push(self.convert_issue(issue, ValidationSeverity::Info));
            }
        }

        violations
    }

    /// Check if an issue should be included based on our config
    fn should_include_issue(&self, issue: &ValidationIssue) -> bool {
        match issue.rule.as_str() {
            r if r.contains("circular") || r.contains("cycle") => self.check_circular_deps,
            r if r.contains("type") => self.check_types,
            r if r.contains("call") => self.check_calls,
            _ => true,
        }
    }

    /// Convert a Crucible ValidationIssue to our Violation
    fn convert_issue(&self, issue: &ValidationIssue, severity: ValidationSeverity) -> Violation {
        let mut message = issue.message.clone();

        // Add found/expected context if available
        if let (Some(found), Some(expected)) = (&issue.found, &issue.expected) {
            message = format!("{} (found: {}, expected: {})", message, found, expected);
        }

        Violation {
            rule: format!("crucible:{}", issue.rule),
            severity,
            message,
            location: issue.location.clone(),
            suggestion: issue.suggestion.clone(),
        }
    }
}

impl ValidationRule for CrucibleAdapter {
    fn name(&self) -> &'static str {
        "crucible"
    }

    fn validate(&self, _delta: &SemanticDelta, _intent: Option<&IntentRecord>) -> Vec<Violation> {
        if !self.enabled {
            return Vec::new();
        }

        self.run_crucible_validation()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_disabled_adapter() {
        let adapter = CrucibleAdapter::disabled();
        assert!(!adapter.enabled);
    }

    #[test]
    fn test_no_crucible_project() {
        let dir = tempdir().unwrap();
        let adapter = CrucibleAdapter::new(dir.path());
        assert!(!adapter.has_crucible_project());
    }

    #[test]
    fn test_config_builder() {
        let dir = tempdir().unwrap();
        let adapter = CrucibleAdapter::new(dir.path())
            .with_circular_deps(false)
            .with_type_checks(false)
            .with_call_checks(true);

        assert!(!adapter.check_circular_deps);
        assert!(!adapter.check_types);
        assert!(adapter.check_calls);
    }
}
