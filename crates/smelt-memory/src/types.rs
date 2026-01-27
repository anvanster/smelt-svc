//! Core types for the memory system

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Outcome of an episode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EpisodeOutcome {
    /// Successfully completed
    Success,
    /// Partially completed
    Partial,
    /// Failed
    Failure,
}

impl EpisodeOutcome {
    /// Convert to a numeric value for scoring
    pub fn score(&self) -> f64 {
        match self {
            EpisodeOutcome::Success => 1.0,
            EpisodeOutcome::Partial => 0.5,
            EpisodeOutcome::Failure => 0.0,
        }
    }
}

/// An episode captures a coding experience for future reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    /// Unique identifier
    pub id: Uuid,

    /// When the episode was captured
    pub created_at: DateTime<Utc>,

    /// Project this episode belongs to
    pub project: Option<String>,

    /// Brief summary of what was accomplished
    pub summary: String,

    /// Type of task (bugfix, feature, refactor, etc.)
    pub task_type: String,

    /// Outcome of the episode
    pub outcome: EpisodeOutcome,

    /// Files that were modified
    pub files_modified: Vec<String>,

    /// Errors encountered and how they were resolved
    pub errors_resolved: Vec<ErrorResolution>,

    /// Domain tags for categorization
    pub tags: Vec<String>,

    /// Associated intent ID (if any)
    pub intent_id: Option<Uuid>,

    /// Associated delta ID (if any)
    pub delta_id: Option<Uuid>,

    /// Git commit SHA (if committed)
    pub commit_sha: Option<String>,

    /// Utility score (updated by feedback and propagation)
    pub utility: f64,

    /// Number of times this episode was helpful
    pub helpful_count: u32,

    /// Total number of feedback events
    pub feedback_count: u32,
}

impl Episode {
    /// Create a new episode
    pub fn new(summary: String, task_type: String, outcome: EpisodeOutcome) -> Self {
        Self {
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            project: None,
            summary,
            task_type,
            outcome,
            files_modified: Vec::new(),
            errors_resolved: Vec::new(),
            tags: Vec::new(),
            intent_id: None,
            delta_id: None,
            commit_sha: None,
            utility: outcome.score(),
            helpful_count: 0,
            feedback_count: 0,
        }
    }

    /// Set the project
    pub fn with_project(mut self, project: String) -> Self {
        self.project = Some(project);
        self
    }

    /// Set files modified
    pub fn with_files(mut self, files: Vec<String>) -> Self {
        self.files_modified = files;
        self
    }

    /// Set errors resolved
    pub fn with_errors(mut self, errors: Vec<ErrorResolution>) -> Self {
        self.errors_resolved = errors;
        self
    }

    /// Set tags
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Set intent ID
    pub fn with_intent(mut self, intent_id: Uuid) -> Self {
        self.intent_id = Some(intent_id);
        self
    }

    /// Set delta ID
    pub fn with_delta(mut self, delta_id: Uuid) -> Self {
        self.delta_id = Some(delta_id);
        self
    }

    /// Set commit SHA
    pub fn with_commit(mut self, sha: String) -> Self {
        self.commit_sha = Some(sha);
        self
    }

    /// Build text representation for embedding
    pub fn to_embedding_text(&self) -> String {
        let mut parts = Vec::new();

        parts.push(self.summary.clone());
        parts.push(format!("Task: {}", self.task_type));

        if !self.tags.is_empty() {
            parts.push(format!("Tags: {}", self.tags.join(", ")));
        }

        if !self.files_modified.is_empty() {
            // Include file names without full paths for better matching
            let file_names: Vec<&str> = self
                .files_modified
                .iter()
                .filter_map(|f| f.split('/').next_back())
                .collect();
            parts.push(format!("Files: {}", file_names.join(", ")));
        }

        for error in &self.errors_resolved {
            parts.push(format!("Error: {} -> {}", error.error, error.resolution));
        }

        parts.join(". ")
    }
}

/// An error that was resolved during the episode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResolution {
    /// The error message or description
    pub error: String,
    /// How the error was resolved
    pub resolution: String,
}

/// Feedback on an episode's helpfulness
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Feedback {
    /// Episode ID
    pub episode_id: Uuid,
    /// When the feedback was given
    pub timestamp: DateTime<Utc>,
    /// Whether the episode was helpful
    pub helpful: bool,
}

/// An episode with retrieval ranking information
#[derive(Debug, Clone)]
pub struct RankedEpisode {
    /// The episode
    pub episode: Episode,
    /// Semantic similarity score (0.0 - 1.0)
    pub similarity: f64,
    /// Combined ranking score
    pub score: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_episode_creation() {
        let episode = Episode::new(
            "Fixed authentication bug".to_string(),
            "bugfix".to_string(),
            EpisodeOutcome::Success,
        );

        assert_eq!(episode.summary, "Fixed authentication bug");
        assert_eq!(episode.task_type, "bugfix");
        assert_eq!(episode.outcome, EpisodeOutcome::Success);
        assert_eq!(episode.utility, 1.0);
    }

    #[test]
    fn test_episode_builder() {
        let episode = Episode::new(
            "Added user profile feature".to_string(),
            "feature".to_string(),
            EpisodeOutcome::Success,
        )
        .with_project("my-app".to_string())
        .with_tags(vec!["auth".to_string(), "user".to_string()])
        .with_files(vec!["src/user.rs".to_string()]);

        assert_eq!(episode.project, Some("my-app".to_string()));
        assert_eq!(episode.tags.len(), 2);
        assert_eq!(episode.files_modified.len(), 1);
    }

    #[test]
    fn test_embedding_text() {
        let episode = Episode::new(
            "Fixed login timeout".to_string(),
            "bugfix".to_string(),
            EpisodeOutcome::Success,
        )
        .with_tags(vec!["auth".to_string()])
        .with_files(vec!["src/auth/login.rs".to_string()])
        .with_errors(vec![ErrorResolution {
            error: "Connection timeout".to_string(),
            resolution: "Increased timeout to 30s".to_string(),
        }]);

        let text = episode.to_embedding_text();
        assert!(text.contains("Fixed login timeout"));
        assert!(text.contains("bugfix"));
        assert!(text.contains("auth"));
        assert!(text.contains("login.rs"));
        assert!(text.contains("Connection timeout"));
    }

    #[test]
    fn test_outcome_score() {
        assert_eq!(EpisodeOutcome::Success.score(), 1.0);
        assert_eq!(EpisodeOutcome::Partial.score(), 0.5);
        assert_eq!(EpisodeOutcome::Failure.score(), 0.0);
    }
}
