//! Benchmarks for smelt-memory operations

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use smelt_memory::{Episode, EpisodeOutcome, SmeltMemory, TaskType};
use chrono::Utc;
use uuid::Uuid;
use tempfile::TempDir;

fn create_test_episode() -> Episode {
    Episode {
        id: Uuid::new_v4(),
        project: Some("benchmark-project".to_string()),
        session_id: Some(Uuid::new_v4()),
        summary: "Test episode for benchmarking memory operations".to_string(),
        task_type: TaskType::Feature,
        outcome: EpisodeOutcome::Success,
        tags: vec!["rust".to_string(), "benchmark".to_string()],
        files_modified: vec!["src/main.rs".to_string(), "src/lib.rs".to_string()],
        errors_resolved: Vec::new(),
        embedding: None,
        utility_score: 0.0,
        helpful_count: 0,
        not_helpful_count: 0,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

fn bench_episode_storage(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_storage");

    // Benchmark episode capture (includes embedding generation)
    // Note: This is slow due to embedding model initialization
    group.sample_size(10);
    group.bench_function("capture_episode", |b| {
        let temp_dir = TempDir::new().unwrap();
        let mut memory = SmeltMemory::open(temp_dir.path()).unwrap();

        b.iter(|| {
            let episode = create_test_episode();
            memory.capture(black_box(episode)).unwrap();
        });
    });

    group.finish();
}

fn bench_episode_retrieval(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_retrieval");
    group.sample_size(10);

    // Benchmark retrieval with different episode counts
    for size in [10, 50].iter() {
        group.bench_with_input(
            BenchmarkId::new("retrieve", size),
            size,
            |b, &size| {
                let temp_dir = TempDir::new().unwrap();
                let mut memory = SmeltMemory::open(temp_dir.path()).unwrap();

                // Pre-populate
                for i in 0..size {
                    let mut episode = create_test_episode();
                    episode.summary = format!("Test episode {} for benchmarking", i);
                    memory.capture(episode).unwrap();
                }

                b.iter(|| {
                    memory.retrieve(black_box("test benchmarking"), 5).unwrap()
                });
            },
        );
    }

    group.finish();
}

fn bench_utility_computation(c: &mut Criterion) {
    let mut group = c.benchmark_group("utility");
    group.sample_size(10);

    // Benchmark utility propagation
    group.bench_function("propagate_utility", |b| {
        let temp_dir = TempDir::new().unwrap();
        let mut memory = SmeltMemory::open(temp_dir.path()).unwrap();

        // Pre-populate with episodes
        for i in 0..20 {
            let mut episode = create_test_episode();
            episode.summary = format!("Test episode {} for utility propagation", i);
            let id = memory.capture(episode).unwrap();

            // Add some feedback
            if i % 3 == 0 {
                memory.record_feedback(id, true).unwrap();
            }
        }

        b.iter(|| {
            memory.propagate_utility(black_box(false)).unwrap()
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_episode_storage,
    bench_episode_retrieval,
    bench_utility_computation,
);
criterion_main!(benches);
