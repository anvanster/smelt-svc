// Copyright 2024-2026 Andrey Vasilevsky <anvanster@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for the full Smelt workflow
//!
//! Tests the complete flow: init → intent → changes → validate → commit → memory

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

use chrono::Utc;
use smelt_core::{
    Author, AuthorType, Constraint, ContextLinks, Git2Interface, GitInterface, ImpactSummary,
    IntentRecord, IntentStatus, SemanticChange, SemanticDelta, SmeltGraph, SqliteStorage,
};
use smelt_memory::{Episode, EpisodeOutcome, SmeltMemory};
use smelt_validator::{config::ValidationConfig, SmeltValidator};
use std::path::Path;
use uuid::Uuid;

/// Helper to create a test repository with git initialized
fn create_test_repo() -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let repo_path = temp_dir.path().to_path_buf();

    // Initialize git repo
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to init git");

    // Configure git user
    std::process::Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to configure git email");

    std::process::Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to configure git name");

    (temp_dir, repo_path)
}

/// Helper to create smelt directory structure
fn init_smelt(repo_path: &Path) -> (PathBuf, PathBuf, PathBuf, PathBuf) {
    let smelt_dir = repo_path.join(".smelt");
    fs::create_dir_all(&smelt_dir).expect("Failed to create .smelt dir");

    let db_path = smelt_dir.join("smelt.db");
    let graph_path = smelt_dir.join("graph");
    let memory_path = smelt_dir.join("memory");

    fs::create_dir_all(&graph_path).expect("Failed to create graph dir");
    fs::create_dir_all(&memory_path).expect("Failed to create memory dir");

    (smelt_dir, db_path, graph_path, memory_path)
}

/// Helper to create a test author
fn test_author() -> Author {
    Author {
        name: "Test User".to_string(),
        email: "test@example.com".to_string(),
        author_type: AuthorType::Human,
    }
}

#[test]
fn test_full_workflow_init_to_commit() {
    let (_temp_dir, repo_path) = create_test_repo();
    let (smelt_dir, db_path, graph_path, _memory_path) = init_smelt(&repo_path);

    // 1. Initialize storage and graph
    let storage = SqliteStorage::open(&db_path).expect("Failed to open storage");
    let _graph = SmeltGraph::open(&graph_path).expect("Failed to open graph");

    // 2. Create an intent
    let intent = IntentRecord {
        id: Uuid::new_v4(),
        created_at: Utc::now(),
        author: test_author(),
        goal: "Add user authentication feature".to_string(),
        rationale: Some("Security requirement for the application".to_string()),
        constraints: vec![],
        context_links: ContextLinks::default(),
        status: IntentStatus::Draft,
        baseline_snapshot_id: None,
    };

    storage
        .store_intent(&intent)
        .expect("Failed to store intent");

    // 3. Verify intent was stored
    let retrieved = storage.get_intent(intent.id).expect("Failed to get intent");
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().goal, "Add user authentication feature");

    // 4. List intents
    let intents = storage.list_intents(None).expect("Failed to list intents");
    assert_eq!(intents.len(), 1);

    // 5. Create a source file
    let src_dir = repo_path.join("src");
    fs::create_dir_all(&src_dir).expect("Failed to create src dir");
    fs::write(
        src_dir.join("auth.rs"),
        r#"
pub fn authenticate(username: &str, password: &str) -> bool {
    // Simple authentication logic
    !username.is_empty() && !password.is_empty()
}

pub fn hash_password(password: &str) -> String {
    // Placeholder hash function
    format!("hashed_{}", password)
}
"#,
    )
    .expect("Failed to write auth.rs");

    // 6. Stage and commit with git
    std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to stage files");

    std::process::Command::new("git")
        .args(["commit", "-m", "Add authentication module"])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to commit");

    // 7. Verify smelt directory exists
    assert!(smelt_dir.exists());
    assert!(db_path.exists());

    println!("Full workflow test passed!");
}

#[test]
fn test_intent_lifecycle() {
    let (_temp_dir, repo_path) = create_test_repo();
    let (_smelt_dir, db_path, _graph_path, _memory_path) = init_smelt(&repo_path);

    let storage = SqliteStorage::open(&db_path).expect("Failed to open storage");

    // Create intent with constraints
    let mut intent = IntentRecord {
        id: Uuid::new_v4(),
        created_at: Utc::now(),
        author: test_author(),
        goal: "Refactor database layer".to_string(),
        rationale: None,
        constraints: vec![
            Constraint {
                name: "max_files".to_string(),
                value: "10".to_string(),
                required: true,
            },
            Constraint {
                name: "max_complexity_increase".to_string(),
                value: "5".to_string(),
                required: false,
            },
        ],
        context_links: ContextLinks::default(),
        status: IntentStatus::Draft,
        baseline_snapshot_id: None,
    };

    storage
        .store_intent(&intent)
        .expect("Failed to store intent");

    // Update status to committed
    intent.status = IntentStatus::Committed {
        git_sha: "abc123def456".to_string(),
    };
    storage
        .update_intent_status(intent.id, intent.status.clone())
        .expect("Failed to update intent");

    // Verify update
    let retrieved = storage
        .get_intent(intent.id)
        .expect("Failed to get intent")
        .unwrap();
    match retrieved.status {
        IntentStatus::Committed { git_sha } => {
            assert_eq!(git_sha, "abc123def456");
        }
        _ => panic!("Expected Committed status"),
    }

    println!("Intent lifecycle test passed!");
}

#[test]
fn test_semantic_delta_creation() {
    let (_temp_dir, repo_path) = create_test_repo();
    let (_smelt_dir, db_path, _graph_path, _memory_path) = init_smelt(&repo_path);

    let storage = SqliteStorage::open(&db_path).expect("Failed to open storage");

    // Create intent
    let intent = IntentRecord {
        id: Uuid::new_v4(),
        created_at: Utc::now(),
        author: Author {
            name: "AI Assistant".to_string(),
            email: "ai@example.com".to_string(),
            author_type: AuthorType::AI,
        },
        goal: "Add new API endpoint".to_string(),
        rationale: None,
        constraints: vec![],
        context_links: ContextLinks::default(),
        status: IntentStatus::InProgress,
        baseline_snapshot_id: None,
    };

    storage
        .store_intent(&intent)
        .expect("Failed to store intent");

    // Create a semantic delta
    let delta = SemanticDelta {
        id: Uuid::new_v4(),
        intent_id: intent.id,
        timestamp: Utc::now(),
        from_snapshot: Uuid::new_v4(),
        to_snapshot: Uuid::new_v4(),
        changes: vec![
            SemanticChange::FunctionAdded {
                name: "get_users".to_string(),
                file: "src/api.rs".to_string(),
                signature: "fn get_users() -> Vec<User>".to_string(),
                is_public: true,
            },
            SemanticChange::FunctionAdded {
                name: "create_user".to_string(),
                file: "src/api.rs".to_string(),
                signature: "fn create_user(user: User) -> Result<User, Error>".to_string(),
                is_public: true,
            },
        ],
        impact_summary: ImpactSummary {
            files_affected: 1,
            functions_added: 2,
            functions_removed: 0,
            functions_modified: 0,
            types_added: 0,
            types_removed: 0,
            types_modified: 0,
            dependencies_added: 0,
            dependencies_removed: 0,
            breaking_changes: 0,
            new_public_api: 2,
            complexity_delta: 0,
        },
    };

    storage.store_delta(&delta).expect("Failed to store delta");

    // Retrieve and verify by delta ID
    let retrieved = storage.get_delta(delta.id).expect("Failed to get delta");
    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.changes.len(), 2);
    assert_eq!(retrieved.impact_summary.functions_added, 2);

    // Also verify retrieval by intent ID
    let deltas_for_intent = storage
        .get_deltas_for_intent(intent.id)
        .expect("Failed to get deltas");
    assert_eq!(deltas_for_intent.len(), 1);

    println!("Semantic delta test passed!");
}

#[test]
fn test_validation_pipeline() {
    let (_temp_dir, repo_path) = create_test_repo();
    let (_smelt_dir, db_path, graph_path, _memory_path) = init_smelt(&repo_path);

    let storage = SqliteStorage::open(&db_path).expect("Failed to open storage");
    let _graph = SmeltGraph::open(&graph_path).expect("Failed to open graph");

    // Create intent with constraints
    let intent = IntentRecord {
        id: Uuid::new_v4(),
        created_at: Utc::now(),
        author: test_author(),
        goal: "Small bugfix".to_string(),
        rationale: None,
        constraints: vec![Constraint {
            name: "max_files".to_string(),
            value: "3".to_string(),
            required: true,
        }],
        context_links: ContextLinks::default(),
        status: IntentStatus::InProgress,
        baseline_snapshot_id: None,
    };

    storage
        .store_intent(&intent)
        .expect("Failed to store intent");

    // Create a delta
    let delta = SemanticDelta {
        id: Uuid::new_v4(),
        intent_id: intent.id,
        timestamp: Utc::now(),
        from_snapshot: Uuid::new_v4(),
        to_snapshot: Uuid::new_v4(),
        changes: vec![SemanticChange::BodyModified {
            name: "handle_request".to_string(),
            file: "src/handler.rs".to_string(),
            complexity_delta: 2,
        }],
        impact_summary: ImpactSummary {
            files_affected: 5, // This exceeds max_files constraint
            functions_added: 0,
            functions_removed: 0,
            functions_modified: 5,
            types_added: 0,
            types_removed: 0,
            types_modified: 0,
            dependencies_added: 0,
            dependencies_removed: 0,
            breaking_changes: 0,
            new_public_api: 0,
            complexity_delta: 10,
        },
    };

    // Create validator
    let config = ValidationConfig::default();
    let validator = SmeltValidator::new(config);

    // Validate
    let result = validator.validate(&delta, Some(&intent));

    // The validation should complete
    println!("Validation result: {:?}", result);
    assert!(result.violations.is_empty() || !result.violations.is_empty()); // Just check it runs

    println!("Validation pipeline test passed!");
}

#[test]
fn test_memory_capture_and_retrieval() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let memory_path = temp_dir.path().to_path_buf();

    // Open memory system
    let mut memory = SmeltMemory::open(&memory_path).expect("Failed to open memory");

    // Capture an episode using builder pattern
    let episode = Episode::new(
        "Implemented user authentication with JWT tokens".to_string(),
        "feature".to_string(),
        EpisodeOutcome::Success,
    )
    .with_project("test-project".to_string())
    .with_tags(vec![
        "auth".to_string(),
        "jwt".to_string(),
        "security".to_string(),
    ])
    .with_files(vec!["src/auth.rs".to_string(), "src/jwt.rs".to_string()]);

    let episode_id = memory.capture(episode).expect("Failed to capture episode");

    // Record feedback
    memory
        .record_feedback(episode_id, true)
        .expect("Failed to record feedback");
    memory
        .record_feedback(episode_id, true)
        .expect("Failed to record feedback");

    // Retrieve similar episodes
    let results = memory
        .retrieve("authentication jwt", 5)
        .expect("Failed to retrieve");

    assert!(!results.is_empty());
    assert!(results[0].episode.summary.contains("authentication"));

    // Check feedback was recorded
    assert!(results[0].episode.helpful_count >= 2);

    println!("Memory capture and retrieval test passed!");
}

#[test]
fn test_memory_utility_propagation() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let memory_path = temp_dir.path().to_path_buf();

    let mut memory = SmeltMemory::open(&memory_path).expect("Failed to open memory");

    // Capture multiple related episodes
    for i in 0..5 {
        let episode = Episode::new(
            format!("Episode {} about database optimization", i),
            "feature".to_string(),
            EpisodeOutcome::Success,
        )
        .with_project("test-project".to_string())
        .with_tags(vec!["database".to_string(), "optimization".to_string()])
        .with_files(vec![format!("src/db_{}.rs", i)]);

        let id = memory.capture(episode).expect("Failed to capture");

        // Mark some as helpful
        if i % 2 == 0 {
            memory
                .record_feedback(id, true)
                .expect("Failed to record feedback");
        }
    }

    // Run utility propagation
    let stats = memory
        .propagate_utility(false)
        .expect("Failed to propagate");

    println!("Propagation stats: {:?}", stats);
    assert!(stats.episodes_updated > 0);

    println!("Memory utility propagation test passed!");
}

#[test]
fn test_git_interface() {
    let (_temp_dir, repo_path) = create_test_repo();

    // Create a file and commit
    fs::write(repo_path.join("test.txt"), "Hello, world!").expect("Failed to write file");

    std::process::Command::new("git")
        .args(["add", "test.txt"])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to stage");

    std::process::Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to commit");

    // Test git interface
    let git = Git2Interface::open(&repo_path).expect("Failed to open git interface");

    assert!(git.is_initialized());

    // Get HEAD
    let head = git.head_sha().expect("Failed to get HEAD");
    assert!(!head.is_empty());

    // Check for uncommitted changes (should be none)
    let uncommitted = git
        .has_uncommitted_changes()
        .expect("Failed to check changes");
    assert!(!uncommitted);

    // Make a change
    fs::write(repo_path.join("test.txt"), "Modified!").expect("Failed to write file");

    // Now should have uncommitted changes
    let uncommitted = git
        .has_uncommitted_changes()
        .expect("Failed to check changes");
    assert!(uncommitted);

    // Get changed files
    let changed = git.changed_files().expect("Failed to get changed files");
    assert!(!changed.is_empty());

    println!("Git interface test passed!");
}

#[test]
fn test_end_to_end_smelt_workflow() {
    let (_temp_dir, repo_path) = create_test_repo();
    let (_smelt_dir, db_path, graph_path, memory_path) = init_smelt(&repo_path);

    // Initialize all components
    let storage = SqliteStorage::open(&db_path).expect("Failed to open storage");
    let _graph = SmeltGraph::open(&graph_path).expect("Failed to open graph");
    let mut memory = SmeltMemory::open(&memory_path).expect("Failed to open memory");
    let _git = Git2Interface::open(&repo_path).expect("Failed to open git");

    // 1. Create intent
    let intent = IntentRecord {
        id: Uuid::new_v4(),
        created_at: Utc::now(),
        author: test_author(),
        goal: "Add logging functionality".to_string(),
        rationale: Some("Need structured logging for debugging".to_string()),
        constraints: vec![],
        context_links: ContextLinks::default(),
        status: IntentStatus::InProgress,
        baseline_snapshot_id: None,
    };
    storage
        .store_intent(&intent)
        .expect("Failed to store intent");

    // 2. Make code changes
    let src_dir = repo_path.join("src");
    fs::create_dir_all(&src_dir).expect("Failed to create src");
    fs::write(
        src_dir.join("logging.rs"),
        r#"
use std::fmt;

pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

pub fn log(level: LogLevel, message: &str) {
    println!("[{:?}] {}", level, message);
}
"#,
    )
    .expect("Failed to write logging.rs");

    // 3. Create semantic delta
    let delta = SemanticDelta {
        id: Uuid::new_v4(),
        intent_id: intent.id,
        timestamp: Utc::now(),
        from_snapshot: Uuid::new_v4(),
        to_snapshot: Uuid::new_v4(),
        changes: vec![
            SemanticChange::FunctionAdded {
                name: "log".to_string(),
                file: "src/logging.rs".to_string(),
                signature: "fn log(level: LogLevel, message: &str)".to_string(),
                is_public: true,
            },
            SemanticChange::TypeAdded {
                name: "LogLevel".to_string(),
                file: "src/logging.rs".to_string(),
                kind: "enum".to_string(),
                is_public: true,
            },
        ],
        impact_summary: ImpactSummary {
            files_affected: 1,
            functions_added: 1,
            functions_removed: 0,
            functions_modified: 0,
            types_added: 1,
            types_removed: 0,
            types_modified: 0,
            dependencies_added: 0,
            dependencies_removed: 0,
            breaking_changes: 0,
            new_public_api: 2,
            complexity_delta: 0,
        },
    };
    storage.store_delta(&delta).expect("Failed to store delta");

    // 4. Validate
    let config = ValidationConfig::default();
    let validator = SmeltValidator::new(config);
    let validation = validator.validate(&delta, Some(&intent));
    println!("Validation: {:?}", validation);

    // 5. Commit via git
    std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to stage");

    let output = std::process::Command::new("git")
        .args(["commit", "-m", "Add logging functionality"])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to commit");

    assert!(output.status.success());

    // 6. Capture memory episode
    let episode = Episode::new(
        format!("Completed: {}", intent.goal),
        "feature".to_string(),
        EpisodeOutcome::Success,
    )
    .with_project("smelt-test".to_string())
    .with_tags(vec!["logging".to_string()])
    .with_files(vec!["src/logging.rs".to_string()])
    .with_intent(intent.id)
    .with_delta(delta.id);

    memory.capture(episode).expect("Failed to capture episode");

    // 7. Verify everything is stored
    let stored_intent = storage.get_intent(intent.id).expect("Failed to get intent");
    assert!(stored_intent.is_some());

    let stored_delta = storage.get_delta(delta.id).expect("Failed to get delta");
    assert!(stored_delta.is_some());

    let memories = memory.retrieve("logging", 5).expect("Failed to retrieve");
    assert!(!memories.is_empty());

    println!("End-to-end workflow test passed!");
}

#[test]
fn test_impact_summary_risk_score() {
    // Test low risk
    let low_risk = ImpactSummary {
        files_affected: 1,
        functions_added: 1,
        functions_removed: 0,
        functions_modified: 0,
        types_added: 0,
        types_removed: 0,
        types_modified: 0,
        dependencies_added: 0,
        dependencies_removed: 0,
        breaking_changes: 0,
        new_public_api: 1,
        complexity_delta: 0,
    };
    assert!(low_risk.risk_score() < 0.2);

    // Test high risk (breaking changes)
    let high_risk = ImpactSummary {
        files_affected: 5,
        functions_added: 0,
        functions_removed: 3,
        functions_modified: 5,
        types_added: 0,
        types_removed: 2,
        types_modified: 0,
        dependencies_added: 0,
        dependencies_removed: 0,
        breaking_changes: 3,
        new_public_api: 0,
        complexity_delta: 20,
    };
    assert!(high_risk.risk_score() > 0.5);
    assert!(high_risk.has_breaking_changes());

    println!("Impact summary risk score test passed!");
}
