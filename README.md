# Smelt

Smelt is a semantic version control system that layers over Git, providing intent-driven development with AI-native workflows.

## Installation

```bash
cargo install smelt-cli
```

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

## End-to-End Example

Here's a complete workflow demonstrating Smelt's semantic version control:

```bash
# 1. Initialize Smelt in your repository
$ smelt init --wait
Initializing Smelt in "/Users/dev/my-project"...
  Database created: "/Users/dev/my-project/.smelt/smelt.db"
  Graph storage created: "/Users/dev/my-project/.smelt/graph"
  Git hooks installed
  Configuration created

Indexing repository...
  Scanning files: 0 found
  Indexing complete.

✓ Smelt initialized successfully!

# 2. Create an intent describing your goal
$ smelt intent create --goal "Add greeting message to CLI startup"
Created intent: fce32c4c

  Goal: Add greeting message to CLI startup
  Status: In Progress
  Baseline snapshot: 8795a78d

Now make your code changes, then run 'smelt status' to see semantic changes.

# 3. Make your code changes (e.g., add a function to main.rs)
# ... edit files ...

# 4. Check semantic status
$ smelt status
Intent: fce32c4c (Add greeting message to CLI startup)
Status: In Progress

Changed files (1):
  M src/main.rs

Semantic changes:
  (Computing delta from 1 files...)

Impact Summary:
  Files affected: 1

# 5. Validate changes against architectural rules
$ smelt validate
Validating 1 changed files...

Validation Results:
==================

✅ Validation passed

# 6. Commit with semantic delta
$ smelt commit --intent fce32c4c
Committing intent: fce32c4c (Add greeting message to CLI startup)
  Staged 1 files
  Computing semantic delta...
  Running validation...
    Validation: passed
  Creating commit...

✓ Committed: 06d0a797
   Intent: fce32c4c
   Delta:  5eeac27c
   Files:  1 changed
```

The resulting git commit includes semantic metadata:

```
commit 06d0a797559d1791f6dc0b5dc3742897e1446e60
Author: Developer <dev@example.com>
Date:   Mon Jan 26 22:23:17 2026 -0800

    Add greeting message to CLI startup

    Intent: fce32c4c-434e-44bb-bcb8-b2c747756279
    Delta: 5eeac27c-ed52-45c1-ab07-3517a7044a85

 src/main.rs | 5 +++++
 1 file changed, 5 insertions(+)
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

## MCP Server (AI Assistant Integration)

Smelt includes an MCP (Model Context Protocol) server that allows AI assistants like Claude Code to interact with Smelt's semantic version control capabilities.

### Installation

```bash
cargo install smelt-mcp
```

### Configuration

Add to your Claude Code MCP configuration (`~/.claude.json`):

```json
{
  "mcpServers": {
    "smelt": {
      "type": "stdio",
      "command": "smelt-mcp",
      "args": [],
      "env": {}
    }
  }
}
```

### Available Tools

| Tool | Description |
|------|-------------|
| `smelt_init` | Initialize Smelt in a repository |
| `smelt_status` | Show semantic status of working directory |
| `smelt_validate` | Validate changes against constraints |
| `smelt_commit` | Commit with semantic delta attached |
| `smelt_intent_create` | Create a new intent |
| `smelt_intent_list` | List intents with optional filtering |
| `smelt_memory_search` | Search episodic memory |
| `smelt_memory_capture` | Capture a task as an episode |
| `smelt_memory_feedback` | Provide feedback on retrieved episodes |

## Architecture

```
smelt-svc/
├── crates/
│   ├── smelt-core/       # Core types, graph, storage, git
│   ├── smelt-validator/  # Semantic and architectural validation
│   ├── smelt-memory/     # Episodic memory system
│   ├── smelt-cli/        # Command-line interface
│   ├── smelt-api/        # REST API server
│   └── smelt-mcp/        # MCP server for AI assistants
└── tests/
    └── integration/      # End-to-end tests
```

## License

Apache-2.0
