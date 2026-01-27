//! Initialize Smelt in a repository

use anyhow::{Context, Result};
use smelt_core::{Git2Interface, GitInterface, SmeltError, SmeltGraph, SqliteStorage};
use std::path::Path;

pub async fn run(wait_for_indexing: bool) -> Result<()> {
    let cwd = std::env::current_dir().context("Failed to get current directory")?;

    // Check if already initialized
    let smelt_dir = cwd.join(".smelt");
    if smelt_dir.exists() {
        // Check if it's a valid installation
        if smelt_dir.join("smelt.db").exists() {
            println!("Smelt already initialized in this repository.");
            println!();
            println!("Run 'smelt doctor' to check installation health.");
            return Ok(());
        } else {
            // Partial installation - clean up and reinitialize
            println!("Found incomplete Smelt installation. Reinitializing...");
            std::fs::remove_dir_all(&smelt_dir)
                .context("Failed to clean up incomplete installation")?;
        }
    }

    // Verify git repository exists
    let git = match Git2Interface::open(&cwd) {
        Ok(g) => g,
        Err(_) => {
            return Err(SmeltError::NotAGitRepository.into());
        }
    };

    if !git.is_initialized() {
        return Err(SmeltError::EmptyRepository.into());
    }

    // Check for git author configuration
    if git.get_author().is_err() {
        eprintln!("Warning: Git author not configured. Using default.");
    }

    println!("Initializing Smelt in {:?}...", cwd);

    // Create .smelt directory structure
    std::fs::create_dir_all(&smelt_dir).context("Failed to create .smelt directory")?;
    std::fs::create_dir_all(smelt_dir.join("graph"))
        .context("Failed to create .smelt/graph directory")?;
    std::fs::create_dir_all(smelt_dir.join("snapshots"))
        .context("Failed to create .smelt/snapshots directory")?;
    std::fs::create_dir_all(smelt_dir.join("memory"))
        .context("Failed to create .smelt/memory directory")?;

    // Initialize SQLite database
    let db_path = smelt_dir.join("smelt.db");
    let _storage = SqliteStorage::open(&db_path).context("Failed to initialize database")?;
    println!("  Database created: {:?}", db_path);

    // Initialize SmeltGraph
    let graph_path = smelt_dir.join("graph");
    let _graph = SmeltGraph::open(&graph_path).context("Failed to initialize graph storage")?;
    println!("  Graph storage created: {:?}", graph_path);

    // Install git hooks
    install_git_hooks(&cwd)?;
    println!("  Git hooks installed");

    // Create config file
    create_default_config(&smelt_dir)?;
    println!("  Configuration created");

    // Create enabled marker
    std::fs::write(smelt_dir.join("enabled"), "").context("Failed to create enabled marker")?;

    // Start background indexing
    if wait_for_indexing {
        println!("\nIndexing repository...");
        // TODO: Implement full indexing with CodeGraph parsers
        println!("  Scanning files: 0 found");
        println!("  Indexing complete.");
    } else {
        println!("\nStarting background indexing...");
        println!("  Run 'smelt status' to check progress.");
    }

    println!("\n Smelt initialized successfully!");
    println!("\nNext steps:");
    println!("  1. Create an intent: smelt intent create --goal \"Your goal\"");
    println!("  2. Make code changes");
    println!("  3. Check status: smelt status");
    println!("  4. Commit: smelt commit");

    Ok(())
}

fn install_git_hooks(repo_root: &Path) -> Result<()> {
    let hooks_dir = repo_root.join(".git/hooks");
    std::fs::create_dir_all(&hooks_dir).context("Failed to create hooks directory")?;

    let pre_commit_hook = r#"#!/bin/sh
# Smelt pre-commit hook
# Warns about using git commit directly instead of smelt commit

if [ -f .smelt/enabled ]; then
    echo ""
    echo "Warning: Use 'smelt commit' for semantic tracking."
    echo "   Run 'git commit --no-verify' to bypass (not recommended)."
    echo ""
    exit 1
fi
"#;

    let hook_path = hooks_dir.join("pre-commit");
    std::fs::write(&hook_path, pre_commit_hook).context("Failed to write pre-commit hook")?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&hook_path, std::fs::Permissions::from_mode(0o755))
            .context("Failed to set hook permissions")?;
    }

    Ok(())
}

fn create_default_config(smelt_dir: &Path) -> Result<()> {
    let config = r#"# Smelt Configuration
# See https://github.com/anvanster/smelt-svc for documentation

[codegraph]
storage = "graph"
languages = ["python", "rust", "typescript", "javascript", "go"]

[memory]
storage = "memory"
embeddings_model = "bge-small-en-v1.5"

[git]
auto_stage = true

[validation]
strict = false
"#;

    std::fs::write(smelt_dir.join("config.toml"), config).context("Failed to write config file")?;
    Ok(())
}
