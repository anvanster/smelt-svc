// Copyright 2024-2026 Andrey Vasilevsky <anvanster@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Backup and restore commands for Smelt data

use anyhow::{Context, Result};
use chrono::Utc;
use smelt_core::SmeltError;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Create a backup of Smelt data
pub async fn create(output: Option<PathBuf>, include_graph: bool) -> Result<()> {
    let cwd = std::env::current_dir().context("Failed to get current directory")?;
    let smelt_dir = cwd.join(".smelt");

    if !smelt_dir.exists() {
        return Err(SmeltError::NotInitialized.into());
    }

    // Determine output path
    let backup_name = format!("smelt-backup-{}.tar", Utc::now().format("%Y%m%d-%H%M%S"));
    let backup_path = output.unwrap_or_else(|| cwd.join(&backup_name));

    println!("Creating backup...");
    println!();

    // Create tar archive
    let file = File::create(&backup_path)?;
    let mut builder = tar::Builder::new(file);

    // Add database
    let db_path = smelt_dir.join("smelt.db");
    if db_path.exists() {
        println!("  Adding: smelt.db");
        builder.append_path_with_name(&db_path, "smelt.db")?;
    }

    // Add memory directory
    let memory_dir = smelt_dir.join("memory");
    if memory_dir.exists() {
        println!("  Adding: memory/");
        add_directory_to_tar(&mut builder, &memory_dir, "memory")?;
    }

    // Optionally add graph directory (can be large)
    if include_graph {
        let graph_dir = smelt_dir.join("graph");
        if graph_dir.exists() {
            println!("  Adding: graph/");
            add_directory_to_tar(&mut builder, &graph_dir, "graph")?;
        }
    }

    // Add config files
    let config_path = smelt_dir.join("config.toml");
    if config_path.exists() {
        println!("  Adding: config.toml");
        builder.append_path_with_name(&config_path, "config.toml")?;
    }

    // Add validation config
    let validation_config = smelt_dir.join("validation.toml");
    if validation_config.exists() {
        println!("  Adding: validation.toml");
        builder.append_path_with_name(&validation_config, "validation.toml")?;
    }

    // Finalize archive
    builder.finish()?;

    // Get file size
    let metadata = fs::metadata(&backup_path)?;
    let size_kb = metadata.len() / 1024;

    println!();
    println!("Backup created: {}", backup_path.display());
    println!("Size: {} KB", size_kb);

    if !include_graph {
        println!();
        println!("Note: Graph data not included. Use --include-graph to include it.");
    }

    Ok(())
}

/// Restore from a backup
pub async fn restore(backup_path: PathBuf, force: bool) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let smelt_dir = cwd.join(".smelt");

    if !backup_path.exists() {
        anyhow::bail!("Backup file not found: {}", backup_path.display());
    }

    if smelt_dir.exists() && !force {
        anyhow::bail!(
            "Smelt directory already exists. Use --force to overwrite, or back up first."
        );
    }

    println!("Restoring from backup...");
    println!();

    // Create smelt directory if needed
    if !smelt_dir.exists() {
        fs::create_dir_all(&smelt_dir)?;
    }

    // Open tar archive
    let file = File::open(&backup_path)?;
    let mut archive = tar::Archive::new(file);

    // Extract files
    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?;
        let dest = smelt_dir.join(&path);

        println!("  Restoring: {}", path.display());

        // Create parent directories
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }

        entry.unpack(&dest)?;
    }

    println!();
    println!("Restore complete.");
    println!();
    println!("Run 'smelt doctor' to verify the installation.");

    Ok(())
}

/// List contents of a backup
pub async fn list(backup_path: PathBuf) -> Result<()> {
    if !backup_path.exists() {
        anyhow::bail!("Backup file not found: {}", backup_path.display());
    }

    println!("Backup contents: {}", backup_path.display());
    println!();

    let file = File::open(&backup_path)?;
    let mut archive = tar::Archive::new(file);

    let mut total_size = 0u64;
    let mut file_count = 0;

    for entry in archive.entries()? {
        let entry = entry?;
        let path = entry.path()?;
        let size = entry.size();

        total_size += size;
        file_count += 1;

        println!("  {} ({} bytes)", path.display(), size);
    }

    println!();
    println!("Total: {} files, {} KB", file_count, total_size / 1024);

    Ok(())
}

/// Verify a backup file
pub async fn verify(backup_path: PathBuf) -> Result<()> {
    if !backup_path.exists() {
        anyhow::bail!("Backup file not found: {}", backup_path.display());
    }

    println!("Verifying backup: {}", backup_path.display());
    println!();

    let file = File::open(&backup_path)?;
    let mut archive = tar::Archive::new(file);

    let mut errors = Vec::new();
    let mut has_database = false;
    let mut file_count = 0;

    for entry in archive.entries()? {
        match entry {
            Ok(entry) => {
                let path = entry.path()?;
                let path_str = path.to_string_lossy();

                if path_str == "smelt.db" {
                    has_database = true;
                }

                file_count += 1;
            }
            Err(e) => {
                errors.push(format!("Corrupt entry: {}", e));
            }
        }
    }

    if !has_database {
        errors.push("Missing smelt.db database file".to_string());
    }

    if errors.is_empty() {
        println!("✓ Backup is valid ({} files)", file_count);
    } else {
        println!("✗ Backup has issues:");
        for error in errors {
            println!("  - {}", error);
        }
        anyhow::bail!("Backup verification failed");
    }

    Ok(())
}

fn add_directory_to_tar<W: Write>(
    builder: &mut tar::Builder<W>,
    dir: &Path,
    prefix: &str,
) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = format!("{}/{}", prefix, entry.file_name().to_string_lossy());

        if path.is_dir() {
            add_directory_to_tar(builder, &path, &name)?;
        } else {
            builder.append_path_with_name(&path, &name)?;
        }
    }
    Ok(())
}
