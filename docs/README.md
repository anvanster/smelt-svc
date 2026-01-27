# Smelt Documentation

Smelt is a semantic version control system that layers over Git, providing intent-driven development with AI-native workflows.

## Quick Start

```bash
# Initialize Smelt in your repository
smelt init

# Create an intent for your work
smelt intent create --goal "Add user authentication"

# Make your code changes...

# Check semantic status
smelt status

# Validate changes
smelt validate

# Commit with semantic delta
smelt commit --intent <intent-id>
```

## Documentation

- [Architecture Overview](./architecture.md) - System design and component structure
- [CLI Usage Guide](./cli-usage.md) - Command-line interface reference
- [REST API Reference](./api.md) - HTTP API documentation

## Core Concepts

### Intents

An **Intent** is a structured declaration of what you want to accomplish. It includes:
- **Goal**: Natural language description of the desired outcome
- **Rationale**: Why this change is being made (optional)
- **Constraints**: Limits on scope, complexity, or behavior
- **Context Links**: References to issues, PRs, or documentation

### Semantic Deltas

A **Semantic Delta** captures the meaning of code changes, not just textual diffs:
- Functions added, removed, or modified
- Types and their structural changes
- Dependency relationships
- Breaking change detection
- Complexity impact analysis

### Episodic Memory

Smelt includes a contextual memory system that learns from your development history:
- Captures successful coding episodes
- Retrieves relevant past experiences via semantic search
- Improves over time through feedback and utility propagation

## Architecture

```
smelt-svc/
├── crates/
│   ├── smelt-core/       # Core types, graph, storage, git
│   ├── smelt-validator/  # Semantic and architectural validation
│   ├── smelt-memory/     # Episodic memory system
│   ├── smelt-cli/        # Command-line interface
│   └── smelt-api/        # REST API server
└── tests/
    └── integration/      # End-to-end tests
```

## License

Apache-2.0
