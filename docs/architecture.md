# Smelt Architecture

## Overview

Smelt is built as a Rust workspace with five crates, each handling a specific domain:

```
┌─────────────────────────────────────────────────────────────┐
│                      smelt-cli / smelt-api                  │
│                    (User Interface Layer)                    │
├─────────────────────────────────────────────────────────────┤
│                      smelt-validator                         │
│              (Semantic + Architectural Validation)           │
├─────────────────────────────────────────────────────────────┤
│        smelt-memory              │        smelt-core         │
│    (Episodic Memory System)      │   (Core Types + Storage)  │
├──────────────────────────────────┼───────────────────────────┤
│     SQLite + fastembed           │  SQLite + RocksDB + git2  │
└──────────────────────────────────┴───────────────────────────┘
```

## Crate Responsibilities

### smelt-core

The foundational crate providing:

- **Types**: `IntentRecord`, `SemanticDelta`, `GraphSnapshot`, `SemanticChange`
- **Storage**: SQLite persistence for intents and deltas
- **Graph**: SmeltGraph wrapper around CodeGraph for semantic analysis
- **Git**: GitInterface trait with git2-rs implementation

Key structures:

```rust
pub struct IntentRecord {
    pub id: IntentId,
    pub goal: String,
    pub rationale: Option<String>,
    pub author: Author,
    pub constraints: Vec<Constraint>,
    pub status: IntentStatus,
    pub baseline_snapshot_id: Option<SnapshotId>,
}

pub struct SemanticDelta {
    pub id: DeltaId,
    pub intent_id: IntentId,
    pub changes: Vec<SemanticChange>,
    pub impact_summary: ImpactSummary,
}
```

### smelt-validator

Validation pipeline combining:

- **Semantic Validation**: Breaking changes, visibility changes, complexity thresholds
- **Architectural Validation**: Layer boundaries, circular dependency detection
- **Intent Validation**: Constraint enforcement, scope validation

Configuration via `crucible.yaml` or `validation.yaml`:

```yaml
architecture:
  layers:
    - name: api
      paths: ["src/api/**"]
      prohibited_dependencies: [infrastructure]
  check_circular_deps: true

semantic:
  check_breaking_changes: true
  complexity:
    max_cyclomatic: 15
    max_cognitive: 25
```

### smelt-memory

Episodic memory system inspired by MemRL:

- **Episode Capture**: Records coding sessions with outcomes
- **Semantic Search**: Vector similarity via fastembed (BGE-Small)
- **Utility Ranking**: Wilson score + Bellman propagation + exponential decay
- **Feedback Learning**: Improves retrieval through user feedback

Key algorithms:

```rust
// Wilson Score - Bayesian confidence for rankings
pub fn wilson_score(helpful: u32, total: u32, confidence: f64) -> f64

// Bellman Propagation - Spreads utility to similar episodes
pub fn propagate_utility(episodes: &[Episode], similarity_matrix: &[f64]) -> Vec<f64>

// Exponential Decay - Time-based utility degradation
pub fn decay_factor(days: f64, rate: f64) -> f64
```

### smelt-cli

Command-line interface built with clap:

- `smelt init` - Initialize Smelt in a repository
- `smelt intent create/list/show` - Intent management
- `smelt status` - Show semantic changes
- `smelt validate` - Run validation pipeline
- `smelt commit` - Commit with semantic delta capture
- `smelt memory search/feedback` - Memory operations
- `smelt sync` - Recover from direct git commits
- `smelt doctor` - Diagnostics and repair
- `smelt backup create/restore` - Data backup

### smelt-api

REST API server built with axum:

- Health check and status endpoints
- Intent CRUD operations
- Delta retrieval
- Validation endpoint
- Memory search and feedback

## Data Flow

### Intent → Commit Flow

```
1. Create Intent
   └─> Store in SQLite
   └─> Capture baseline snapshot

2. Make Code Changes
   └─> Developer modifies files

3. Compute Delta
   └─> SmeltGraph diff between snapshots
   └─> Generate SemanticChange list
   └─> Calculate ImpactSummary

4. Validate
   └─> Check breaking changes
   └─> Verify layer boundaries
   └─> Enforce complexity limits
   └─> Validate intent constraints

5. Commit
   └─> Store delta in SQLite
   └─> Create git commit
   └─> Update intent status
   └─> Capture memory episode
```

### Memory Retrieval Flow

```
1. Query
   └─> Generate embedding for query text

2. Search
   └─> Vector similarity search
   └─> Retrieve candidate episodes

3. Rank
   └─> Apply Wilson score (confidence)
   └─> Apply time decay
   └─> Combine similarity + utility

4. Return
   └─> Sorted RankedEpisode list
```

## Storage

### SQLite Schema

**intents table**:
- id, created_at, author info
- goal, rationale, constraints (JSON)
- status, status_data
- baseline_snapshot_id

**deltas table**:
- id, intent_id, timestamp
- from_snapshot, to_snapshot
- changes (JSON), impact_summary (JSON)

**episodes table** (in memory.db):
- id, project, summary, task_type
- outcome, files_modified, tags
- utility, helpful_count, feedback_count

### Vector Storage

Episodes are embedded using BGE-Small-EN-v1.5 and stored as 384-dimensional vectors for semantic similarity search.

## Error Handling

Smelt uses a unified error type with recovery suggestions:

```rust
pub enum SmeltError {
    NotInitialized,           // Run 'smelt init'
    EmptyRepository,          // Need at least one commit
    DatabaseCorrupted(String), // Run 'smelt doctor --fix'
    UncommittedChanges,       // Commit or stash changes
    // ... etc
}

impl SmeltError {
    pub fn is_recoverable(&self) -> bool;
    pub fn suggestion(&self) -> Option<&'static str>;
}
```

## Testing

- **Unit Tests**: Per-crate tests for individual functions
- **Integration Tests**: End-to-end workflow tests in `tests/integration/`
- **Benchmarks**: Performance tests with criterion

Run tests:
```bash
cargo test --workspace              # All tests
cargo test -p smelt-integration-tests  # Integration only
cargo bench --workspace             # Benchmarks
```
