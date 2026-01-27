//! Benchmarks for smelt-memory operations

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use smelt_memory::{Episode, EpisodeOutcome, SmeltMemory};
use tempfile::TempDir;

fn create_test_episode(suffix: &str) -> Episode {
    Episode::new(
        format!("Test episode {} for benchmarking memory operations", suffix),
        "feature".to_string(),
        EpisodeOutcome::Success,
    )
    .with_project("benchmark-project".to_string())
    .with_tags(vec!["rust".to_string(), "benchmark".to_string()])
    .with_files(vec!["src/main.rs".to_string(), "src/lib.rs".to_string()])
}

fn bench_episode_storage(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_storage");

    // Benchmark episode capture (includes embedding generation)
    // Note: This is slow due to embedding model initialization
    group.sample_size(10);
    group.bench_function("capture_episode", |b| {
        let temp_dir = TempDir::new().unwrap();
        let mut memory = SmeltMemory::open(temp_dir.path()).unwrap();
        let mut counter = 0;

        b.iter(|| {
            counter += 1;
            let episode = create_test_episode(&counter.to_string());
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
                    let episode = create_test_episode(&format!("prepop_{}", i));
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
            let episode = create_test_episode(&format!("utility_{}", i));
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
