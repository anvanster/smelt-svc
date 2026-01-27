//! Tests for SQLite storage

use super::SqliteStorage;
use crate::types::{
    Author, AuthorType, Constraint, ContextLinks, ImpactSummary, IntentRecord, IntentStatus,
    SemanticDelta,
};
use chrono::Utc;
use tempfile::tempdir;
use uuid::Uuid;

fn create_test_intent() -> IntentRecord {
    IntentRecord {
        id: Uuid::new_v4(),
        created_at: Utc::now(),
        author: Author {
            name: "Test User".to_string(),
            email: "test@example.com".to_string(),
            author_type: AuthorType::Human,
        },
        goal: "Test goal".to_string(),
        rationale: Some("Test rationale".to_string()),
        constraints: vec![Constraint {
            name: "test".to_string(),
            value: "value".to_string(),
            required: true,
        }],
        context_links: ContextLinks::default(),
        status: IntentStatus::InProgress,
        baseline_snapshot_id: Some(Uuid::new_v4()),
    }
}

fn create_test_delta(intent_id: Uuid) -> SemanticDelta {
    SemanticDelta {
        id: Uuid::new_v4(),
        intent_id,
        timestamp: Utc::now(),
        from_snapshot: Uuid::new_v4(),
        to_snapshot: Uuid::new_v4(),
        changes: Vec::new(),
        impact_summary: ImpactSummary::default(),
    }
}

#[test]
fn test_open_creates_schema() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db");

    let storage = SqliteStorage::open(&db_path).unwrap();
    drop(storage);

    // Reopen should work
    let _storage = SqliteStorage::open(&db_path).unwrap();
}

#[test]
fn test_store_and_get_intent() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    let storage = SqliteStorage::open(&db_path).unwrap();

    let intent = create_test_intent();
    let intent_id = intent.id;

    storage.store_intent(&intent).unwrap();

    let retrieved = storage.get_intent(intent_id).unwrap().unwrap();
    assert_eq!(retrieved.id, intent_id);
    assert_eq!(retrieved.goal, "Test goal");
    assert_eq!(retrieved.author.name, "Test User");
}

#[test]
fn test_get_nonexistent_intent() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    let storage = SqliteStorage::open(&db_path).unwrap();

    let result = storage.get_intent(Uuid::new_v4()).unwrap();
    assert!(result.is_none());
}

#[test]
fn test_find_intent_by_prefix() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    let storage = SqliteStorage::open(&db_path).unwrap();

    let intent = create_test_intent();
    let prefix = intent.id.to_string()[..8].to_string();

    storage.store_intent(&intent).unwrap();

    let found = storage.find_intent_by_prefix(&prefix).unwrap().unwrap();
    assert_eq!(found.id, intent.id);
}

#[test]
fn test_find_intent_by_prefix_not_found() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    let storage = SqliteStorage::open(&db_path).unwrap();

    let result = storage.find_intent_by_prefix("nonexistent").unwrap();
    assert!(result.is_none());
}

#[test]
fn test_list_intents() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    let storage = SqliteStorage::open(&db_path).unwrap();

    // Store multiple intents
    for _ in 0..3 {
        let intent = create_test_intent();
        storage.store_intent(&intent).unwrap();
    }

    let intents = storage.list_intents(None).unwrap();
    assert_eq!(intents.len(), 3);
}

#[test]
fn test_list_intents_with_status_filter() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    let storage = SqliteStorage::open(&db_path).unwrap();

    // Store intent with InProgress status
    let mut intent1 = create_test_intent();
    intent1.status = IntentStatus::InProgress;
    storage.store_intent(&intent1).unwrap();

    // Store intent with Draft status
    let mut intent2 = create_test_intent();
    intent2.status = IntentStatus::Draft;
    storage.store_intent(&intent2).unwrap();

    let in_progress = storage.list_intents(Some(IntentStatus::InProgress)).unwrap();
    assert_eq!(in_progress.len(), 1);

    let drafts = storage.list_intents(Some(IntentStatus::Draft)).unwrap();
    assert_eq!(drafts.len(), 1);
}

#[test]
fn test_update_intent_status() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    let storage = SqliteStorage::open(&db_path).unwrap();

    let intent = create_test_intent();
    let intent_id = intent.id;
    storage.store_intent(&intent).unwrap();

    // Update to committed
    let new_status = IntentStatus::Committed {
        git_sha: "abc123".to_string(),
    };
    storage.update_intent_status(intent_id, new_status).unwrap();

    let retrieved = storage.get_intent(intent_id).unwrap().unwrap();
    match retrieved.status {
        IntentStatus::Committed { git_sha } => assert_eq!(git_sha, "abc123"),
        _ => panic!("Expected Committed status"),
    }
}

#[test]
fn test_update_nonexistent_intent_status() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    let storage = SqliteStorage::open(&db_path).unwrap();

    let result = storage.update_intent_status(Uuid::new_v4(), IntentStatus::Draft);
    assert!(result.is_err());
}

#[test]
fn test_store_and_get_delta() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    let storage = SqliteStorage::open(&db_path).unwrap();

    // First store an intent
    let intent = create_test_intent();
    storage.store_intent(&intent).unwrap();

    // Then store a delta
    let delta = create_test_delta(intent.id);
    let delta_id = delta.id;
    storage.store_delta(&delta).unwrap();

    let retrieved = storage.get_delta(delta_id).unwrap().unwrap();
    assert_eq!(retrieved.id, delta_id);
    assert_eq!(retrieved.intent_id, intent.id);
}

#[test]
fn test_get_deltas_for_intent() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    let storage = SqliteStorage::open(&db_path).unwrap();

    let intent = create_test_intent();
    storage.store_intent(&intent).unwrap();

    // Store multiple deltas for same intent
    for _ in 0..3 {
        let delta = create_test_delta(intent.id);
        storage.store_delta(&delta).unwrap();
    }

    let deltas = storage.get_deltas_for_intent(intent.id).unwrap();
    assert_eq!(deltas.len(), 3);
}

#[test]
fn test_intent_with_rejected_status() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    let storage = SqliteStorage::open(&db_path).unwrap();

    let mut intent = create_test_intent();
    intent.status = IntentStatus::Rejected {
        violations: vec!["violation1".to_string(), "violation2".to_string()],
    };
    storage.store_intent(&intent).unwrap();

    let retrieved = storage.get_intent(intent.id).unwrap().unwrap();
    match retrieved.status {
        IntentStatus::Rejected { violations } => {
            assert_eq!(violations.len(), 2);
            assert!(violations.contains(&"violation1".to_string()));
        }
        _ => panic!("Expected Rejected status"),
    }
}

#[test]
fn test_intent_with_all_author_types() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    let storage = SqliteStorage::open(&db_path).unwrap();

    for author_type in [AuthorType::Human, AuthorType::AI, AuthorType::Hybrid] {
        let mut intent = create_test_intent();
        intent.author.author_type = author_type;
        storage.store_intent(&intent).unwrap();

        let retrieved = storage.get_intent(intent.id).unwrap().unwrap();
        assert_eq!(retrieved.author.author_type, author_type);
    }
}

#[test]
fn test_upsert_intent() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    let storage = SqliteStorage::open(&db_path).unwrap();

    let mut intent = create_test_intent();
    storage.store_intent(&intent).unwrap();

    // Update the same intent
    intent.goal = "Updated goal".to_string();
    storage.store_intent(&intent).unwrap();

    let retrieved = storage.get_intent(intent.id).unwrap().unwrap();
    assert_eq!(retrieved.goal, "Updated goal");

    // Should still only have one intent
    let all = storage.list_intents(None).unwrap();
    assert_eq!(all.len(), 1);
}
