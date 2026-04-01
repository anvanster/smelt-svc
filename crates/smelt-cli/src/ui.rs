// Copyright 2024-2026 Andrey Vasilevsky <anvanster@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! UI utilities for consistent CLI output and error handling

#![allow(dead_code)]

use smelt_core::{GitInterface, SmeltError};
use std::path::Path;

/// Display an error with optional suggestion
pub fn display_error(err: &anyhow::Error) {
    eprintln!("Error: {}", err);

    // Check for SmeltError to show suggestions
    if let Some(smelt_err) = err.downcast_ref::<SmeltError>() {
        if let Some(suggestion) = smelt_err.suggestion() {
            eprintln!();
            eprintln!("Suggestion: {}", suggestion);
        }
    }

    // Show cause chain for debugging
    let mut source = err.source();
    if source.is_some() {
        eprintln!();
        eprintln!("Caused by:");
        while let Some(cause) = source {
            eprintln!("  - {}", cause);
            source = cause.source();
        }
    }
}

/// Print a success message with checkmark
pub fn success(msg: &str) {
    println!("✓ {}", msg);
}

/// Print a warning message
pub fn warn(msg: &str) {
    println!("⚠ {}", msg);
}

/// Print an info message
pub fn info(msg: &str) {
    println!("ℹ {}", msg);
}

/// Print a step in a process
pub fn step(msg: &str) {
    println!("  {}", msg);
}

/// Check if Smelt is initialized in the given directory
pub fn check_smelt_initialized(cwd: &Path) -> anyhow::Result<()> {
    let smelt_dir = cwd.join(".smelt");
    if !smelt_dir.exists() {
        return Err(SmeltError::NotInitialized.into());
    }
    Ok(())
}

/// Check if in a git repository
pub fn check_git_repo(cwd: &Path) -> anyhow::Result<smelt_core::Git2Interface> {
    match smelt_core::Git2Interface::open(&cwd.to_path_buf()) {
        Ok(git) => {
            if git.is_initialized() {
                Ok(git)
            } else {
                Err(SmeltError::EmptyRepository.into())
            }
        }
        Err(_) => Err(SmeltError::NotAGitRepository.into()),
    }
}

/// Format file size in human-readable form
pub fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}

/// Truncate a string with ellipsis
pub fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else if max_len <= 3 {
        "...".to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

/// Format a duration in human-readable form
pub fn format_duration(seconds: u64) -> String {
    if seconds < 60 {
        format!("{}s", seconds)
    } else if seconds < 3600 {
        format!("{}m {}s", seconds / 60, seconds % 60)
    } else {
        format!("{}h {}m", seconds / 3600, (seconds % 3600) / 60)
    }
}
