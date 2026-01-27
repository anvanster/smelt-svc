//! Error types for Smelt

use thiserror::Error;

/// Result type alias for Smelt operations
pub type Result<T> = std::result::Result<T, SmeltError>;

/// Smelt error types
#[derive(Error, Debug)]
pub enum SmeltError {
    // Graph errors
    #[error("Graph error: {0}")]
    Graph(String),

    #[error("Snapshot not found: {0}")]
    SnapshotNotFound(String),

    #[error("Failed to capture snapshot: {0}")]
    SnapshotCapture(String),

    #[error("Failed to compute delta: {0}")]
    DeltaComputation(String),

    #[error("Graph database corrupted: {0}")]
    GraphCorrupted(String),

    // Storage errors
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Intent not found: {0}")]
    IntentNotFound(String),

    #[error("Delta not found: {0}")]
    DeltaNotFound(String),

    #[error("Database corrupted: {0}")]
    DatabaseCorrupted(String),

    #[error("Database locked: another process may be using it")]
    DatabaseLocked,

    // Git errors
    #[error("Git error: {0}")]
    Git(String),

    #[error("Not a git repository")]
    NotAGitRepository,

    #[error("Git author not configured")]
    GitAuthorNotConfigured,

    #[error("Empty repository: no commits found")]
    EmptyRepository,

    #[error("Git reference not found: {0}")]
    GitRefNotFound(String),

    #[error("Working directory has uncommitted changes")]
    UncommittedChanges,

    // Serialization errors
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    // Validation errors
    #[error("Validation failed: {0}")]
    Validation(String),

    #[error("Constraint violation: {0}")]
    ConstraintViolation(String),

    // IO errors
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("File too large: {path} ({size_mb} MB exceeds limit of {limit_mb} MB)")]
    FileTooLarge {
        path: String,
        size_mb: u64,
        limit_mb: u64,
    },

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    // Parse errors
    #[error("Parse error: {0}")]
    Parse(String),

    // Configuration errors
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Smelt not initialized. Run 'smelt init' first.")]
    NotInitialized,

    #[error("Smelt already initialized in this directory")]
    AlreadyInitialized,

    // Recovery errors
    #[error("Recovery failed: {0}")]
    RecoveryFailed(String),

    #[error("Backup corrupted: {0}")]
    BackupCorrupted(String),

    // General errors
    #[error("{0}")]
    Other(String),
}

impl SmeltError {
    /// Returns true if this error is recoverable
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            SmeltError::DatabaseLocked
                | SmeltError::UncommittedChanges
                | SmeltError::NotInitialized
        )
    }

    /// Returns a user-friendly suggestion for fixing the error
    pub fn suggestion(&self) -> Option<&'static str> {
        match self {
            SmeltError::NotInitialized => Some("Run 'smelt init' to initialize Smelt."),
            SmeltError::NotAGitRepository => {
                Some("Initialize a git repository with 'git init' first.")
            }
            SmeltError::GitAuthorNotConfigured => {
                Some("Configure git author with 'git config user.name' and 'git config user.email'.")
            }
            SmeltError::EmptyRepository => Some("Create at least one commit before using Smelt."),
            SmeltError::UncommittedChanges => {
                Some("Commit or stash your changes before proceeding.")
            }
            SmeltError::DatabaseLocked => {
                Some("Close other Smelt processes or wait for them to finish.")
            }
            SmeltError::DatabaseCorrupted(_) => {
                Some("Try 'smelt doctor --fix' to repair the database.")
            }
            SmeltError::GraphCorrupted(_) => {
                Some("Try 'smelt doctor --fix' or restore from backup.")
            }
            SmeltError::BackupCorrupted(_) => Some("Use 'smelt backup verify' to check backups."),
            SmeltError::AlreadyInitialized => Some("Smelt is already set up in this directory."),
            _ => None,
        }
    }
}

impl From<git2::Error> for SmeltError {
    fn from(err: git2::Error) -> Self {
        SmeltError::Git(err.message().to_string())
    }
}

impl From<codegraph::error::GraphError> for SmeltError {
    fn from(err: codegraph::error::GraphError) -> Self {
        SmeltError::Graph(err.to_string())
    }
}
