//! SQLite storage for episode metadata

use crate::error::{MemoryError, MemoryResult};
use crate::types::{Episode, EpisodeOutcome, Feedback};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

/// SQLite-based storage for episode metadata
pub struct EpisodeStorage {
    conn: Arc<Mutex<Connection>>,
}

impl EpisodeStorage {
    /// Open or create a storage database
    pub fn open(path: &Path) -> MemoryResult<Self> {
        let conn = Connection::open(path)?;
        let storage = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        storage.init_schema()?;
        Ok(storage)
    }

    /// Create an in-memory storage (for testing)
    pub fn in_memory() -> MemoryResult<Self> {
        let conn = Connection::open_in_memory()?;
        let storage = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        storage.init_schema()?;
        Ok(storage)
    }

    /// Initialize the database schema
    fn init_schema(&self) -> MemoryResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MemoryError::Storage(format!("Failed to acquire lock: {}", e)))?;

        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS episodes (
                id TEXT PRIMARY KEY,
                created_at TEXT NOT NULL,
                project TEXT,
                summary TEXT NOT NULL,
                task_type TEXT NOT NULL,
                outcome TEXT NOT NULL,
                files_modified TEXT NOT NULL,
                errors_resolved TEXT NOT NULL,
                tags TEXT NOT NULL,
                intent_id TEXT,
                delta_id TEXT,
                commit_sha TEXT,
                utility REAL NOT NULL DEFAULT 0.5,
                helpful_count INTEGER NOT NULL DEFAULT 0,
                feedback_count INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS feedback (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                episode_id TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                helpful INTEGER NOT NULL,
                FOREIGN KEY (episode_id) REFERENCES episodes(id)
            );

            CREATE INDEX IF NOT EXISTS idx_episodes_project ON episodes(project);
            CREATE INDEX IF NOT EXISTS idx_episodes_task_type ON episodes(task_type);
            CREATE INDEX IF NOT EXISTS idx_episodes_created_at ON episodes(created_at);
            CREATE INDEX IF NOT EXISTS idx_feedback_episode_id ON feedback(episode_id);
            "#,
        )?;

        Ok(())
    }

    /// Store an episode
    pub fn store_episode(&self, episode: &Episode) -> MemoryResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MemoryError::Storage(format!("Failed to acquire lock: {}", e)))?;

        conn.execute(
            r#"
            INSERT OR REPLACE INTO episodes
            (id, created_at, project, summary, task_type, outcome,
             files_modified, errors_resolved, tags, intent_id, delta_id,
             commit_sha, utility, helpful_count, feedback_count)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
            "#,
            params![
                episode.id.to_string(),
                episode.created_at.to_rfc3339(),
                episode.project,
                episode.summary,
                episode.task_type,
                outcome_to_str(&episode.outcome),
                serde_json::to_string(&episode.files_modified)?,
                serde_json::to_string(&episode.errors_resolved)?,
                serde_json::to_string(&episode.tags)?,
                episode.intent_id.map(|id| id.to_string()),
                episode.delta_id.map(|id| id.to_string()),
                episode.commit_sha,
                episode.utility,
                episode.helpful_count,
                episode.feedback_count,
            ],
        )?;

        Ok(())
    }

    /// Get an episode by ID
    pub fn get_episode(&self, id: Uuid) -> MemoryResult<Option<Episode>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MemoryError::Storage(format!("Failed to acquire lock: {}", e)))?;

        let mut stmt = conn.prepare(
            r#"
            SELECT id, created_at, project, summary, task_type, outcome,
                   files_modified, errors_resolved, tags, intent_id, delta_id,
                   commit_sha, utility, helpful_count, feedback_count
            FROM episodes WHERE id = ?1
            "#,
        )?;

        let result = stmt
            .query_row([id.to_string()], |row| Ok(row_to_episode_raw(row)))
            .optional()?;

        match result {
            Some(ep) => Ok(Some(ep)),
            None => Ok(None),
        }
    }

    /// List all episodes, optionally filtered by project
    pub fn list_episodes(&self, project: Option<&str>) -> MemoryResult<Vec<Episode>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MemoryError::Storage(format!("Failed to acquire lock: {}", e)))?;

        let mut episodes = Vec::new();

        if let Some(proj) = project {
            let mut stmt = conn.prepare(
                r#"
                SELECT id, created_at, project, summary, task_type, outcome,
                       files_modified, errors_resolved, tags, intent_id, delta_id,
                       commit_sha, utility, helpful_count, feedback_count
                FROM episodes WHERE project = ?1
                ORDER BY created_at DESC
                "#,
            )?;

            let rows = stmt.query_map([proj], |row| Ok(row_to_episode_raw(row)))?;

            for row in rows {
                episodes.push(row?);
            }
        } else {
            let mut stmt = conn.prepare(
                r#"
                SELECT id, created_at, project, summary, task_type, outcome,
                       files_modified, errors_resolved, tags, intent_id, delta_id,
                       commit_sha, utility, helpful_count, feedback_count
                FROM episodes ORDER BY created_at DESC
                "#,
            )?;

            let rows = stmt.query_map([], |row| Ok(row_to_episode_raw(row)))?;

            for row in rows {
                episodes.push(row?);
            }
        }

        Ok(episodes)
    }

    /// Update episode utility score
    pub fn update_utility(&self, id: Uuid, utility: f64) -> MemoryResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MemoryError::Storage(format!("Failed to acquire lock: {}", e)))?;

        conn.execute(
            "UPDATE episodes SET utility = ?1 WHERE id = ?2",
            params![utility, id.to_string()],
        )?;

        Ok(())
    }

    /// Record feedback for an episode
    pub fn record_feedback(&self, episode_id: Uuid, helpful: bool) -> MemoryResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MemoryError::Storage(format!("Failed to acquire lock: {}", e)))?;

        // Insert feedback record
        conn.execute(
            r#"
            INSERT INTO feedback (episode_id, timestamp, helpful)
            VALUES (?1, ?2, ?3)
            "#,
            params![
                episode_id.to_string(),
                Utc::now().to_rfc3339(),
                helpful as i32,
            ],
        )?;

        // Update episode counts
        if helpful {
            conn.execute(
                r#"
                UPDATE episodes
                SET helpful_count = helpful_count + 1,
                    feedback_count = feedback_count + 1
                WHERE id = ?1
                "#,
                [episode_id.to_string()],
            )?;
        } else {
            conn.execute(
                "UPDATE episodes SET feedback_count = feedback_count + 1 WHERE id = ?1",
                [episode_id.to_string()],
            )?;
        }

        Ok(())
    }

    /// Get all feedback for an episode
    pub fn get_feedback(&self, episode_id: Uuid) -> MemoryResult<Vec<Feedback>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MemoryError::Storage(format!("Failed to acquire lock: {}", e)))?;

        let mut stmt = conn
            .prepare("SELECT episode_id, timestamp, helpful FROM feedback WHERE episode_id = ?1")?;

        let mut feedback = Vec::new();
        let rows = stmt.query_map([episode_id.to_string()], |row| {
            let episode_id_str: String = row.get(0)?;
            let timestamp_str: String = row.get(1)?;
            let helpful: i32 = row.get(2)?;

            Ok(Feedback {
                episode_id: Uuid::parse_str(&episode_id_str).unwrap_or(Uuid::nil()),
                timestamp: DateTime::parse_from_rfc3339(&timestamp_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                helpful: helpful != 0,
            })
        })?;

        for row in rows {
            feedback.push(row?);
        }

        Ok(feedback)
    }

    /// Get episode IDs for utility propagation
    pub fn get_all_episode_ids(&self) -> MemoryResult<Vec<Uuid>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MemoryError::Storage(format!("Failed to acquire lock: {}", e)))?;

        let mut stmt = conn.prepare("SELECT id FROM episodes")?;
        let mut ids = Vec::new();

        let rows = stmt.query_map([], |row| {
            let id_str: String = row.get(0)?;
            Ok(Uuid::parse_str(&id_str).unwrap_or(Uuid::nil()))
        })?;

        for row in rows {
            ids.push(row?);
        }

        Ok(ids)
    }

    /// Get statistics about the memory
    pub fn get_stats(&self, project: Option<&str>) -> MemoryResult<MemoryStats> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MemoryError::Storage(format!("Failed to acquire lock: {}", e)))?;

        let (total_episodes, total_feedback, avg_utility) = if let Some(proj) = project {
            let mut stmt = conn.prepare(
                r#"
                SELECT COUNT(*), SUM(feedback_count), AVG(utility)
                FROM episodes WHERE project = ?1
                "#,
            )?;
            stmt.query_row([proj], |row| {
                Ok((
                    row.get::<_, i64>(0)? as usize,
                    row.get::<_, Option<i64>>(1)?.unwrap_or(0) as usize,
                    row.get::<_, Option<f64>>(2)?.unwrap_or(0.0),
                ))
            })?
        } else {
            let mut stmt =
                conn.prepare("SELECT COUNT(*), SUM(feedback_count), AVG(utility) FROM episodes")?;
            stmt.query_row([], |row| {
                Ok((
                    row.get::<_, i64>(0)? as usize,
                    row.get::<_, Option<i64>>(1)?.unwrap_or(0) as usize,
                    row.get::<_, Option<f64>>(2)?.unwrap_or(0.0),
                ))
            })?
        };

        Ok(MemoryStats {
            total_episodes,
            total_feedback,
            avg_utility,
        })
    }
}

/// Memory statistics
#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub total_episodes: usize,
    pub total_feedback: usize,
    pub avg_utility: f64,
}

fn outcome_to_str(outcome: &EpisodeOutcome) -> &'static str {
    match outcome {
        EpisodeOutcome::Success => "success",
        EpisodeOutcome::Partial => "partial",
        EpisodeOutcome::Failure => "failure",
    }
}

fn str_to_outcome(s: &str) -> EpisodeOutcome {
    match s {
        "success" => EpisodeOutcome::Success,
        "partial" => EpisodeOutcome::Partial,
        "failure" => EpisodeOutcome::Failure,
        _ => EpisodeOutcome::Partial,
    }
}

/// Convert a row to an Episode - infallible version that uses defaults for parse errors
fn row_to_episode_raw(row: &rusqlite::Row) -> Episode {
    let id_str: String = row.get(0).unwrap_or_default();
    let created_at_str: String = row.get(1).unwrap_or_default();
    let project: Option<String> = row.get(2).ok();
    let summary: String = row.get(3).unwrap_or_default();
    let task_type: String = row.get(4).unwrap_or_default();
    let outcome_str: String = row.get(5).unwrap_or_default();
    let files_json: String = row.get(6).unwrap_or_default();
    let errors_json: String = row.get(7).unwrap_or_default();
    let tags_json: String = row.get(8).unwrap_or_default();
    let intent_id_str: Option<String> = row.get(9).ok();
    let delta_id_str: Option<String> = row.get(10).ok();
    let commit_sha: Option<String> = row.get(11).ok();
    let utility: f64 = row.get(12).unwrap_or(0.5);
    let helpful_count: u32 = row.get(13).unwrap_or(0);
    let feedback_count: u32 = row.get(14).unwrap_or(0);

    Episode {
        id: Uuid::parse_str(&id_str).unwrap_or(Uuid::nil()),
        created_at: DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now()),
        project,
        summary,
        task_type,
        outcome: str_to_outcome(&outcome_str),
        files_modified: serde_json::from_str(&files_json).unwrap_or_default(),
        errors_resolved: serde_json::from_str(&errors_json).unwrap_or_default(),
        tags: serde_json::from_str(&tags_json).unwrap_or_default(),
        intent_id: intent_id_str.and_then(|s| Uuid::parse_str(&s).ok()),
        delta_id: delta_id_str.and_then(|s| Uuid::parse_str(&s).ok()),
        commit_sha,
        utility,
        helpful_count,
        feedback_count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_and_get_episode() {
        let storage = EpisodeStorage::in_memory().unwrap();

        let episode = Episode::new(
            "Test episode".to_string(),
            "bugfix".to_string(),
            EpisodeOutcome::Success,
        )
        .with_project("test-project".to_string())
        .with_tags(vec!["rust".to_string()]);

        storage.store_episode(&episode).unwrap();

        let retrieved = storage.get_episode(episode.id).unwrap().unwrap();
        assert_eq!(retrieved.summary, "Test episode");
        assert_eq!(retrieved.project, Some("test-project".to_string()));
    }

    #[test]
    fn test_list_episodes() {
        let storage = EpisodeStorage::in_memory().unwrap();

        let ep1 = Episode::new(
            "Episode 1".to_string(),
            "feature".to_string(),
            EpisodeOutcome::Success,
        )
        .with_project("proj-a".to_string());
        let ep2 = Episode::new(
            "Episode 2".to_string(),
            "bugfix".to_string(),
            EpisodeOutcome::Success,
        )
        .with_project("proj-b".to_string());

        storage.store_episode(&ep1).unwrap();
        storage.store_episode(&ep2).unwrap();

        // List all
        let all = storage.list_episodes(None).unwrap();
        assert_eq!(all.len(), 2);

        // List by project
        let proj_a = storage.list_episodes(Some("proj-a")).unwrap();
        assert_eq!(proj_a.len(), 1);
        assert_eq!(proj_a[0].summary, "Episode 1");
    }

    #[test]
    fn test_feedback() {
        let storage = EpisodeStorage::in_memory().unwrap();

        let episode = Episode::new(
            "Test".to_string(),
            "test".to_string(),
            EpisodeOutcome::Success,
        );
        storage.store_episode(&episode).unwrap();

        // Record positive feedback
        storage.record_feedback(episode.id, true).unwrap();
        storage.record_feedback(episode.id, true).unwrap();
        storage.record_feedback(episode.id, false).unwrap();

        let updated = storage.get_episode(episode.id).unwrap().unwrap();
        assert_eq!(updated.helpful_count, 2);
        assert_eq!(updated.feedback_count, 3);

        let feedback = storage.get_feedback(episode.id).unwrap();
        assert_eq!(feedback.len(), 3);
    }

    #[test]
    fn test_update_utility() {
        let storage = EpisodeStorage::in_memory().unwrap();

        let episode = Episode::new(
            "Test".to_string(),
            "test".to_string(),
            EpisodeOutcome::Success,
        );
        storage.store_episode(&episode).unwrap();

        storage.update_utility(episode.id, 0.85).unwrap();

        let updated = storage.get_episode(episode.id).unwrap().unwrap();
        assert!((updated.utility - 0.85).abs() < 0.001);
    }

    #[test]
    fn test_stats() {
        let storage = EpisodeStorage::in_memory().unwrap();

        let ep1 = Episode::new(
            "Ep1".to_string(),
            "test".to_string(),
            EpisodeOutcome::Success,
        );
        let ep2 = Episode::new(
            "Ep2".to_string(),
            "test".to_string(),
            EpisodeOutcome::Partial,
        );

        storage.store_episode(&ep1).unwrap();
        storage.store_episode(&ep2).unwrap();

        let stats = storage.get_stats(None).unwrap();
        assert_eq!(stats.total_episodes, 2);
    }
}
