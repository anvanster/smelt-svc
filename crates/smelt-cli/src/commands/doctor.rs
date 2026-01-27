//! Doctor command - diagnose and repair Smelt installation

use anyhow::Result;
use smelt_core::{Git2Interface, GitInterface, SmeltGraph, SqliteStorage};
use std::fs;
use std::path::Path;

/// Diagnostic check result
#[derive(Debug)]
struct CheckResult {
    name: &'static str,
    passed: bool,
    message: String,
    fixable: bool,
}

impl CheckResult {
    fn pass(name: &'static str, message: impl Into<String>) -> Self {
        Self {
            name,
            passed: true,
            message: message.into(),
            fixable: false,
        }
    }

    fn fail(name: &'static str, message: impl Into<String>, fixable: bool) -> Self {
        Self {
            name,
            passed: false,
            message: message.into(),
            fixable,
        }
    }
}

/// Run the doctor command
pub async fn run(fix: bool) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let smelt_dir = cwd.join(".smelt");

    println!("Smelt Doctor");
    println!("============");
    println!();

    let mut checks: Vec<CheckResult> = Vec::new();

    // Check 1: Smelt initialization
    checks.push(check_smelt_initialized(&smelt_dir));

    // Check 2: Git repository
    checks.push(check_git_repository(&cwd));

    // If smelt not initialized, skip remaining checks
    if !smelt_dir.exists() {
        print_results(&checks);
        if !checks[0].passed {
            println!();
            println!("Run 'smelt init' to initialize Smelt in this repository.");
        }
        return Ok(());
    }

    // Check 3: Database integrity
    checks.push(check_database(&smelt_dir));

    // Check 4: Graph directory
    checks.push(check_graph(&smelt_dir));

    // Check 5: Memory directory
    checks.push(check_memory(&smelt_dir));

    // Check 6: Directory permissions
    checks.push(check_permissions(&smelt_dir));

    // Check 7: Orphaned files
    checks.push(check_orphaned_files(&smelt_dir));

    // Print results
    print_results(&checks);

    // Count issues
    let failed: Vec<_> = checks.iter().filter(|c| !c.passed).collect();
    let fixable: Vec<_> = failed.iter().filter(|c| c.fixable).collect();

    if failed.is_empty() {
        println!();
        println!("All checks passed! Smelt installation is healthy.");
    } else {
        println!();
        println!(
            "Found {} issue(s), {} fixable.",
            failed.len(),
            fixable.len()
        );

        if fix && !fixable.is_empty() {
            println!();
            println!("Attempting repairs...");
            println!();

            for check in fixable {
                if let Some(repair_result) = attempt_repair(check.name, &smelt_dir).await {
                    if repair_result {
                        println!("  ✓ Fixed: {}", check.name);
                    } else {
                        println!("  ✗ Could not fix: {}", check.name);
                    }
                }
            }
        } else if !fixable.is_empty() {
            println!();
            println!("Run 'smelt doctor --fix' to attempt automatic repairs.");
        }
    }

    Ok(())
}

fn check_smelt_initialized(smelt_dir: &Path) -> CheckResult {
    if smelt_dir.exists() && smelt_dir.is_dir() {
        CheckResult::pass("smelt-initialized", "Smelt directory found")
    } else {
        CheckResult::fail(
            "smelt-initialized",
            "Smelt not initialized in this directory",
            false,
        )
    }
}

fn check_git_repository(cwd: &Path) -> CheckResult {
    match Git2Interface::open(&cwd.to_path_buf()) {
        Ok(git) => {
            if git.is_initialized() {
                match git.current_branch() {
                    Ok(branch) => CheckResult::pass(
                        "git-repository",
                        format!("Git repository found (branch: {})", branch),
                    ),
                    Err(_) => CheckResult::pass("git-repository", "Git repository found"),
                }
            } else {
                CheckResult::fail(
                    "git-repository",
                    "Git repository exists but has no commits",
                    false,
                )
            }
        }
        Err(_) => CheckResult::fail("git-repository", "Not a git repository", false),
    }
}

fn check_database(smelt_dir: &Path) -> CheckResult {
    let db_path = smelt_dir.join("smelt.db");

    if !db_path.exists() {
        return CheckResult::fail("database", "Database file not found", true);
    }

    match SqliteStorage::open(&db_path) {
        Ok(storage) => {
            // Try a simple query to verify integrity
            match storage.list_intents(None) {
                Ok(intents) => CheckResult::pass(
                    "database",
                    format!("Database OK ({} intents)", intents.len()),
                ),
                Err(e) => CheckResult::fail(
                    "database",
                    format!("Database query failed: {}", e),
                    false,
                ),
            }
        }
        Err(e) => CheckResult::fail("database", format!("Cannot open database: {}", e), false),
    }
}

fn check_graph(smelt_dir: &Path) -> CheckResult {
    let graph_path = smelt_dir.join("graph");

    if !graph_path.exists() {
        return CheckResult::fail("graph", "Graph directory not found", true);
    }

    match SmeltGraph::open(&graph_path) {
        Ok(graph) => {
            let node_count = graph.node_count();
            let edge_count = graph.edge_count();
            CheckResult::pass(
                "graph",
                format!("Graph OK ({} nodes, {} edges)", node_count, edge_count),
            )
        }
        Err(e) => CheckResult::fail("graph", format!("Cannot open graph: {}", e), false),
    }
}

fn check_memory(smelt_dir: &Path) -> CheckResult {
    let memory_dir = smelt_dir.join("memory");

    if !memory_dir.exists() {
        // Memory directory is optional - create if missing
        return CheckResult::fail("memory", "Memory directory not found", true);
    }

    let db_path = memory_dir.join("memory.db");
    if !db_path.exists() {
        return CheckResult::fail("memory", "Memory database not found", true);
    }

    CheckResult::pass("memory", "Memory directory OK")
}

fn check_permissions(smelt_dir: &Path) -> CheckResult {
    // Check if we can write to the smelt directory
    let test_file = smelt_dir.join(".doctor_test");

    match fs::write(&test_file, "test") {
        Ok(_) => {
            let _ = fs::remove_file(&test_file);
            CheckResult::pass("permissions", "Directory is writable")
        }
        Err(e) => CheckResult::fail(
            "permissions",
            format!("Cannot write to .smelt directory: {}", e),
            false,
        ),
    }
}

fn check_orphaned_files(smelt_dir: &Path) -> CheckResult {
    // Check for common orphaned/temporary files
    let orphaned_patterns = [".lock", ".tmp", ".bak"];
    let mut orphaned_count = 0;

    if let Ok(entries) = fs::read_dir(smelt_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            for pattern in &orphaned_patterns {
                if name_str.ends_with(pattern) {
                    orphaned_count += 1;
                }
            }
        }
    }

    if orphaned_count > 0 {
        CheckResult::fail(
            "orphaned-files",
            format!("{} orphaned/temporary files found", orphaned_count),
            true,
        )
    } else {
        CheckResult::pass("orphaned-files", "No orphaned files")
    }
}

fn print_results(checks: &[CheckResult]) {
    for check in checks {
        let status = if check.passed { "✓" } else { "✗" };
        let fixable = if !check.passed && check.fixable {
            " (fixable)"
        } else {
            ""
        };
        println!("  {} {}: {}{}", status, check.name, check.message, fixable);
    }
}

async fn attempt_repair(check_name: &str, smelt_dir: &Path) -> Option<bool> {
    match check_name {
        "database" => {
            // Create empty database if missing
            let db_path = smelt_dir.join("smelt.db");
            if !db_path.exists() {
                match SqliteStorage::open(&db_path) {
                    Ok(_) => Some(true),
                    Err(_) => Some(false),
                }
            } else {
                Some(false)
            }
        }
        "graph" => {
            // Create empty graph directory if missing
            let graph_path = smelt_dir.join("graph");
            if !graph_path.exists() {
                match fs::create_dir_all(&graph_path) {
                    Ok(_) => match SmeltGraph::open(&graph_path) {
                        Ok(_) => Some(true),
                        Err(_) => Some(false),
                    },
                    Err(_) => Some(false),
                }
            } else {
                Some(false)
            }
        }
        "memory" => {
            // Create memory directory if missing
            let memory_dir = smelt_dir.join("memory");
            match fs::create_dir_all(&memory_dir) {
                Ok(_) => Some(true),
                Err(_) => Some(false),
            }
        }
        "orphaned-files" => {
            // Remove orphaned files
            let orphaned_patterns = [".lock", ".tmp", ".bak"];
            let mut removed = 0;

            if let Ok(entries) = fs::read_dir(smelt_dir) {
                for entry in entries.flatten() {
                    let name = entry.file_name();
                    let name_str = name.to_string_lossy();
                    for pattern in &orphaned_patterns {
                        if name_str.ends_with(pattern) {
                            if fs::remove_file(entry.path()).is_ok() {
                                removed += 1;
                            }
                        }
                    }
                }
            }

            Some(removed > 0)
        }
        _ => None,
    }
}

/// Show detailed diagnostics
pub async fn verbose() -> Result<()> {
    let cwd = std::env::current_dir()?;
    let smelt_dir = cwd.join(".smelt");

    println!("Smelt Diagnostics (Verbose)");
    println!("===========================");
    println!();

    // System info
    println!("System Information:");
    println!("  Working directory: {}", cwd.display());
    println!("  Smelt directory: {}", smelt_dir.display());

    if smelt_dir.exists() {
        // Directory contents
        println!();
        println!("Smelt Directory Contents:");
        if let Ok(entries) = fs::read_dir(&smelt_dir) {
            for entry in entries.flatten() {
                let metadata = entry.metadata();
                let size = metadata.map(|m| m.len()).unwrap_or(0);
                let file_type = if entry.path().is_dir() {
                    "dir"
                } else {
                    "file"
                };
                println!(
                    "  {} ({}, {} bytes)",
                    entry.file_name().to_string_lossy(),
                    file_type,
                    size
                );
            }
        }

        // Database stats
        let db_path = smelt_dir.join("smelt.db");
        if db_path.exists() {
            if let Ok(storage) = SqliteStorage::open(&db_path) {
                println!();
                println!("Database Statistics:");
                if let Ok(intents) = storage.list_intents(None) {
                    println!("  Total intents: {}", intents.len());
                    let committed = intents
                        .iter()
                        .filter(|i| matches!(i.status, smelt_core::IntentStatus::Committed { .. }))
                        .count();
                    let in_progress = intents
                        .iter()
                        .filter(|i| matches!(i.status, smelt_core::IntentStatus::InProgress))
                        .count();
                    println!("  Committed: {}", committed);
                    println!("  In progress: {}", in_progress);
                }
            }
        }

        // Graph stats
        let graph_path = smelt_dir.join("graph");
        if graph_path.exists() {
            if let Ok(graph) = SmeltGraph::open(&graph_path) {
                println!();
                println!("Graph Statistics:");
                println!("  Nodes: {}", graph.node_count());
                println!("  Edges: {}", graph.edge_count());
            }
        }
    }

    // Git info
    if let Ok(git) = Git2Interface::open(&cwd) {
        println!();
        println!("Git Information:");
        if let Ok(branch) = git.current_branch() {
            println!("  Current branch: {}", branch);
        }
        if let Ok(sha) = git.head_sha() {
            println!("  HEAD: {}", &sha[..8]);
        }
        if let Ok(commits) = git.list_commits(5) {
            println!("  Recent commits: {}", commits.len());
        }
    }

    Ok(())
}
