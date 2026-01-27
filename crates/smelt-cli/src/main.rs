//! Smelt CLI - Semantic version control command-line interface

use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use std::io;

mod commands;
mod ui;

/// Smelt version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const BUILD_DATE: &str = "2024-01-27";

/// Get full version string with build info
pub fn version_string() -> String {
    format!("smelt {} (built {})", VERSION, BUILD_DATE)
}

#[derive(Parser)]
#[command(name = "smelt")]
#[command(about = "Semantic version control for AI-native development")]
#[command(version)]
#[command(propagate_version = true)]
struct Cli {
    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize Smelt in the current repository
    Init {
        /// Wait for indexing to complete (default: background)
        #[arg(long)]
        wait: bool,
    },

    /// Manage intents
    Intent {
        #[command(subcommand)]
        action: IntentAction,
    },

    /// Show current semantic status
    Status {
        /// Show full details including indexing progress
        #[arg(long)]
        full: bool,
    },

    /// Commit with semantic delta
    Commit {
        /// Use existing intent ID
        #[arg(long)]
        intent: Option<String>,

        /// Create inline intent with goal
        #[arg(long)]
        goal: Option<String>,

        /// Skip validation (not recommended)
        #[arg(long)]
        skip_validation: bool,
    },

    /// Validate changes without committing
    Validate {
        /// Validate against specific intent
        #[arg(long)]
        intent: Option<String>,

        /// Use strict validation mode
        #[arg(long)]
        strict: bool,

        /// Show validation configuration
        #[arg(long)]
        show_config: bool,
    },

    /// Manage episodic memory
    Memory {
        #[command(subcommand)]
        action: MemoryAction,
    },

    /// Sync with git history (recover from direct git commits)
    Sync {
        #[command(subcommand)]
        action: Option<SyncAction>,

        /// Show what would be done without making changes
        #[arg(long)]
        dry_run: bool,

        /// Number of commits to scan (default: 50)
        #[arg(long, default_value = "50")]
        limit: usize,
    },

    /// Diagnose and repair Smelt installation
    Doctor {
        /// Attempt automatic repairs for fixable issues
        #[arg(long)]
        fix: bool,

        /// Show detailed diagnostic information
        #[arg(long)]
        verbose: bool,
    },

    /// Backup and restore Smelt data
    Backup {
        #[command(subcommand)]
        action: BackupAction,
    },

    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },
}

#[derive(Subcommand)]
enum IntentAction {
    /// Create a new intent
    Create {
        /// Intent goal
        #[arg(long)]
        goal: String,

        /// Optional rationale
        #[arg(long)]
        rationale: Option<String>,
    },

    /// List intents
    List {
        /// Filter by status
        #[arg(long)]
        status: Option<String>,
    },

    /// Show intent details
    Show {
        /// Intent ID (can be partial)
        id: String,
    },
}

#[derive(Subcommand)]
enum MemoryAction {
    /// Search for relevant past experiences
    Search {
        /// Search query
        query: String,

        /// Maximum results to return
        #[arg(long, default_value = "5")]
        limit: usize,
    },

    /// Record feedback for an episode
    Feedback {
        /// Episode ID
        episode_id: String,

        /// Mark as helpful
        #[arg(long, group = "feedback_type")]
        helpful: bool,

        /// Mark as not helpful
        #[arg(long, group = "feedback_type")]
        not_helpful: bool,
    },

    /// Show memory statistics
    Stats,

    /// Run utility propagation
    Propagate {
        /// Include temporal credit assignment
        #[arg(long)]
        temporal: bool,
    },

    /// List all episodes
    List {
        /// Maximum episodes to show
        #[arg(long, default_value = "10")]
        limit: usize,
    },

    /// Capture an episode manually (for testing)
    Capture {
        /// Episode summary
        #[arg(long)]
        summary: String,

        /// Task type (bugfix, feature, refactor, test, docs, research, debug, setup)
        #[arg(long, default_value = "feature")]
        task_type: String,

        /// Outcome (success, partial, failure)
        #[arg(long, default_value = "success")]
        outcome: String,

        /// Tags for categorization
        #[arg(long)]
        tags: Vec<String>,
    },
}

#[derive(Subcommand)]
enum SyncAction {
    /// Show sync status
    Status,
}

#[derive(Subcommand)]
enum BackupAction {
    /// Create a backup
    Create {
        /// Output file path (default: smelt-backup-{timestamp}.tar)
        #[arg(long, short)]
        output: Option<std::path::PathBuf>,

        /// Include graph data (can be large)
        #[arg(long)]
        include_graph: bool,
    },

    /// Restore from a backup
    Restore {
        /// Backup file to restore from
        backup_file: std::path::PathBuf,

        /// Overwrite existing Smelt data
        #[arg(long)]
        force: bool,
    },

    /// List contents of a backup
    List {
        /// Backup file to list
        backup_file: std::path::PathBuf,
    },

    /// Verify a backup file
    Verify {
        /// Backup file to verify
        backup_file: std::path::PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize tracing
    let filter = if cli.verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::new("info")
    };

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(filter)
        .init();

    match cli.command {
        Commands::Init { wait } => commands::init::run(wait).await,
        Commands::Intent { action } => match action {
            IntentAction::Create { goal, rationale } => {
                commands::intent::create(goal, rationale).await
            }
            IntentAction::List { status } => commands::intent::list(status).await,
            IntentAction::Show { id } => commands::intent::show(id).await,
        },
        Commands::Status { full } => commands::status::run(full).await,
        Commands::Commit {
            intent,
            goal,
            skip_validation,
        } => commands::commit::run(intent, goal, skip_validation).await,
        Commands::Validate {
            intent,
            strict,
            show_config,
        } => commands::validate::run(intent, strict, show_config).await,
        Commands::Memory { action } => match action {
            MemoryAction::Search { query, limit } => {
                commands::memory::search(query, limit).await
            }
            MemoryAction::Feedback {
                episode_id,
                helpful,
                not_helpful,
            } => {
                let is_helpful = helpful || !not_helpful;
                commands::memory::feedback(episode_id, is_helpful).await
            }
            MemoryAction::Stats => commands::memory::stats().await,
            MemoryAction::Propagate { temporal } => {
                commands::memory::propagate(temporal).await
            }
            MemoryAction::List { limit } => commands::memory::list(limit).await,
            MemoryAction::Capture {
                summary,
                task_type,
                outcome,
                tags,
            } => commands::memory::capture(summary, task_type, outcome, tags).await,
        },
        Commands::Sync { action, dry_run, limit } => match action {
            Some(SyncAction::Status) => commands::sync::status().await,
            None => commands::sync::run(dry_run, limit).await,
        },
        Commands::Doctor { fix, verbose } => {
            if verbose {
                commands::doctor::verbose().await
            } else {
                commands::doctor::run(fix).await
            }
        }
        Commands::Backup { action } => match action {
            BackupAction::Create { output, include_graph } => {
                commands::backup::create(output, include_graph).await
            }
            BackupAction::Restore { backup_file, force } => {
                commands::backup::restore(backup_file, force).await
            }
            BackupAction::List { backup_file } => {
                commands::backup::list(backup_file).await
            }
            BackupAction::Verify { backup_file } => {
                commands::backup::verify(backup_file).await
            }
        },
        Commands::Completions { shell } => {
            let mut cmd = Cli::command();
            let name = cmd.get_name().to_string();
            generate(shell, &mut cmd, name, &mut io::stdout());
            Ok(())
        }
    }
}
