# Product Requirements Document
# Smelt: Semantic Version Control System

**Version:** 1.2
**Date:** January 25, 2026
**Status:** Initial Development
**Classification:** Internal
**Revision Notes:** v1.2 - Deep architectural review of all components; revised integration strategies based on actual code analysis; MemRL changed to "build inspired-by" approach; added codegraph-vscode as Phase 6 foundation  

---

## Executive Summary

Smelt is a next-generation semantic version control system designed for AI-native development workflows. Unlike traditional VCS that tracks text changes, Smelt tracks *meaning*—capturing intent, semantic deltas, architectural constraints, and decision context as first-class citizens.

The system layers over Git (not replacing it), enabling incremental adoption while providing capabilities that Git cannot: semantic merge conflict detection, architectural validation, contextual memory retrieval, and AI-native code review.

**Core Thesis:** As AI agents become primary code authors, the version control paradigm must shift from "what lines changed" to "what meaning changed and does it preserve architectural intent."

> **v1.2 Key Updates (Deep Architectural Review):**
> - **CodeGraph**: ✅ Integrate as library + build SmeltGraph wrapper for snapshot/diff (~3-4 weeks)
> - **Crucible**: ⚠️ Integrate validator only + build adapter layer; cannot validate semantic deltas (~3 weeks)
> - **MemRL**: ❌ Too coupled to Claude Code; **build Smelt-native memory system** inspired by MemRL patterns (~4-5 weeks)
> - **codegraph-vscode**: Discovered existing VSCode extension; use as **Phase 6 foundation**
> - **Timeline revised** to ~18-19 weeks (more realistic based on actual integration complexity)
> - **Critical path**: CodeGraph snapshot/diff + Smelt semantic delta validator

---

## Table of Contents

1. [Vision & Goals](#1-vision--goals)
2. [Problem Statement](#2-problem-statement)
3. [System Architecture](#3-system-architecture)
4. [Component Specifications](#4-component-specifications)
   - 4.1 [CodeGraph Integration](#41-codegraph-integration)
   - 4.2 [Crucible: Architectural Validation](#42-crucible-architectural-validation)
   - 4.3 [SmeltMemory: Contextual Memory](#43-smeltmemory-contextual-memory-system)
   - 4.4 [Smelt Core: Orchestration](#44-smelt-core-orchestration)
5. [Data Models](#5-data-models)
6. [API Specifications](#6-api-specifications)
7. [Development Phases](#7-development-phases)
8. [Technical Requirements](#8-technical-requirements)
9. [Success Metrics](#9-success-metrics)
10. [Future Scope: Cupola Integration](#10-future-scope-cupola-integration)
11. [Risks & Mitigations](#11-risks--mitigations)
12. [Operational Considerations](#12-operational-considerations)
13. [Appendices](#13-appendices)

---

## 1. Vision & Goals

### 1.1 Product Vision

Smelt transforms version control from a recording system to a contract system. Every change is:
- **Intentional**: Preceded by a declared intent with constraints
- **Semantic**: Tracked at the meaning level, not text level
- **Architecturally Sound**: Validated against executable specifications
- **Contextual**: Enriched with relevant historical decisions

### 1.2 Strategic Goals

| Goal | Description | Success Indicator |
|------|-------------|-------------------|
| **G1** | Enable semantic code review | 60% reduction in PR review time |
| **G2** | Prevent architectural violations | 80% of violations caught pre-commit |
| **G3** | Preserve decision context | Relevant memories surfaced for 70% of intents |
| **G4** | Support AI-native workflows | AI agents can commit with validated semantic deltas |
| **G5** | Maintain Git compatibility | 100% backward compatible with Git workflows |

### 1.3 Design Principles

1. **Layer Over Git**: Never replace Git—enhance it
2. **Explicit Over Implicit**: No magic, no hidden behavior
3. **Semantic First**: Meaning is the primary abstraction
4. **Incremental Adoption**: Value at every adoption stage
5. **Test Everything**: If it's not tested, it's broken

### 1.4 Explicit Non-Goals (V1)

To maintain focus, the following are **out of scope** for initial release:

| Non-Goal | Rationale |
|----------|-----------|
| Real-time IDE tracking | Requires deep IDE integration; on-demand analysis is sufficient |
| Behavioral/contract analysis | Requires AI/theorem provers; AST-level analysis only in V1 |
| Semantic merge tooling | Complex UX problem; focus on detection first |
| Multi-repository support | Single-repo first; federation is future scope |
| Web UI | CLI and API first; UI can be built on API later |
| Windows support | Linux/macOS first; Windows adds complexity |

---

## 2. Problem Statement

### 2.1 Git's Assumptions (Now Broken)

| Git Assumption | AI-Native Reality |
|----------------|-------------------|
| Human-sized commits | AI generates massive, atomic refactors |
| Text diffs are reviewable | 200-file changes are cognitively impossible |
| Sequential collaboration | Multiple AI agents + humans simultaneously |
| Commit messages capture intent | Intent is multi-layered, needs machine-readability |
| Merge conflicts are textual | Real conflicts are semantic (contract violations) |

### 2.2 Current Pain Points

**For Developers:**
- Cannot review AI-generated PRs effectively
- Architecture degrades silently
- Decision rationale lost over time
- Repeated mistakes from forgotten context

**For AI Agents:**
- No understanding of architectural constraints
- No access to historical decision context
- Cannot validate changes semantically
- Feedback loop is slow (wait for CI)

### 2.3 Target Users

1. **Primary**: Engineering teams adopting AI coding assistants
2. **Secondary**: Open source maintainers reviewing AI-generated PRs
3. **Tertiary**: Developer tools builders needing semantic code analysis

---

## 3. System Architecture

### 3.1 High-Level Architecture

> **Integration Strategy (v1.2 - Based on Deep Architectural Review):**
>
> | Component | Strategy | Rationale |
> |-----------|----------|-----------|
> | **CodeGraph** | ✅ Integrate + Extend | Library-ready; add SmeltGraph wrapper for snapshot/diff |
> | **Crucible** | ⚠️ Validator Only + Adapter | Skip Parser; build Project from CodeGraph; add semantic delta validation |
> | **MemRL** | 🆕 Build Inspired-By | Too coupled to Claude Code; build Smelt-native using MemRL's proven algorithms |
> | **Smelt Core** | 🆕 New Orchestration | Primary development effort; ties everything together |
> | **codegraph-vscode** | 📋 Phase 6 Foundation | Existing VSCode extension; proven architecture for IDE integration |

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           USER INTERFACES                               │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐   │
│  │    CLI      │  │   VSCode    │  │   Web UI    │  │  Agent API  │   │
│  │  (smelt)  │  │  Extension  │  │  (Future)   │  │   (REST)    │   │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘   │
└─────────┼────────────────┼────────────────┼────────────────┼───────────┘
          │                │                │                │
          └────────────────┴────────────────┴────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                         SMELT CORE                                    │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                    Orchestration Engine                          │   │
│  │  • Intent lifecycle management                                   │   │
│  │  • Semantic delta coordination                                   │   │
│  │  • Validation pipeline execution                                 │   │
│  │  • Git integration layer                                         │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                    │                                    │
│          ┌─────────────────────────┼─────────────────────────┐         │
│          │                         │                         │         │
│          ▼                         ▼                         ▼         │
│  ┌───────────────┐       ┌───────────────┐       ┌───────────────┐    │
│  │   CodeGraph   │       │    Crucible   │       │  SmeltMemory  │    │
│  │  (Semantic    │       │ (Architectural│       │  (Contextual  │    │
│  │   Analysis)   │       │  Validation)  │       │    Memory)    │    │
│  └───────┬───────┘       └───────┬───────┘       └───────┬───────┘    │
│          │                       │                       │            │
└──────────┼───────────────────────┼───────────────────────┼────────────┘
           │                       │                       │
           ▼                       ▼                       ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                          STORAGE LAYER                                  │
│  ┌───────────────┐  ┌───────────────┐  ┌───────────────┐               │
│  │   RocksDB     │  │   SQLite      │  │  Vector DB    │               │
│  │ (Code Graph)  │  │ (Intents,     │  │ (Embeddings)  │               │
│  │               │  │  Deltas)      │  │               │               │
│  └───────────────┘  └───────────────┘  └───────────────┘               │
│                                                                         │
│  Note: Architecture definitions (crucible.yaml) stored as YAML files   │
│        in the repository, versioned with Git.                          │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                            GIT LAYER                                    │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │  Git Repository (Source of Truth for Files)                      │   │
│  │  • Commits generated from semantic deltas                        │   │
│  │  • Branches, merges work normally                                │   │
│  │  • Push to GitHub/GitLab unchanged                               │   │
│  └─────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────┘
```

### 3.2 Data Flow

Smelt supports two workflows: **Intent-First** (planned changes) and **Code-First** (exploratory development).

#### Workflow A: Intent-First (Recommended for AI Agents)

```
                    ┌──────────────────┐
                    │  Developer/Agent │
                    │  declares intent │
                    └────────┬─────────┘
                             │
                             ▼
┌──────────────────────────────────────────────────────────────┐
│                    1. INTENT PHASE                           │
│  • Create Intent Record                                      │
│  • SmeltMemory retrieves relevant memories                   │
│  • Crucible identifies applicable constraints                │
│  • ⚡ BASELINE SNAPSHOT captured from current graph state    │
└────────────────────────────┬─────────────────────────────────┘
                             │
                             ▼
┌──────────────────────────────────────────────────────────────┐
│                    2. IMPLEMENTATION PHASE                   │
│  • Developer/Agent makes code changes                        │
│  • Changes tracked via file system (no real-time parsing)    │
└────────────────────────────┬─────────────────────────────────┘
                             │
                             ▼
┌──────────────────────────────────────────────────────────────┐
│                    3. VALIDATION PHASE (on `smelt validate`) │
│  • CodeGraph parses changed files (on-demand, not real-time) │
│  • Semantic Delta computed: baseline snapshot vs current     │
│  • Crucible validates against architecture                   │
│  • SmeltMemory checks for pattern violations                 │
└────────────────────────────┬─────────────────────────────────┘
```

#### Workflow B: Code-First (For Exploratory Development)

```
                    ┌──────────────────┐
                    │  Developer codes │
                    │  (no intent yet) │
                    └────────┬─────────┘
                             │
                             ▼
┌──────────────────────────────────────────────────────────────┐
│                    1. IMPLEMENTATION PHASE                   │
│  • Developer makes code changes normally                     │
│  • Uses git add/status as usual                              │
└────────────────────────────┬─────────────────────────────────┘
                             │
                             ▼
┌──────────────────────────────────────────────────────────────┐
│                    2. ANALYSIS PHASE (on `smelt status`)     │
│  • Baseline = last committed graph state (HEAD)              │
│  • CodeGraph parses changed files                            │
│  • Semantic Delta computed and displayed                     │
│  • System suggests intent based on changes                   │
└────────────────────────────┬─────────────────────────────────┘
                             │
                             ▼
┌──────────────────────────────────────────────────────────────┐
│                    3. RETROACTIVE INTENT                     │
│  • `smelt commit --goal "description"` creates intent inline │
│  • Or `smelt intent create` then `smelt commit --intent ID`  │
│  • Validation runs, then commit proceeds                     │
└──────────────────────────────────────────────────────────────┘
```

#### Validation & Commit (Both Workflows)

```
                      ┌──────┴──────┐
                      │  Valid?     │
                      └──────┬──────┘
                    ┌────────┴────────┐
                    │                 │
                    ▼                 ▼
           ┌─────────────┐   ┌─────────────┐
           │   REJECT    │   │   ACCEPT    │
           │ (with       │   │             │
           │  feedback)  │   │             │
           └─────────────┘   └──────┬──────┘
                                    │
                                    ▼
┌──────────────────────────────────────────────────────────────┐
│                    4. COMMIT PHASE                           │
│  • Generate Git commit from semantic delta                   │
│  • Store Intent Record in Smelt DB                           │
│  • Store Semantic Delta linked to git SHA                    │
│  • SmeltMemory captures decision episode                     │
└──────────────────────────────────────────────────────────────┘
```

#### Baseline Snapshot Strategy

| Scenario | Baseline Source |
|----------|-----------------|
| Intent created, no changes yet | Current graph state at intent creation |
| Intent created, changes in progress | Snapshot stored when intent was created |
| No intent (code-first) | Last committed state (HEAD) |
| Recovery after crash | Rebuild from last committed state |

---

## 4. Component Specifications

### 4.1 CodeGraph Integration

**Source:** [github.com/anvanster/codegraph-monorepo](https://github.com/anvanster/codegraph-monorepo)
**Version:** 0.2.0 (existing, v0.3 in development with complexity metrics)
**Status:** ✅ Stable, ready for integration (85% alignment with Smelt requirements)

#### 4.1.1 Current Capabilities

CodeGraph provides:
- Graph database optimized for code relationships (RocksDB backend)
- **14 production-ready parsers**: Python, Rust, TypeScript/JavaScript, Go, C, C++, PHP, Java, Kotlin, Ruby, C#, Swift
- Unified `CodeParser` trait across all languages
- Relationship tracking: calls, imports, contains, implements, extends
- Export formats: DOT, JSON, CSV, RDF
- **Cyclomatic complexity metrics** (v0.3.0): Branches, loops, nesting depth, letter grading (A-F)
- Sub-millisecond query performance (node lookup ~7ns, BFS ~5ms)
- 421+ tests with ~90% coverage on parsers

#### 4.1.1a Integration Strategy: INTEGRATE + EXTEND (v1.2)

**Decision:** Use CodeGraph as a direct library dependency with a wrapper layer for Smelt-specific needs.

**Why This Works:**
- ✅ Clean library design with stable public API (`codegraph = "0.2"`)
- ✅ Parser trait is well-abstracted; 14 languages ready
- ✅ RocksDB backend is production-proven
- ✅ No fork required; extensions are orthogonal to core
- ✅ 421+ tests provide confidence

**What Smelt Must Build (SmeltGraph Wrapper):**

```rust
/// Smelt's wrapper around CodeGraph for versioning and delta computation
pub struct SmeltGraph {
    inner: CodeGraph,                              // Core graph (CodeGraph library)
    snapshots: BTreeMap<CommitSha, GraphSnapshot>, // Version history
    temporal_index: TemporalIndex,                 // "State at commit X" queries
    intent_map: HashMap<IntentId, Vec<NodeId>>,   // Intent → affected nodes
}

impl SmeltGraph {
    /// Capture graph state at intent boundary
    pub fn snapshot_for_intent(&mut self, intent: &Intent) -> Result<SnapshotId>;

    /// Compute semantic delta between two states
    pub fn compute_delta(&self, from: CommitSha, to: CommitSha) -> Result<SemanticDelta>;

    /// Query graph state at specific commit
    pub fn query_at_commit(&self, commit: CommitSha) -> Result<&GraphSnapshot>;

    // Pass-through to CodeGraph for standard operations
    pub fn add_node(&mut self, ...) -> Result<NodeId> { self.inner.add_node(...) }
}
```

**Estimated Effort:**
| Component | Lines of Code | Time |
|-----------|---------------|------|
| SmeltGraph wrapper | ~500-700 | 1.5 weeks |
| Graph diff algorithm | ~150 | 3 days |
| Temporal index | ~300 | 1 week |
| Integration tests | ~400 | 1 week |
| **Total** | ~1350-1550 | **3-4 weeks** |

**Risk:** LOW - CodeGraph is stable, well-tested, and actively maintained.

#### 4.1.2 Integration Points

> **Note (v1.2):** See SmeltGraph definition in Section 4.1.1a for the complete wrapper API. The code below shows how SmeltGraph uses CodeGraph internally.

```rust
// SmeltGraph internally uses CodeGraph library
use codegraph::{CodeGraph, NodeType, EdgeType};
use codegraph_parser_api::CodeParser;

impl SmeltGraph {
    /// Parse changed files and update inner CodeGraph
    pub fn track_changes(&mut self, changed_files: &[PathBuf]) -> Result<()> {
        for file in changed_files {
            let parser = self.parser_for_file(file)?;
            parser.parse_file(file, &mut self.inner)?;
        }
        Ok(())
    }
}
```

#### 4.1.3 Required Extensions

| Extension | Priority | Status | Description |
|-----------|----------|--------|-------------|
| Snapshot/Diff | P0 | **To Build** | Compare graph states to produce semantic deltas. Not yet in CodeGraph—must be implemented for Smelt. |
| Signature Tracking | P0 | **To Build** | Detect breaking vs non-breaking signature changes |
| Incremental Parse | P1 | Roadmap v0.6+ | Update graph without full reparse. On CodeGraph roadmap for medium-term. |
| Complexity Metrics | P2 | **✅ Complete** | Track cyclomatic complexity changes. Implemented in CodeGraph v0.3.0. |

> **Note:** CodeGraph v0.3.0 added cyclomatic complexity calculation with letter grading (A-F), branches, loops, logical operators, and nesting depth tracking. All 14 language parsers support this feature.

> **Out of Scope (Future Research):** Contract extraction (pre/post conditions) and behavioral analysis require AI/symbolic execution capabilities beyond static AST analysis. These are explicitly deferred.

#### 4.1.4 Semantic Delta Structure

```rust
pub struct SemanticDelta {
    pub id: Uuid,
    pub intent_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub changes: Vec<SemanticChange>,
    pub impact_summary: ImpactSummary,
}

pub enum SemanticChange {
    // Function changes
    FunctionAdded { id: NodeId, signature: FunctionSignature },
    FunctionRemoved { id: NodeId, signature: FunctionSignature },
    SignatureChanged { 
        id: NodeId, 
        before: FunctionSignature, 
        after: FunctionSignature,
        breaking: bool,  // Determined by: added required params, changed return type, etc.
    },
    BodyChanged {
        id: NodeId,
        // NOTE: We track that the body changed, not *how* behavior changed.
        // Behavioral analysis requires AI/symbolic execution (future scope).
        lines_added: usize,
        lines_removed: usize,
        complexity_delta: Option<i32>,  // Cyclomatic complexity change
    },
    
    // Dependency changes
    DependencyAdded { from: NodeId, to: NodeId, kind: EdgeType },
    DependencyRemoved { from: NodeId, to: NodeId, kind: EdgeType },
    
    // Structural changes
    ModuleAdded { path: String },
    ModuleRemoved { path: String },
    ClassAdded { id: NodeId, name: String },
    ClassRemoved { id: NodeId, name: String },
    
    // Visibility changes
    VisibilityChanged { id: NodeId, before: Visibility, after: Visibility },
}

pub struct ImpactSummary {
    pub files_affected: usize,
    pub functions_modified: usize,
    pub breaking_changes: usize,
    pub new_dependencies: usize,
    pub layers_affected: Vec<String>,
}
```

---

### 4.2 Crucible: Architectural Validation

**Source:** [github.com/anvanster/crucible](https://github.com/anvanster/crucible)
**Version:** 0.1.11 (existing)
**Status:** ⚠️ Validator usable; requires adapter layer and supplemental validation
**Priority:** P0 (Core feature)

#### 4.2.1 Purpose

Crucible transforms architecture from documentation into executable specifications. It defines, validates, and enforces structural constraints on the codebase.

#### 4.2.1a Integration Strategy: VALIDATOR ONLY + ADAPTER (v1.2)

**Decision:** Use Crucible's validator directly but skip its file-based Parser; build Project from CodeGraph data programmatically. Add Smelt-specific semantic delta validation.

**Why This Approach:**
- ✅ Crucible's `Validator` is well-decoupled from file I/O
- ✅ Can construct `Project` struct programmatically from CodeGraph
- ⚠️ Crucible's `Parser` is file-system-bound (reads `.crucible/*.json`)—skip it
- ⚠️ Crucible validates **final state only**, not semantic deltas
- ❌ Cannot validate: breaking signature changes, new public API, complexity increases

**Critical Limitation:** Crucible validates architecture rules (layers, cycles, dependencies) but **cannot validate semantic intent constraints**. Smelt must build its own semantic delta validator on top.

**What Crucible CAN Validate:**
| Validation | Status | Notes |
|------------|--------|-------|
| Circular dependencies | ✅ | Graph-based cycle detection via petgraph |
| Layer boundary violations | ✅ | File-path → layer mapping |
| Type existence | ✅ | Cross-reference validation |
| Call target resolution | ✅ | Function/method resolution |
| Dependency tracking | ✅ | Declared vs actual matching |

**What Smelt MUST Build (Semantic Delta Validator):**
| Validation | Status | Notes |
|------------|--------|-------|
| Breaking signature changes | 🆕 To Build | Compare function signatures before/after |
| New public API detection | 🆕 To Build | Detect visibility changes |
| Complexity threshold enforcement | 🆕 To Build | Use CodeGraph metrics |
| Intent constraint validation | 🆕 To Build | Map intent to semantic rules |

**Integration Architecture:**
```rust
/// Smelt's validation pipeline (combines Crucible + custom validation)
pub struct SmeltValidator {
    crucible: CrucibleValidator,      // Crucible's validator (architecture rules)
    semantic: SemanticDeltaValidator, // Smelt-built (intent constraints)
}

impl SmeltValidator {
    pub fn validate(&self, delta: &SemanticDelta, intent: &Intent) -> ValidationResult {
        // 1. Build Crucible Project from current CodeGraph state
        let project = self.build_project_from_graph()?;

        // 2. Run Crucible architectural validation
        let arch_result = self.crucible.validate(&project);
        if !arch_result.valid {
            return ValidationResult::reject(arch_result.errors);
        }

        // 3. Run Smelt semantic delta validation (Crucible can't do this)
        let semantic_result = self.semantic.validate_delta(delta, intent);
        if !semantic_result.valid {
            return ValidationResult::reject(semantic_result.violations);
        }

        ValidationResult::accept()
    }
}
```

**Estimated Effort:**
| Component | Lines of Code | Time |
|-----------|---------------|------|
| YAML↔JSON adapter | ~200 | 3 days |
| CodeGraph → Project builder | ~300 | 1 week |
| Crucible integration | ~150 | 3 days |
| Semantic delta validator | ~600-800 | 1.5 weeks |
| Integration tests | ~400 | 1 week |
| **Total** | ~1650-1850 | **~3 weeks** |

**Schema Format Decision:** Use **JSON** (Crucible's native format). YAML examples in this PRD are for documentation clarity; actual implementation uses JSON with optional YAML import for user convenience.

#### 4.2.1b Current Capabilities (Crucible v0.1.11)

| Feature | Status | Notes |
|---------|--------|-------|
| Circular dependency detection | ✅ Complete | Graph-based cycle detection |
| Layer boundary enforcement | ✅ Complete | File-path based layer validation |
| Type existence validation | ✅ Complete | Cross-reference validation |
| Call target validation | ✅ Complete | Function/method resolution |
| Dependency usage validation | ✅ Complete | Declared vs actual matching |
| Unused dependency detection | ✅ Complete | Warning-level detection |
| CLI with 8 slash commands | ✅ Complete | Claude Code integration |
| Compliance frameworks | ✅ Complete | HIPAA, PCI-DSS, SOC2 |
| 70 tests passing | ✅ Complete | 100% pass rate |

#### 4.2.2 Architecture Definition Schema

> **Note:** The examples below show the target YAML format for Smelt. Crucible currently uses JSON (`.crucible/modules/*.json`). An adapter layer will translate between formats. See Section 4.2.1b for integration details.

```yaml
# crucible.yaml - Architecture definition file (Smelt format)
# Adapter translates to Crucible's JSON format internally
version: "1.0"
name: "my-project"

# Layer definitions
layers:
  - name: api_gateway
    paths:
      - "src/api/**"
      - "src/routes/**"
    allowed_dependencies:
      - domain
      - infrastructure.cache
    prohibited_dependencies:
      - infrastructure.database
    constraints:
      - public_functions_require: openapi_annotation
      - max_cyclomatic_complexity: 10
    
  - name: domain
    paths:
      - "src/domain/**"
      - "src/services/**"
    allowed_dependencies:
      - infrastructure
    prohibited_dependencies:
      - api_gateway  # No upward dependencies
    constraints:
      - no_side_effects_in_pure_functions: true
      
  - name: infrastructure
    paths:
      - "src/infra/**"
    sublayers:
      - name: database
        paths: ["src/infra/db/**"]
      - name: cache
        paths: ["src/infra/cache/**"]
      - name: external
        paths: ["src/infra/external/**"]

# Global rules
rules:
  - name: no_circular_dependencies
    type: acyclicity
    scope: layers
    
  - name: public_api_stability
    type: breaking_change_detection
    scope: 
      - "src/api/public/**"
    severity: error
    
  - name: test_coverage_minimum
    type: coverage
    threshold: 80
    scope: domain
    severity: warning

# Custom validators (plugin system)
validators:
  - name: openapi_annotation
    plugin: crucible-openapi
    config:
      spec_file: "openapi.yaml"
```

#### 4.2.3 Core Data Structures

```rust
// Crucible core types
pub struct ArchitectureDefinition {
    pub version: String,
    pub name: String,
    pub layers: Vec<Layer>,
    pub rules: Vec<Rule>,
    pub validators: Vec<ValidatorConfig>,
}

pub struct Layer {
    pub name: String,
    pub paths: Vec<GlobPattern>,
    pub allowed_dependencies: Vec<LayerRef>,
    pub prohibited_dependencies: Vec<LayerRef>,
    pub constraints: Vec<Constraint>,
    pub sublayers: Option<Vec<Layer>>,
}

pub struct Rule {
    pub name: String,
    pub rule_type: RuleType,
    pub scope: Scope,
    pub severity: Severity,
    pub config: Option<serde_json::Value>,
}

pub enum RuleType {
    Acyclicity,
    BreakingChangeDetection,
    Coverage,
    Complexity,
    Custom(String),
}

pub enum Severity {
    Error,   // Blocks commit
    Warning, // Allows commit with notice
    Info,    // Informational only
}
```

#### 4.2.4 Validation Engine

```rust
pub struct CrucibleEngine {
    definition: ArchitectureDefinition,
    validators: HashMap<String, Box<dyn Validator>>,
}

impl CrucibleEngine {
    /// Load architecture definition from file
    pub fn load(path: &Path) -> Result<Self>;
    
    /// Validate a semantic delta against architecture
    pub fn validate(&self, delta: &SemanticDelta, graph: &CodeGraph) -> ValidationResult {
        let mut violations = Vec::new();
        
        // Check layer boundary violations
        for change in &delta.changes {
            if let SemanticChange::DependencyAdded { from, to, .. } = change {
                if let Some(v) = self.check_layer_boundary(from, to, graph) {
                    violations.push(v);
                }
            }
        }
        
        // Check rule violations
        for rule in &self.definition.rules {
            violations.extend(self.check_rule(rule, delta, graph));
        }
        
        // Run custom validators
        for (name, validator) in &self.validators {
            violations.extend(validator.validate(delta, graph));
        }
        
        ValidationResult { violations }
    }
    
    /// Check if a dependency crosses prohibited layer boundaries
    fn check_layer_boundary(
        &self,
        from: &NodeId,
        to: &NodeId,
        graph: &CodeGraph,
    ) -> Option<Violation>;
}

pub struct ValidationResult {
    pub violations: Vec<Violation>,
}

impl ValidationResult {
    pub fn has_errors(&self) -> bool {
        self.violations.iter().any(|v| v.severity == Severity::Error)
    }
    
    pub fn blocking_violations(&self) -> Vec<&Violation> {
        self.violations.iter().filter(|v| v.severity == Severity::Error).collect()
    }
}

pub struct Violation {
    pub rule: String,
    pub severity: Severity,
    pub message: String,
    pub location: Option<Location>,
    pub suggestion: Option<String>,
    pub documentation_link: Option<String>,
}
```

#### 4.2.5 CLI Interface

```bash
# Initialize architecture definition
$ smelt crucible init
Created crucible.yaml with default layer structure

# Validate current state
$ smelt crucible validate
✅ All 12 architectural rules passed

# Validate a semantic delta
$ smelt crucible check --delta delta_abc123
❌ 2 violations found:

  ERROR: Layer boundary violation
  File: src/api/routes/users.rs:45
  Rule: api_gateway cannot depend on infrastructure.database
  Found: api.routes.users → infra.db.connection
  Suggestion: Use domain.services.user_service instead
  Docs: crucible://rules/layer-boundaries

  WARNING: Cyclomatic complexity exceeded
  File: src/domain/services/auth.rs:120
  Rule: max_cyclomatic_complexity = 10
  Found: authenticate() has complexity 14
  Suggestion: Extract validation logic to separate function

# Generate architecture diagram
$ smelt crucible diagram --output arch.svg
Generated architecture diagram: arch.svg
```

---

### 4.3 SmeltMemory: Contextual Memory System

**Inspiration:** [github.com/anvanster/MemRL](https://github.com/anvanster/MemRL)
**Strategy:** 🆕 Build Smelt-native (inspired by MemRL's proven algorithms)
**Status:** New development
**Priority:** P0 (Core feature)

#### 4.3.1 Purpose

SmeltMemory captures, stores, and retrieves decision context—the "why" behind code changes. It enables institutional knowledge to persist across team changes and surfaces relevant historical context for new decisions.

#### 4.3.1a Integration Strategy: Build Smelt-Native (Inspired by MemRL)

> **Deep Architectural Review Finding (v1.2):**
>
> MemRL is a fully-implemented memory-augmented RL system with excellent algorithms, but it is **too tightly coupled to Claude Code** for direct integration into Smelt:
>
> | Issue | Impact | Resolution |
> |-------|--------|------------|
> | MCP-only API | No programmatic library interface | Build native Rust library |
> | Hardcoded paths | `~/.claude/memory/` not configurable | Smelt-managed storage |
> | Claude Code session model | Assumes `claude.md` hooks | Generic session abstraction |
> | Node.js runtime | Different stack from Smelt (Rust) | Pure Rust implementation |
>
> **Valuable Algorithms to Port:**
> - **Bellman propagation** - Utility value spreading through episode graph
> - **Wilson score** - Bayesian confidence for episode helpfulness
> - **Exponential decay** - Time-based utility degradation
> - **LanceDB embeddings** - Vector similarity search with BGE-Small
>
> **Build Timeline:** ~4-5 weeks

#### 4.3.1b SmeltMemory Architecture

```rust
/// Smelt-native memory system inspired by MemRL's proven algorithms
pub struct SmeltMemory {
    storage: MemoryStorage,           // LanceDB + SQLite (Smelt-managed)
    embedder: Box<dyn Embedder>,      // BGE-Small-EN-v1.5 (same as MemRL)
    ranker: UtilityRanker,            // Wilson score + Bellman propagation
    session: SessionTracker,          // Generic session (not Claude-specific)
}

impl SmeltMemory {
    /// Create memory from completed intent
    pub fn capture_from_intent(&mut self, intent: &Intent, outcome: &Outcome) -> Result<EpisodeId>;

    /// Retrieve relevant memories for new intent (with utility ranking)
    pub fn retrieve_for_intent(&self, intent: &Intent, limit: usize) -> Vec<RankedEpisode>;

    /// Record feedback (feeds into Wilson score + Bellman propagation)
    pub fn record_feedback(&mut self, episode_id: EpisodeId, helpful: bool) -> Result<()>;

    /// Run utility propagation (ported from MemRL)
    pub fn propagate_utility(&mut self, temporal: bool) -> Result<PropagationStats>;
}

/// Episode structure (simplified from MemRL)
pub struct Episode {
    pub id: EpisodeId,
    pub intent_summary: String,
    pub context: EpisodeContext,
    pub outcome: EpisodeOutcome,
    pub utility: UtilityScore,        // Wilson score + temporal decay
    pub embedding: Vec<f32>,          // For semantic search
    pub created_at: DateTime<Utc>,
}

/// Utility scoring (ported from MemRL's Phase 3)
pub struct UtilityScore {
    pub raw_score: f32,               // Bellman-propagated value
    pub confidence: f32,              // Wilson score (handles sparse feedback)
    pub decay_factor: f32,            // Exponential time decay
    pub feedback_count: u32,          // Total feedback events
}
```

#### 4.3.1c Development Tasks

| Task | Priority | Effort | Description |
|------|----------|--------|-------------|
| Core storage layer | P0 | 1 week | LanceDB vectors + SQLite metadata (Smelt-managed paths) |
| Episode capture | P0 | 3 days | Intent → Episode transformation with embedding |
| Semantic retrieval | P0 | 4 days | Vector similarity + utility ranking |
| Wilson score | P1 | 3 days | Port Bayesian confidence calculation from MemRL |
| Bellman propagation | P1 | 4 days | Port utility spreading algorithm from MemRL |
| Temporal decay | P1 | 2 days | Port exponential decay with configurable half-life |
| Feedback loop | P0 | 2 days | Feedback capture and score updates |

> **Why Not Integrate MemRL Directly?**
>
> MemRL's excellent algorithms are embedded in a Claude Code-specific MCP server architecture. Extracting them as a library would require significant refactoring of MemRL itself. Building Smelt-native allows:
> 1. Pure Rust implementation (no Node.js dependency)
> 2. Smelt-managed storage (integrates with `.smelt/` directory)
> 3. Direct API integration (no MCP overhead)
> 4. Intent-native data model (not adapted from Claude Code sessions)

#### 4.3.2 Memory Types

> **Implementation Note (v1.2):** The types below represent SmeltMemory's data model. Since we're building Smelt-native (not integrating MemRL), these types are the authoritative design. The Episode structure in Section 4.3.1b provides the core storage model; these MemoryTypes provide the semantic categorization layer.

```rust
pub enum MemoryType {
    /// Decision episode: why a particular approach was chosen
    DecisionEpisode {
        topic: String,
        trigger: String,
        options_considered: Vec<OptionConsidered>,
        selected_option: String,
        rationale: String,
        constraints_discovered: Vec<String>,
        participants: Vec<String>,
    },
    
    /// Pattern learned: reusable insight from experience
    PatternLearned {
        pattern_name: String,
        description: String,
        context: String,
        applicability: Vec<String>,
        counter_examples: Vec<String>,
    },
    
    /// Failure record: what went wrong and why
    FailureRecord {
        incident_id: Option<String>,
        description: String,
        root_cause: String,
        resolution: String,
        prevention_measures: Vec<String>,
    },
    
    /// Constraint: discovered limitation or requirement
    Constraint {
        name: String,
        description: String,
        source: ConstraintSource,
        applies_to: Vec<String>,
    },
}

pub struct OptionConsidered {
    pub approach: String,
    pub pros: Vec<String>,
    pub cons: Vec<String>,
    pub rejected_because: Option<String>,
    pub selected_because: Option<String>,
}

pub enum ConstraintSource {
    Performance,      // "Must not exceed 50ms latency"
    Security,         // "PII must be encrypted at rest"
    Compliance,       // "GDPR requires consent"
    Technical,        // "Redis cluster doesn't support Lua scripts"
    Business,         // "Must support offline mode"
}
```

#### 4.3.3 Memory Record Structure

```rust
pub struct MemoryRecord {
    pub id: Uuid,
    pub memory_type: MemoryType,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub author: String,
    pub participants: Vec<String>,
    
    // Semantic links
    pub related_intents: Vec<Uuid>,
    pub related_files: Vec<String>,
    pub related_functions: Vec<NodeId>,
    pub tags: Vec<String>,
    
    // Retrieval optimization
    pub embedding: Vec<f32>,
    pub keywords: Vec<String>,
    
    // Quality signals
    pub referenced_count: u32,
    pub last_referenced: Option<DateTime<Utc>>,
    pub relevance_feedback: Vec<RelevanceFeedback>,
}

pub struct RelevanceFeedback {
    pub query_context: String,
    pub was_helpful: bool,
    pub timestamp: DateTime<Utc>,
}
```

#### 4.3.4 Retrieval System

```rust
pub struct SmeltMemoryEngine {
    storage: MemoryStorage,
    embedder: Box<dyn Embedder>,
    ranker: RetrievalRanker,
}

impl SmeltMemoryEngine {
    /// Store a new memory record
    pub fn store(&mut self, memory: MemoryRecord) -> Result<Uuid>;
    
    /// Retrieve relevant memories for an intent
    pub fn retrieve_for_intent(&self, intent: &IntentRecord) -> Vec<RankedMemory> {
        // Phase 1: Semantic similarity (coarse filter)
        let query_embedding = self.embedder.embed(&intent.to_query_text());
        let candidates = self.storage.similarity_search(&query_embedding, k: 20);
        
        // Phase 2: Value-aware ranking (fine filter)
        let ranked = self.ranker.rank(candidates, intent);
        
        // Return top results with relevance scores
        ranked.into_iter().take(5).collect()
    }
    
    /// Retrieve memories related to specific code regions
    pub fn retrieve_for_files(&self, files: &[PathBuf]) -> Vec<RankedMemory>;
    
    /// Update memory based on usage feedback
    pub fn feedback(&mut self, memory_id: Uuid, was_helpful: bool, context: &str);
}

pub struct RetrievalRanker {
    // Weights for ranking factors
    semantic_weight: f32,
    recency_weight: f32,
    usage_weight: f32,
    author_match_weight: f32,
}

impl RetrievalRanker {
    pub fn rank(&self, candidates: Vec<MemoryRecord>, intent: &IntentRecord) -> Vec<RankedMemory> {
        candidates.into_iter()
            .map(|m| {
                let score = 
                    self.semantic_weight * m.semantic_similarity +
                    self.recency_weight * self.recency_score(&m) +
                    self.usage_weight * self.usage_score(&m) +
                    self.author_match_weight * self.author_match(&m, intent);
                RankedMemory { memory: m, score }
            })
            .sorted_by(|a, b| b.score.partial_cmp(&a.score).unwrap())
            .collect()
    }
}

pub struct RankedMemory {
    pub memory: MemoryRecord,
    pub score: f32,
}
```

#### 4.3.5 Memory Capture Triggers

```rust
pub enum CaptureTrigger {
    /// Explicit: User/agent creates memory intentionally
    Explicit {
        source: String,  // "cli", "vscode", "agent"
    },
    
    /// Implicit: System detects memory-worthy event
    Implicit {
        detector: String,
        confidence: f32,
    },
}

pub struct MemoryDetector {
    /// Detect decision-worthy patterns in conversations/commits
    pub fn detect_decision_episode(&self, context: &CaptureContext) -> Option<DecisionEpisode>;
    
    /// Detect learned patterns from repeated code structures
    pub fn detect_pattern(&self, graph: &CodeGraph) -> Vec<PatternLearned>;
    
    /// Extract constraints from code comments and documentation
    pub fn extract_constraints(&self, files: &[PathBuf]) -> Vec<Constraint>;
}
```

#### 4.3.6 CLI Interface

```bash
# Record a decision
$ smelt memory record decision
Topic: Rate limiting strategy
Trigger: API abuse incident INC-2024-012
Options considered:
  1. Token bucket at load balancer [rejected: no user visibility]
  2. Application-level with Redis [selected: user-aware limiting]
Constraints discovered:
  - Must not add >50ms latency
  - Must work across multiple app instances
Saved: mem_abc123

# Search memories
$ smelt memory search "rate limiting"
Found 3 relevant memories:

  mem_abc123 (Decision Episode) - Rate limiting strategy
  Score: 0.92 | 2024-01-15 | @andrey, @sarah
  "Selected application-level Redis approach for user-aware limiting"

  mem_def456 (Failure Record) - Redis connection pool exhaustion
  Score: 0.78 | 2023-11-20 | @ops-team
  "Rate limiter caused connection pool exhaustion under load"

  mem_ghi789 (Constraint) - API latency budget
  Score: 0.65 | 2023-06-01 | @platform-team
  "Public API endpoints must respond within 100ms p99"

# Link memory to current intent
$ smelt memory link mem_abc123 --intent int_current
Linked memory to current intent

# View memory details
$ smelt memory show mem_abc123
```

---

### 4.4 Smelt Core: Orchestration

**Status:** 🆕 New component to build
**Priority:** P0 (Essential glue—this is where the new work happens)

#### 4.4.1 Purpose

Smelt Core orchestrates the interaction between SmeltGraph, SmeltValidator, and SmeltMemory. It manages the intent lifecycle, coordinates validation, and interfaces with Git.

> **Key Insight (v1.2):** Smelt Core's primary role is:
> 1. **SmeltGraph wrapper**: Snapshot/diff functionality around CodeGraph library
> 2. **SmeltValidator orchestration**: Crucible (architecture) + custom semantic delta validation
> 3. **SmeltMemory integration**: Episode capture and retrieval for context
> 4. **Workflow orchestration**: Managing the intent → validate → commit lifecycle
> 5. **CLI/API surface**: Providing the user-facing commands that coordinate everything

#### 4.4.2 Core Data Structures

```rust
pub struct IntentRecord {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub author: Author,
    pub goal: String,
    pub rationale: Option<String>,
    pub constraints: Vec<IntentConstraint>,
    pub context_links: ContextLinks,
    pub status: IntentStatus,
}

pub struct Author {
    pub name: String,
    pub email: String,
    pub author_type: AuthorType,
}

pub enum AuthorType {
    Human,
    Agent { agent_id: String, model: String },
}

pub struct IntentConstraint {
    pub name: String,
    pub constraint_type: ConstraintType,
    pub value: String,
}

pub enum ConstraintType {
    LatencyBudget,
    MemoryBudget,
    Scope,
    NoBreakingChanges,
    Custom(String),
}

pub struct ContextLinks {
    pub memories: Vec<Uuid>,
    pub architecture_refs: Vec<String>,  // crucible://layer/component
    pub related_intents: Vec<Uuid>,
    pub external_refs: Vec<String>,      // JIRA tickets, PRs, etc.
}

pub enum IntentStatus {
    Draft,
    InProgress,
    PendingValidation,
    Validated,
    Committed { git_sha: String },
    Rejected { violations: Vec<Violation> },
}
```

#### 4.4.3 Smelt Engine

```rust
pub struct SmeltEngine {
    graph: SmeltGraph,             // Wraps CodeGraph with snapshot/diff
    validator: SmeltValidator,      // Crucible + semantic delta validation
    memory: SmeltMemory,           // Smelt-native contextual memory
    git: GitInterface,
    storage: SmeltStorage,
}

impl SmeltEngine {
    /// Create a new intent
    pub fn create_intent(&mut self, request: CreateIntentRequest) -> Result<IntentRecord> {
        // 1. Create intent record
        let intent = IntentRecord::new(request);
        
        // 2. Retrieve relevant memories
        let memories = self.memory.retrieve_for_intent(&intent);
        
        // 3. Identify applicable architectural constraints
        let arch_constraints = self.validator.constraints_for_scope(&intent.constraints);
        
        // 4. Store and return enriched intent
        self.storage.store_intent(&intent)?;
        
        Ok(intent.with_context(memories, arch_constraints))
    }
    
    /// Track changes and compute semantic delta
    pub fn track_changes(&mut self, intent_id: Uuid) -> Result<SemanticDelta> {
        let intent = self.storage.get_intent(intent_id)?;
        
        // 1. Get changed files from Git
        let changed_files = self.git.changed_files()?;
        
        // 2. Update SmeltGraph with changes
        let snapshot_before = self.graph.snapshot();
        self.graph.track_changes(&changed_files)?;
        let snapshot_after = self.graph.snapshot();

        // 3. Compute semantic delta
        let delta = self.graph.compute_delta(&snapshot_before, &snapshot_after);
        
        Ok(delta.with_intent(intent_id))
    }
    
    /// Validate semantic delta
    pub fn validate(&self, delta: &SemanticDelta) -> ValidationResult {
        let mut all_violations = Vec::new();
        
        // 1. SmeltValidator architectural + semantic validation
        let arch_result = self.validator.validate(delta, &self.graph);
        all_violations.extend(arch_result.violations);
        
        // 2. SmeltMemory pattern validation (check for known anti-patterns)
        let memory_warnings = self.memory.check_patterns(delta);
        all_violations.extend(memory_warnings);
        
        // 3. Intent constraint validation
        let intent = self.storage.get_intent(delta.intent_id)?;
        let intent_violations = self.validate_intent_constraints(&intent, delta);
        all_violations.extend(intent_violations);
        
        ValidationResult { violations: all_violations }
    }
    
    /// Commit validated changes
    pub fn commit(&mut self, delta: &SemanticDelta) -> Result<CommitResult> {
        // 1. Final validation
        let validation = self.validate(delta);
        if validation.has_errors() {
            return Err(CommitError::ValidationFailed(validation));
        }
        
        // 2. Generate Git commit
        let intent = self.storage.get_intent(delta.intent_id)?;
        let commit_message = self.generate_commit_message(&intent, delta);
        let git_sha = self.git.commit(&commit_message)?;
        
        // 3. Store semantic delta
        self.storage.store_delta(delta)?;
        
        // 4. Capture memory from this commit
        self.memory.capture_from_commit(&intent, delta);
        
        // 5. Update intent status
        self.storage.update_intent_status(
            delta.intent_id,
            IntentStatus::Committed { git_sha: git_sha.clone() },
        )?;
        
        Ok(CommitResult { 
            git_sha, 
            delta_id: delta.id,
            intent_id: delta.intent_id,
        })
    }
}
```

#### 4.4.4 CLI Interface

```bash
# Start a new intent
$ smelt intent create --goal "Add rate limiting to public API"
Created intent: int_abc123

Relevant memories surfaced:
  📝 mem_def456: Previous rate limiting discussion
  ⚠️  mem_ghi789: Redis connection pool incident

Applicable constraints:
  🏛️  api_gateway layer restrictions apply
  ⏱️  Latency budget: 100ms

# Track changes during development
$ smelt status
Intent: int_abc123 (Add rate limiting to public API)
Status: In Progress

Semantic changes detected:
  + function: api.middleware.rate_limit
  + dependency: api.routes → api.middleware.rate_limit
  ~ signature: api.routes.users.get_users (non-breaking)

# Validate before commit
$ smelt validate
⏳ Running validation...

✅ Architectural validation: passed
✅ Intent constraints: passed
⚠️  Memory check: Similar to failed approach in mem_ghi789
   Note: Previous Redis approach caused connection exhaustion
   
1 warning, 0 errors. Ready to commit.

# Commit with semantic delta
$ smelt commit
⏳ Generating commit...

✅ Committed: abc123def
   Intent: int_abc123
   Delta: delta_xyz789
   Files: 5 changed
   Semantic: +2 functions, +3 dependencies, 1 signature change

📝 Memory captured: Rate limiting implementation decision
```

---

## 5. Data Models

### 5.1 Entity Relationship Diagram

```
┌─────────────────┐       ┌─────────────────┐       ┌─────────────────┐
│  IntentRecord   │       │  SemanticDelta  │       │   GitCommit     │
├─────────────────┤       ├─────────────────┤       ├─────────────────┤
│ id (PK)         │──────<│ intent_id (FK)  │       │ sha (PK)        │
│ author          │       │ id (PK)         │>──────│ delta_id (FK)   │
│ goal            │       │ timestamp       │       │ message         │
│ rationale       │       │ changes[]       │       │ author          │
│ constraints[]   │       │ impact_summary  │       │ timestamp       │
│ status          │       └────────┬────────┘       └─────────────────┘
│ created_at      │                │
└────────┬────────┘                │
         │                         │
         │    ┌────────────────────┘
         │    │
         ▼    ▼
┌─────────────────┐       ┌─────────────────┐
│  MemoryRecord   │       │    CodeGraph    │
├─────────────────┤       ├─────────────────┤
│ id (PK)         │       │ Nodes           │
│ memory_type     │       │  - Functions    │
│ content         │       │  - Classes      │
│ embedding[]     │       │  - Modules      │
│ related_intents │       │ Edges           │
│ tags[]          │       │  - Calls        │
│ created_at      │       │  - Imports      │
└─────────────────┘       │  - Contains     │
                          └─────────────────┘
         │
         │
         ▼
┌─────────────────┐
│ ArchDefinition  │
├─────────────────┤
│ layers[]        │
│ rules[]         │
│ validators[]    │
└─────────────────┘
```

### 5.2 Storage Strategy

| Data Type | Storage Backend | Rationale |
|-----------|-----------------|-----------|
| Code Graph | RocksDB | High-performance graph queries (SmeltGraph wraps CodeGraph) |
| Intent Records | SQLite | Relational queries, ACID transactions |
| Semantic Deltas | SQLite + RocksDB | Metadata in SQLite, full deltas in RocksDB |
| Memory Episodes | SQLite + **LanceDB** | Metadata + vector embeddings. SmeltMemory uses LanceDB with model2vec embeddings. |
| Architecture Defs | YAML files (versioned) | Human-editable, Git-tracked. Adapter translates to Crucible's JSON format. |

> **Storage Decisions (v1.2):**
> - **SmeltGraph**: Uses RocksDB (wraps CodeGraph) with snapshot checkpoints
> - **SmeltMemory**: Uses LanceDB for vectors (ported from MemRL architecture)
> - **SmeltValidator**: Uses Crucible's JSON format internally (YAML adapter for user-facing files)

---

## 6. API Specifications

### 6.1 CLI Commands

```
smelt
├── init                # Initialize Smelt in repository
│   ├── --background    # Index in background (default)
│   └── --wait          # Wait for indexing to complete
│
├── status              # Show current state and semantic changes
│   └── --full          # Include indexing progress
│
├── intent
│   ├── create          # Create new intent
│   ├── list            # List intents
│   ├── show <id>       # Show intent details
│   └── link <id>       # Link memory/constraint to intent
│
├── validate            # Validate current changes
│   └── --fix           # Auto-fix simple issues
│
├── commit              # Commit with semantic delta
│   ├── --intent <id>   # Use existing intent
│   ├── --goal "..."    # Create inline intent
│   └── --skip-validation # Bypass validation (not recommended)
│
├── delta
│   ├── show <id>       # Show semantic delta
│   ├── compare         # Compare two deltas
│   └── history         # Show delta history
│
├── crucible
│   ├── init            # Initialize architecture definition
│   ├── validate        # Validate architecture
│   ├── diagram         # Generate architecture diagram
│   └── check           # Check specific delta
│
├── memory
│   ├── record          # Record new memory
│   ├── search          # Search memories
│   ├── show <id>       # Show memory details
│   └── link            # Link memory to intent
│
├── sync                # Recover from direct git commits
│   ├── --analyze       # Show untracked commits
│   └── --interactive   # Choose action per commit
│
├── doctor              # Diagnose and repair issues
│   ├── --repair        # Auto-repair detected issues
│   └── --restore-backup # Restore from backup
│
└── config
    ├── show            # Show current configuration
    └── edit            # Open config in editor
```

### 6.2 Agent API (REST)

```yaml
openapi: 3.0.0
info:
  title: Smelt Agent API
  version: 1.0.0

paths:
  /intents:
    post:
      summary: Create intent
      requestBody:
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/CreateIntentRequest'
      responses:
        '201':
          description: Intent created
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/IntentRecord'

  /intents/{id}/validate:
    post:
      summary: Validate changes for intent
      parameters:
        - name: id
          in: path
          required: true
          schema:
            type: string
      responses:
        '200':
          description: Validation result
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/ValidationResult'

  /intents/{id}/commit:
    post:
      summary: Commit validated changes
      responses:
        '201':
          description: Commit successful
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/CommitResult'

  /memories/search:
    post:
      summary: Search memories
      requestBody:
        content:
          application/json:
            schema:
              type: object
              properties:
                query:
                  type: string
                intent_id:
                  type: string
                limit:
                  type: integer
                  default: 5
      responses:
        '200':
          description: Search results
          content:
            application/json:
              schema:
                type: array
                items:
                  $ref: '#/components/schemas/RankedMemory'

components:
  schemas:
    CreateIntentRequest:
      type: object
      required:
        - goal
      properties:
        goal:
          type: string
        rationale:
          type: string
        constraints:
          type: array
          items:
            $ref: '#/components/schemas/IntentConstraint'
            
    IntentConstraint:
      type: object
      properties:
        name:
          type: string
        constraint_type:
          type: string
          enum: [latency_budget, memory_budget, scope, no_breaking_changes, custom]
        value:
          type: string
          
    ValidationResult:
      type: object
      properties:
        valid:
          type: boolean
        violations:
          type: array
          items:
            $ref: '#/components/schemas/Violation'
            
    Violation:
      type: object
      properties:
        rule:
          type: string
        severity:
          type: string
          enum: [error, warning, info]
        message:
          type: string
        location:
          type: string
        suggestion:
          type: string
```

---

## 7. Development Phases

> **Note:** Timelines assume 2-3 full-time engineers. Adjust based on team capacity.
>
> **REVISED (v1.2 - Deep Architectural Review):**
> - CodeGraph: Integrate as library + build SmeltGraph wrapper (~4 weeks)
> - Crucible: Validator only + adapter layer (~3 weeks) - cannot validate semantic deltas
> - MemRL: **Build Smelt-native** (~4-5 weeks) - existing MemRL too coupled to Claude Code
> - codegraph-vscode: Use as Phase 6 foundation (existing VSCode extension + Rust LSP)

### Timeline Summary

| Phase | v1.0 Estimate | v1.1 Estimate | v1.2 Estimate | Notes |
|-------|---------------|---------------|---------------|-------|
| Phase 1: Foundation + SmeltGraph | 6 weeks | 4 weeks | **4 weeks** | SmeltGraph wrapper for snapshots/diffs |
| Phase 2: SmeltValidator | 6 weeks | 1.5 weeks | **3 weeks** | Crucible + custom semantic delta validator |
| Phase 3: SmeltMemory | 6 weeks | 1.5 weeks | **4-5 weeks** | Build Smelt-native (MemRL too coupled) |
| Phase 4: Robustness | 4 weeks | 4 weeks | **4 weeks** | Unchanged |
| Phase 5: Polish | 4 weeks | 3 weeks | **3 weeks** | Unchanged |
| **Total** | **26 weeks** | **~14 weeks** | **~18-19 weeks** | More realistic based on actual code review |

> **Critical Path (v1.2):** SmeltGraph snapshot/diff implementation → SmeltMemory Bellman propagation → End-to-end validation

### Phase 1: Foundation + SmeltGraph (Weeks 1-4)

**Goal:** Basic intent → semantic delta → git commit flow

| Task | Owner | Priority | Effort | Notes |
|------|-------|----------|--------|-------|
| Smelt Core scaffold | - | P0 | 1 week | Workspace setup, core types |
| **CodeGraph snapshot API** | - | P0 | 1.5 weeks | **New capability needed in CodeGraph** |
| **CodeGraph diff algorithm** | - | P0 | 1.5 weeks | **New capability needed in CodeGraph** |
| Intent record storage (SQLite) | - | P0 | 3 days | |
| Basic CLI (init, intent, status, commit) | - | P0 | 1 week | |
| Git integration layer (git2-rs) | - | P0 | 3 days | |

**Critical Path:** CodeGraph snapshot/diff is blocking. This functionality does not exist in CodeGraph and must be implemented.

**Deliverables:**
- `smelt init` with background indexing
- `smelt intent create` → `smelt commit` workflow
- `smelt status` showing semantic changes
- Semantic deltas stored alongside Git commits
- Git hooks for bypass warning

### Phase 2: SmeltValidator Development (Weeks 5-7)

**Goal:** Architectural validation via Crucible + custom semantic delta validation

> **Note (v1.2):** Crucible v0.1.11 provides architectural validation (layer boundaries, cycles) but **cannot validate semantic deltas** (it only validates final state). SmeltValidator combines Crucible + custom semantic delta validation.

| Task | Owner | Priority | Effort | Notes |
|------|-------|----------|--------|-------|
| YAML-to-JSON adapter | - | P0 | 3 days | Translate Smelt YAML to Crucible JSON |
| Crucible validator integration | - | P0 | 3 days | Integrate validator (skip Parser, build Project from SmeltGraph) |
| Semantic delta validator | - | P0 | 1 week | **Custom build**: validate delta against intent constraints |
| Validation pipeline wiring | - | P0 | 3 days | Connect SmeltValidator to commit flow |
| CLI integration | - | P0 | 2 days | `smelt validate` subcommands |

**Deliverables:**
- `crucible.yaml` definition format (with adapter)
- SmeltValidator: Crucible architectural checks + semantic delta validation
- `smelt validate` command
- Validation integrated into commit flow
- Violations block commits with clear messages

### Phase 3: SmeltMemory Development (Weeks 8-12)

**Goal:** Build Smelt-native contextual memory system (inspired by MemRL algorithms)

> **Note (v1.2):** MemRL is too tightly coupled to Claude Code (MCP-only API, hardcoded paths, Node.js runtime). Building Smelt-native allows pure Rust implementation with direct API integration. Valuable algorithms (Bellman propagation, Wilson score, exponential decay) are ported to Smelt.

| Task | Owner | Priority | Effort | Notes |
|------|-------|----------|--------|-------|
| Core storage layer | - | P0 | 1 week | LanceDB vectors + SQLite metadata |
| Episode capture | - | P0 | 3 days | Intent → Episode transformation |
| Semantic retrieval | - | P0 | 4 days | Vector similarity + utility ranking |
| Wilson score | - | P1 | 3 days | Port Bayesian confidence from MemRL |
| Bellman propagation | - | P1 | 4 days | Port utility spreading from MemRL |
| Temporal decay | - | P1 | 2 days | Port exponential decay |
| CLI integration | - | P0 | 3 days | `smelt memory` subcommands |

**Ported Algorithms (from MemRL):**
- **Wilson score:** Handles sparse feedback with Bayesian confidence intervals
- **Bellman propagation:** Spreads utility values through episode similarity graph
- **Exponential decay:** Time-based utility degradation (configurable half-life)

**Deliverables:**
- `smelt memory record` command (Smelt-native)
- `smelt memory search` command (Smelt-native)
- Memories surfaced during intent creation
- Utility-weighted relevance ranking (Bellman + Wilson)

### Phase 4: Robustness & Recovery (Weeks 13-16)

**Goal:** Production-ready reliability

| Task | Owner | Priority | Effort |
|------|-------|----------|--------|
| `smelt sync` (git bypass recovery) | - | P0 | 1 week |
| `smelt doctor` (diagnostics & repair) | - | P0 | 1 week |
| Backup/restore mechanisms | - | P1 | 1 week |
| Error handling & edge cases | - | P0 | 1 week |

**Deliverables:**
- Recovery from direct git commits
- Diagnostic and repair tooling
- Automatic backups
- Graceful degradation modes

### Phase 5: Integration & Polish (Weeks 17-19)

**Goal:** End-to-end workflow with external integrations

| Task | Owner | Priority | Effort |
|------|-------|----------|--------|
| Agent REST API | - | P1 | 1.5 weeks |
| Documentation site | - | P0 | 1 week |
| Performance optimization | - | P1 | 3 days |

**Deliverables:**
- Agent API for programmatic access
- Comprehensive documentation
- Performance benchmarks meeting targets

### Phase 6 (Future): VSCode Extension & Cupola

**Scope:** Out of initial release

> **Foundation Discovered (v1.2):** [codegraph-vscode](https://github.com/anvanster/codegraph-vscode) provides proven architecture:
> - TypeScript VSCode extension + Rust LSP server
> - Already uses CodeGraph as library (not process spawning)
> - 26 Language Model Tools for AI agents
> - Memory layer with RocksDB + model2vec embeddings
> - v0.4.1, actively maintained

**Planned Integration:**
- Extend codegraph-vscode with Smelt-specific features
- Add intent creation/validation UI
- Integrate SmeltMemory retrieval into editor suggestions
- Real-time semantic delta visualization
- Cupola integration for test execution (Cupola is ~24.5K LOC, actively developed)

---

## 8. Technical Requirements

### 8.1 Language & Frameworks

| Component | Language | Key Dependencies |
|-----------|----------|------------------|
| Smelt Core | Rust | tokio, serde, clap |
| SmeltGraph | Rust | (wraps CodeGraph) tree-sitter, rocksdb |
| SmeltValidator | Rust | crucible (validator), serde_yaml |
| SmeltMemory | Rust | lancedb, sqlx, model2vec (Smelt-native, inspired by MemRL) |
| Agent API | Rust | axum, tower |
| VSCode Extension | TypeScript | vscode-api (extends codegraph-vscode) |

### 8.2 Storage Requirements

| Store | Technology | Contents | Sizing |
|-------|------------|----------|--------|
| Code Graph | RocksDB | AST nodes, edges, relationships | ~1GB per 100K nodes |
| Metadata | SQLite | Intents, deltas, memory metadata | ~100MB per 10K intents |
| Embeddings | LanceDB/Qdrant | Memory vectors | ~500MB per 10K memories |
| Arch Definitions | YAML files | crucible.yaml (git-versioned) | Negligible |
| Snapshots | RocksDB checkpoints | Baseline states for delta computation | ~500MB per active intent |

### 8.3 Performance Targets

| Operation | Target | Current Benchmark | Feasibility |
|-----------|--------|-------------------|-------------|
| Intent creation | < 500ms | ~200ms (retrieval) | ✅ Achievable |
| Semantic delta computation | < 2s | TBD (SmeltGraph snapshot/diff) | ⚠️ Critical path |
| Crucible validation | < 1s | ~15ms (current Crucible) | ✅ Exceeds target |
| SmeltMemory retrieval | < 200ms | ~150ms (LanceDB baseline) | ✅ Achievable |
| Commit with validation | < 5s | TBD (integration dependent) | ✅ Achievable |

> **Performance Notes (v1.2):**
> - CodeGraph node lookup: ~7ns (1000x better than target)
> - CodeGraph BFS traversal (depth=5): ~5ms
> - Crucible self-validation: ~15ms
> - SmeltMemory uses LanceDB with local model2vec embeddings (no network latency)
> - Wilson score + Bellman propagation: O(k) for k similar episodes

### 8.4 Compatibility

| Requirement | Specification |
|-------------|---------------|
| Git versions | 2.20+ |
| Languages (CodeGraph) | **14 languages**: Python, Rust, TypeScript/JavaScript, Go, C, C++, PHP, Java, Kotlin, Ruby, C#, Swift (all with complexity metrics) |
| Platforms | Linux, macOS (Windows later) |
| Git hosts | GitHub, GitLab, Bitbucket |

---

## 9. Success Metrics

### 9.1 Adoption Metrics

| Metric | Target (6 months) | Measurement |
|--------|-------------------|-------------|
| Active repositories | 100 | Repos with smelt.yaml |
| Daily commits via Smelt | 500 | Commits with semantic deltas |
| Architectural rules defined | 1000 | Rules across all repos |
| Memories created | 5000 | Memory records |

### 9.2 Quality Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Architectural violations caught | 80% pre-commit | Violations blocked vs. escaped |
| PR review time reduction | 60% | Time comparison study |
| Memory retrieval relevance | 70% helpful | User feedback |
| False positive rate (Crucible) | < 5% | User overrides |

### 9.3 Performance Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| P95 commit time | < 10s | Instrumentation |
| Memory retrieval latency | < 500ms | Instrumentation |
| CodeGraph parse throughput | > 1000 files/min | Benchmark |

---

## 10. Future Scope: Cupola Integration

**Note:** Cupola integration is planned for Phase 5+ after core features stabilize.

### 10.1 Cupola Purpose

Cupola provides on-demand compute infrastructure for:
- Fast, targeted test execution
- AI review agents with context
- Build validation
- Performance benchmarking

### 10.2 Integration Points

```
┌─────────────────────────────────────────────────────────────┐
│                    VALIDATION PHASE                         │
│                                                             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐         │
│  │  Crucible   │  │ SmeltMemory │  │   Cupola    │  ← New  │
│  │ (Arch)      │  │ (Memory)    │  │ (Compute)   │         │
│  └─────────────┘  └─────────────┘  └─────────────┘         │
│        │                │                │                  │
│        └────────────────┼────────────────┘                  │
│                         ▼                                   │
│              ┌─────────────────────┐                        │
│              │  Unified Validation │                        │
│              │       Result        │                        │
│              └─────────────────────┘                        │
└─────────────────────────────────────────────────────────────┘
```

### 10.3 Cupola Capabilities (Planned)

```bash
# Targeted test execution
$ smelt commit
⏳ Analyzing semantic delta...
✅ Architectural validation: passed
⏳ Cupola: Identifying affected tests...
⏳ Cupola: Running 47 tests on Linux runner...
✅ Tests: 47 passed (8.3s)
⏳ AI Review: Analyzing changes...
⚠️  Suggestion: Add rate limit headers to OpenAPI spec

# Performance validation
$ smelt commit --perf-check
⏳ Cupola: Running performance benchmark...
✅ Latency: 42ms p99 (budget: 50ms)
✅ Memory: 128MB peak (budget: 256MB)
```

---

## 11. Risks & Mitigations

### 11.1 Technical Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| CodeGraph parser gaps | Medium | High | Prioritize language support by user demand |
| Embedding costs (SmeltMemory) | Medium | Medium | Support local models, cache aggressively. SmeltMemory uses local model2vec embeddings. |
| False positive fatigue (Crucible) | High | High | Conservative defaults, easy override |
| Git edge cases | Medium | Medium | Extensive testing, graceful degradation |
| Adoption friction | High | High | Zero-config defaults, incremental value |
| Graph/Git sync loss | Medium | High | Recovery commands, consistency checks |

### 11.2 Integration Risks (Added v1.1)

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Schema format divergence (YAML vs JSON) | High | Medium | Decide format early; create bidirectional adapter layer |
| Version compatibility drift | Medium | High | Pin component versions; comprehensive integration tests |
| API contract changes | Medium | High | Define clear API boundaries; semantic versioning |
| Learning curve for contributors | Medium | Medium | Comprehensive documentation; onboarding guides |
| CodeGraph snapshot/diff implementation | High | High | This is the critical path; allocate senior engineers |
| SmeltMemory algorithm porting complexity | Medium | Medium | Focus on core algorithms (Bellman, Wilson); validate against MemRL test cases |

---

## 12. Operational Considerations

### 12.1 Git Bypass Handling

**Problem:** Users may run `git commit` directly, bypassing Smelt and causing the semantic layer to lose sync.

**Solutions:**

1. **Git Hooks (Recommended)**
   ```bash
   # .git/hooks/pre-commit (installed by `smelt init`)
   #!/bin/sh
   if [ -f .smelt/enabled ]; then
     echo "⚠️  Use 'smelt commit' for semantic tracking."
     echo "   Run 'git commit --no-verify' to bypass (not recommended)."
     exit 1
   fi
   ```

2. **Post-hoc Sync Recovery**
   ```bash
   # Detect and recover from direct git commits
   $ smelt sync
   ⚠️  Found 3 commits without semantic deltas:
     abc1234 - "fix typo" (2 files)
     def5678 - "add feature" (5 files)  
     ghi9012 - "refactor" (12 files)
   
   Options:
     [a] Analyze and create deltas retroactively
     [s] Skip and mark as untracked
     [i] Interactive (choose per commit)
   ```

3. **Graceful Degradation**
   - Smelt continues working even with gaps in semantic history
   - Untracked commits shown in `smelt log` with ⚠️ indicator
   - Architectural validation works on current state, not history

### 12.2 Repository Bootstrap

**Problem:** Existing repositories need initial CodeGraph indexing, which can be slow for large codebases.

**Solution: Progressive Initialization**

```bash
$ smelt init
⏳ Initializing Smelt in /path/to/repo...
✅ Configuration created: smelt.yaml
✅ Git hooks installed
⏳ Building initial code graph...
   Scanning files: 1,247 found
   Parsing: [████████░░░░░░░░░░░░] 40% (498/1247)
   
   💡 Smelt is usable now in degraded mode.
      Run 'smelt status' to check progress.
      Full validation available when indexing completes.

# Check indexing progress
$ smelt status
Code Graph: 73% indexed (912/1247 files)
Estimated completion: 2 minutes

# Degraded mode capabilities
- ✅ Intent creation
- ✅ Basic commit workflow  
- ⚠️  Architectural validation (partial - indexed files only)
- ⚠️  Dependency tracking (partial)
```

**Background Indexing:**
- Indexing runs in background process
- Does not block normal git operations
- Prioritizes recently changed files
- Resumes automatically after interruption

### 12.3 Error Recovery

**Scenario 1: Validation crash mid-commit**
```bash
$ smelt recover
Checking Smelt state...
⚠️  Found incomplete commit operation (intent: int_abc123)
   Files staged: 5
   Delta computed: yes
   Git commit: not created

Options:
  [r] Retry commit
  [a] Abort and unstage
  [v] Re-run validation only
```

**Scenario 2: CodeGraph corruption/mismatch**
```bash
$ smelt doctor
Running diagnostics...

❌ CodeGraph inconsistency detected
   Graph has 1,234 files
   Repository has 1,247 files
   Missing: 13 files (likely added since last parse)

Repair options:
  [f] Fast repair (parse missing files only)
  [r] Full rebuild (slower, guaranteed consistent)
  
$ smelt doctor --repair
⏳ Repairing code graph...
✅ Parsed 13 missing files
✅ Code graph consistent with repository
```

**Scenario 3: Smelt metadata corruption**
```bash
$ smelt doctor
❌ SQLite database corrupted

Repair options:
  [b] Restore from backup (.smelt/backup/)
  [r] Rebuild from git history (loses memory links)
  
$ smelt doctor --restore-backup
✅ Restored from backup (2 hours old)
⚠️  3 recent intents may need re-creation
```

---

## 13. Appendices

### Appendix A: Glossary

| Term | Definition |
|------|------------|
| **Intent Record** | Structured declaration of what change is desired and why |
| **Semantic Delta** | Machine-readable representation of meaning changes |
| **Architectural Gravity** | System-enforced architectural constraints |
| **Memory Record** | Captured decision context for future retrieval |
| **Violation** | Detected conflict with architectural rules |

### Appendix B: File Structure

```
smelt/
├── Cargo.toml              # Workspace manifest
├── crates/
│   ├── smelt-core/       # Orchestration engine
│   ├── smelt-cli/        # Command-line interface
│   ├── smelt-api/        # REST API server
│   ├── crucible/           # Architectural validation
│   └── memrl/              # Contextual memory
├── docs/
│   ├── architecture.md
│   └── user-guide/
└── tests/
    └── integration/
```

### Appendix C: Configuration File

```yaml
# smelt.yaml - Repository configuration
version: "1.0"

# CodeGraph settings
codegraph:
  storage: ".smelt/graph"
  languages:
    - python
    - rust
    - typescript

# Crucible architecture (inline or reference)
crucible:
  definition: "crucible.yaml"
  
# SmeltMemory settings
memory:
  storage: ".smelt/memory"
  embeddings:
    provider: "local"  # Uses model2vec (no external API needed)
    model: "model2vec-base"
  retrieval:
    top_k: 5
    min_relevance: 0.5
  utility:
    decay_half_life_days: 30  # Exponential decay for episode utility

# Git integration
git:
  auto_stage: true
  commit_template: |
    {intent.goal}
    
    Intent: {intent.id}
    Delta: {delta.id}
    
    {delta.summary}
```

### Appendix D: References

- [CodeGraph Repository](https://github.com/anvanster/codegraph)
- [Article Pitch: Beyond Git](./article-pitch-semantic-vcs.md)
- [Mem0: AI Agent Memory](https://mem0.ai/blog/memory-in-agents-what-why-and-how)
- [MemRL Paper: Self-Evolving Agents](https://arxiv.org/abs/2601.03192)

---

**Document History:**

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0 | 2026-01-25 | - | Initial PRD |
| 1.1 | 2026-01-25 | - | Updated component status: Crucible (v0.1.11) and MemRL are production-ready, not new builds. Revised timeline from 26 weeks to ~14 weeks. Added integration risks. Updated storage strategy. Noted YAML/JSON schema decision needed. |
| 1.2 | 2026-01-25 | - | **Deep architectural review of all components.** CodeGraph: integrate + SmeltGraph wrapper. Crucible: validator only + build semantic delta validator. MemRL: **build Smelt-native** (too coupled to Claude Code). Discovered codegraph-vscode for Phase 6. Revised timeline to ~18-19 weeks. |

---

*End of Document*
