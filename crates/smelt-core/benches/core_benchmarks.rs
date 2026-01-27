//! Benchmarks for smelt-core operations

use chrono::Utc;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use smelt_core::{
    Author, AuthorType, ContextLinks, ImpactSummary, IntentRecord, IntentStatus, SemanticDelta,
    SmeltGraph, SqliteStorage,
};
use tempfile::TempDir;
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
        goal: "Test intent for benchmarking".to_string(),
        rationale: Some("Benchmark rationale".to_string()),
        constraints: Vec::new(),
        context_links: ContextLinks::default(),
        status: IntentStatus::InProgress,
        baseline_snapshot_id: None,
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
        impact_summary: ImpactSummary {
            files_affected: 10,
            functions_added: 5,
            functions_removed: 2,
            functions_modified: 3,
            breaking_changes: 0,
            ..Default::default()
        },
    }
}

fn bench_storage_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("storage");

    // Benchmark intent storage
    group.bench_function("store_intent", |b| {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("bench.db");
        let storage = SqliteStorage::open(&db_path).unwrap();

        b.iter(|| {
            let intent = create_test_intent();
            storage.store_intent(black_box(&intent)).unwrap();
        });
    });

    // Benchmark intent retrieval
    group.bench_function("get_intent", |b| {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("bench.db");
        let storage = SqliteStorage::open(&db_path).unwrap();

        // Pre-populate with intents
        let mut ids = Vec::new();
        for _ in 0..100 {
            let intent = create_test_intent();
            ids.push(intent.id);
            storage.store_intent(&intent).unwrap();
        }

        let mut idx = 0;
        b.iter(|| {
            let id = ids[idx % ids.len()];
            idx += 1;
            storage.get_intent(black_box(id)).unwrap()
        });
    });

    // Benchmark intent listing
    group.bench_function("list_intents", |b| {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("bench.db");
        let storage = SqliteStorage::open(&db_path).unwrap();

        // Pre-populate
        for _ in 0..100 {
            let intent = create_test_intent();
            storage.store_intent(&intent).unwrap();
        }

        b.iter(|| storage.list_intents(black_box(None)).unwrap());
    });

    // Benchmark delta storage
    group.bench_function("store_delta", |b| {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("bench.db");
        let storage = SqliteStorage::open(&db_path).unwrap();

        let intent = create_test_intent();
        storage.store_intent(&intent).unwrap();

        b.iter(|| {
            let delta = create_test_delta(intent.id);
            storage.store_delta(black_box(&delta)).unwrap();
        });
    });

    group.finish();
}

fn bench_graph_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("graph");

    // Benchmark graph opening
    group.bench_function("open_graph", |b| {
        let temp_dir = TempDir::new().unwrap();
        let graph_path = temp_dir.path().join("graph");
        std::fs::create_dir_all(&graph_path).unwrap();

        // Initialize graph first
        let _ = SmeltGraph::open(&graph_path).unwrap();

        b.iter(|| SmeltGraph::open(black_box(&graph_path)).unwrap());
    });

    // Benchmark snapshot capture
    group.bench_function("snapshot", |b| {
        let temp_dir = TempDir::new().unwrap();
        let graph_path = temp_dir.path().join("graph");
        std::fs::create_dir_all(&graph_path).unwrap();

        let graph = SmeltGraph::open(&graph_path).unwrap();

        b.iter(|| graph.snapshot());
    });

    // Benchmark snapshot for intent
    group.bench_function("snapshot_for_intent", |b| {
        let temp_dir = TempDir::new().unwrap();
        let graph_path = temp_dir.path().join("graph");
        std::fs::create_dir_all(&graph_path).unwrap();

        let mut graph = SmeltGraph::open(&graph_path).unwrap();

        b.iter(|| {
            let intent_id = Uuid::new_v4();
            graph.snapshot_for_intent(black_box(intent_id))
        });
    });

    group.finish();
}

fn bench_intent_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("scaling");

    for size in [10, 100, 1000].iter() {
        group.bench_with_input(BenchmarkId::new("list_intents", size), size, |b, &size| {
            let temp_dir = TempDir::new().unwrap();
            let db_path = temp_dir.path().join("bench.db");
            let storage = SqliteStorage::open(&db_path).unwrap();

            // Pre-populate
            for _ in 0..size {
                let intent = create_test_intent();
                storage.store_intent(&intent).unwrap();
            }

            b.iter(|| storage.list_intents(None).unwrap());
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_storage_operations,
    bench_graph_operations,
    bench_intent_scaling,
);
criterion_main!(benches);
