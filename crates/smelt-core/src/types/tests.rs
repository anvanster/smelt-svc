//! Tests for core types

use super::*;
use chrono::Utc;
use uuid::Uuid;

mod intent_tests {
    use super::*;

    #[test]
    fn test_intent_status_is_terminal() {
        assert!(!IntentStatus::Draft.is_terminal());
        assert!(!IntentStatus::InProgress.is_terminal());
        assert!(!IntentStatus::PendingValidation.is_terminal());
        assert!(!IntentStatus::Validated.is_terminal());

        assert!(IntentStatus::Committed {
            git_sha: "abc".to_string()
        }
        .is_terminal());
        assert!(IntentStatus::Rejected {
            violations: vec![]
        }
        .is_terminal());
        assert!(IntentStatus::Abandoned.is_terminal());
    }

    #[test]
    fn test_intent_status_is_active() {
        assert!(IntentStatus::Draft.is_active());
        assert!(IntentStatus::InProgress.is_active());
        assert!(IntentStatus::PendingValidation.is_active());
        assert!(IntentStatus::Validated.is_active());

        assert!(!IntentStatus::Committed {
            git_sha: "abc".to_string()
        }
        .is_active());
        assert!(!IntentStatus::Rejected {
            violations: vec![]
        }
        .is_active());
        assert!(!IntentStatus::Abandoned.is_active());
    }

    #[test]
    fn test_context_links_default() {
        let links = ContextLinks::default();
        assert!(links.issues.is_empty());
        assert!(links.pull_requests.is_empty());
        assert!(links.documentation.is_empty());
        assert!(links.other.is_empty());
    }
}

mod delta_tests {
    use super::*;

    #[test]
    fn test_impact_summary_default() {
        let summary = ImpactSummary::default();
        assert_eq!(summary.files_affected, 0);
        assert_eq!(summary.functions_added, 0);
        assert_eq!(summary.breaking_changes, 0);
        assert_eq!(summary.complexity_delta, 0);
    }

    #[test]
    fn test_impact_summary_has_breaking_changes() {
        let mut summary = ImpactSummary::default();
        assert!(!summary.has_breaking_changes());

        summary.breaking_changes = 1;
        assert!(summary.has_breaking_changes());
    }

    #[test]
    fn test_impact_summary_expands_public_api() {
        let mut summary = ImpactSummary::default();
        assert!(!summary.expands_public_api());

        summary.new_public_api = 5;
        assert!(summary.expands_public_api());
    }

    #[test]
    fn test_impact_summary_risk_score_empty() {
        let summary = ImpactSummary::default();
        assert_eq!(summary.risk_score(), 0.0);
    }

    #[test]
    fn test_impact_summary_risk_score_breaking_changes() {
        let mut summary = ImpactSummary::default();
        summary.breaking_changes = 3;
        let score = summary.risk_score();
        assert!(score > 0.8); // 3 * 0.3 = 0.9
    }

    #[test]
    fn test_impact_summary_risk_score_capped_at_one() {
        let mut summary = ImpactSummary::default();
        summary.breaking_changes = 10;
        summary.functions_removed = 20;
        summary.complexity_delta = 100;
        assert_eq!(summary.risk_score(), 1.0);
    }

    #[test]
    fn test_impact_summary_risk_score_removals() {
        let mut summary = ImpactSummary::default();
        summary.functions_removed = 2;
        summary.types_removed = 1;
        let score = summary.risk_score();
        assert!(score > 0.0);
        assert!(score < 1.0);
    }

    #[test]
    fn test_dependency_type_values() {
        // Just ensure all variants are valid
        let _ = DependencyType::Call;
        let _ = DependencyType::Import;
        let _ = DependencyType::TypeReference;
        let _ = DependencyType::Inheritance;
        let _ = DependencyType::Composition;
    }

    #[test]
    fn test_visibility_values() {
        let _ = Visibility::Public;
        let _ = Visibility::Internal;
        let _ = Visibility::Private;
    }

    #[test]
    fn test_semantic_change_variants() {
        // Test that all SemanticChange variants can be constructed
        let changes = vec![
            SemanticChange::FunctionAdded {
                name: "test".to_string(),
                file: "test.rs".to_string(),
                signature: "fn test()".to_string(),
                is_public: true,
            },
            SemanticChange::FunctionRemoved {
                name: "old".to_string(),
                file: "old.rs".to_string(),
                was_public: false,
            },
            SemanticChange::SignatureChanged {
                name: "func".to_string(),
                file: "f.rs".to_string(),
                old_signature: "fn f()".to_string(),
                new_signature: "fn f(x: i32)".to_string(),
                is_breaking: true,
            },
            SemanticChange::BodyModified {
                name: "func".to_string(),
                file: "f.rs".to_string(),
                complexity_delta: 5,
            },
            SemanticChange::TypeAdded {
                name: "MyType".to_string(),
                file: "types.rs".to_string(),
                kind: "struct".to_string(),
                is_public: true,
            },
            SemanticChange::FileAdded {
                path: "new.rs".to_string(),
                symbol_count: 10,
            },
            SemanticChange::FileRemoved {
                path: "old.rs".to_string(),
                symbol_count: 5,
            },
        ];

        assert_eq!(changes.len(), 7);
    }
}

mod snapshot_tests {
    use super::*;

    #[test]
    fn test_graph_snapshot_new() {
        let snapshot = GraphSnapshot::new();
        assert!(!snapshot.id.is_nil());
        assert_eq!(snapshot.node_count, 0);
        assert_eq!(snapshot.edge_count, 0);
        assert!(snapshot.git_sha.is_none());
        assert!(snapshot.files.is_empty());
    }

    #[test]
    fn test_graph_snapshot_with_counts() {
        let snapshot = GraphSnapshot::with_counts(100, 50);
        assert_eq!(snapshot.node_count, 100);
        assert_eq!(snapshot.edge_count, 50);
    }

    #[test]
    fn test_graph_snapshot_default() {
        let snapshot = GraphSnapshot::default();
        assert_eq!(snapshot.node_count, 0);
        assert_eq!(snapshot.edge_count, 0);
    }

    #[test]
    fn test_snapshot_metadata_from_snapshot() {
        let snapshot = GraphSnapshot::with_counts(42, 21);
        let metadata: SnapshotMetadata = (&snapshot).into();

        assert_eq!(metadata.id, snapshot.id);
        assert_eq!(metadata.node_count, 42);
        assert_eq!(metadata.edge_count, 21);
    }
}
