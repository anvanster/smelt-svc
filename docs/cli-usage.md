# Smelt CLI Usage Guide

## Installation

Build from source:

```bash
cargo install --path crates/smelt-cli
```

## Global Options

```bash
smelt [OPTIONS] <COMMAND>

Options:
  -v, --verbose  Enable verbose logging
  -h, --help     Print help
  -V, --version  Print version
```

## Commands

### smelt init

Initialize Smelt in the current repository.

```bash
smelt init [OPTIONS]

Options:
  --wait  Wait for indexing to complete (default: background)
```

Creates a `.smelt/` directory with:
- `smelt.db` - SQLite database for intents and deltas
- `graph/` - Code graph data (RocksDB)
- `memory/` - Episodic memory storage

Example:
```bash
cd my-project
smelt init
# Initializing Smelt...
# Background indexing started. Use 'smelt status --full' to check progress.
```

### smelt intent

Manage development intents.

#### intent create

Create a new intent for your work.

```bash
smelt intent create --goal <GOAL> [--rationale <RATIONALE>]

Options:
  --goal <GOAL>            Intent goal (required)
  --rationale <RATIONALE>  Optional rationale
```

Example:
```bash
smelt intent create --goal "Add user authentication" --rationale "Security requirement"
# Created intent: abc12345
# Goal: Add user authentication
```

#### intent list

List all intents.

```bash
smelt intent list [--status <STATUS>]

Options:
  --status <STATUS>  Filter by status (Draft, InProgress, Committed, etc.)
```

Example:
```bash
smelt intent list
# ID         STATUS      GOAL
# abc12345   InProgress  Add user authentication
# def67890   Committed   Fix database connection leak
```

#### intent show

Show intent details.

```bash
smelt intent show <ID>

Arguments:
  <ID>  Intent ID (can be partial)
```

Example:
```bash
smelt intent show abc1
# Intent: abc12345
# Goal: Add user authentication
# Status: InProgress
# Created: 2024-01-15 10:30:00
```

### smelt status

Show current semantic status.

```bash
smelt status [--full]

Options:
  --full  Show full details including indexing progress
```

Example:
```bash
smelt status
# Smelt Status
# Repository: my-project
# Active Intent: abc12345 (Add user authentication)
#
# Changes:
#   + src/auth.rs (new file)
#   ~ src/main.rs (modified)
```

### smelt validate

Validate changes without committing.

```bash
smelt validate [OPTIONS]

Options:
  --intent <INTENT>  Validate against specific intent
  --strict           Use strict validation mode
  --show-config      Show validation configuration
```

Example:
```bash
smelt validate
# Validation Results
# ✓ No breaking changes detected
# ✓ Layer boundaries respected
# ⚠ Complexity increased by 5 (threshold: 10)
#
# Passed: 2 checks
# Warnings: 1
```

### smelt commit

Commit with semantic delta capture.

```bash
smelt commit [OPTIONS]

Options:
  --intent <INTENT>      Use existing intent ID
  --goal <GOAL>          Create inline intent with goal
  --skip-validation      Skip validation (not recommended)
```

Example:
```bash
smelt commit --intent abc12345
# Validating changes...
# ✓ Validation passed
# Creating semantic delta...
# Creating git commit...
# Committed: 1a2b3c4d
# Intent abc12345 completed
```

### smelt memory

Manage episodic memory.

#### memory search

Search for relevant past experiences.

```bash
smelt memory search <QUERY> [--limit <LIMIT>]

Arguments:
  <QUERY>  Search query

Options:
  --limit <LIMIT>  Maximum results to return (default: 5)
```

Example:
```bash
smelt memory search "authentication JWT"
# Found 3 relevant episodes:
#
# 1. [0.85] Implemented JWT token validation
#    Files: src/auth.rs, src/jwt.rs
#    Tags: auth, jwt, security
#
# 2. [0.72] Fixed token expiration bug
#    Files: src/auth.rs
#    Tags: auth, bugfix
```

#### memory feedback

Record feedback for an episode.

```bash
smelt memory feedback <EPISODE_ID> [--helpful | --not-helpful]

Arguments:
  <EPISODE_ID>  Episode ID

Options:
  --helpful      Mark as helpful
  --not-helpful  Mark as not helpful
```

#### memory stats

Show memory statistics.

```bash
smelt memory stats
# Memory Statistics
# Total episodes: 47
# Helpful: 32 (68%)
# Average utility: 0.72
# Last propagation: 2024-01-14
```

#### memory propagate

Run utility propagation.

```bash
smelt memory propagate [--temporal]

Options:
  --temporal  Include temporal credit assignment
```

### smelt sync

Sync with git history (recover from direct git commits).

```bash
smelt sync [--dry-run] [--limit <LIMIT>]

Options:
  --dry-run         Show what would be done without making changes
  --limit <LIMIT>   Number of commits to scan (default: 50)
```

Example:
```bash
smelt sync --dry-run
# Found 3 untracked commits:
#   1a2b3c4d - Add feature X
#   5e6f7g8h - Fix bug Y
#   9i0j1k2l - Update docs
#
# Run without --dry-run to create synthetic intents.
```

### smelt doctor

Diagnose and repair Smelt installation.

```bash
smelt doctor [--fix] [--verbose]

Options:
  --fix      Attempt automatic repairs for fixable issues
  --verbose  Show detailed diagnostic information
```

Example:
```bash
smelt doctor
# Smelt Doctor
# ✓ .smelt directory exists
# ✓ Database accessible
# ✓ Graph storage valid
# ⚠ 2 orphaned deltas found
#
# Run 'smelt doctor --fix' to repair issues.
```

### smelt backup

Backup and restore Smelt data.

#### backup create

Create a backup.

```bash
smelt backup create [--output <PATH>] [--include-graph]

Options:
  --output <PATH>   Output file path (default: smelt-backup-{timestamp}.tar)
  --include-graph   Include graph data (can be large)
```

#### backup restore

Restore from a backup.

```bash
smelt backup restore <BACKUP_FILE> [--force]

Arguments:
  <BACKUP_FILE>  Backup file to restore from

Options:
  --force  Overwrite existing Smelt data
```

#### backup list

List contents of a backup.

```bash
smelt backup list <BACKUP_FILE>
```

#### backup verify

Verify a backup file.

```bash
smelt backup verify <BACKUP_FILE>
```

### smelt completions

Generate shell completions.

```bash
smelt completions <SHELL>

Arguments:
  <SHELL>  Shell to generate completions for (bash, zsh, fish, powershell)
```

Example:
```bash
# Bash
smelt completions bash > ~/.local/share/bash-completion/completions/smelt

# Zsh
smelt completions zsh > ~/.zfunc/_smelt

# Fish
smelt completions fish > ~/.config/fish/completions/smelt.fish
```

## Environment Variables

- `SMELT_LOG` - Set log level (trace, debug, info, warn, error)
- `SMELT_DIR` - Override .smelt directory location

## Exit Codes

- `0` - Success
- `1` - General error
- `2` - Validation failed
- `3` - Not initialized (run `smelt init`)
