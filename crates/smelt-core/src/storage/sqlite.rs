//! SQLite storage implementation for intents and deltas

use crate::{
    types::{IntentId, IntentRecord, IntentStatus, SemanticDelta},
    Result, SmeltError,
};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
use uuid::Uuid;

/// SQLite-based storage for Smelt data
pub struct SqliteStorage {
    conn: Connection,
}

impl SqliteStorage {
    /// Open or create a SQLite database at the given path
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        let storage = Self { conn };
        storage.initialize_schema()?;
        Ok(storage)
    }

    /// Initialize the database schema
    fn initialize_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS intents (
                id TEXT PRIMARY KEY,
                created_at TEXT NOT NULL,
                author_name TEXT NOT NULL,
                author_email TEXT NOT NULL,
                author_type TEXT NOT NULL,
                goal TEXT NOT NULL,
                rationale TEXT,
                constraints TEXT NOT NULL,
                context_links TEXT NOT NULL,
                status TEXT NOT NULL,
                status_data TEXT,
                baseline_snapshot_id TEXT
            );

            CREATE TABLE IF NOT EXISTS deltas (
                id TEXT PRIMARY KEY,
                intent_id TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                from_snapshot TEXT NOT NULL,
                to_snapshot TEXT NOT NULL,
                changes TEXT NOT NULL,
                impact_summary TEXT NOT NULL,
                FOREIGN KEY (intent_id) REFERENCES intents(id)
            );

            CREATE INDEX IF NOT EXISTS idx_intents_status ON intents(status);
            CREATE INDEX IF NOT EXISTS idx_deltas_intent ON deltas(intent_id);
            "#,
        )?;
        Ok(())
    }

    /// Store an intent record
    pub fn store_intent(&self, intent: &IntentRecord) -> Result<()> {
        let (status_str, status_data) = serialize_status(&intent.status);

        self.conn.execute(
            r#"
            INSERT OR REPLACE INTO intents
            (id, created_at, author_name, author_email, author_type, goal, rationale,
             constraints, context_links, status, status_data, baseline_snapshot_id)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            "#,
            params![
                intent.id.to_string(),
                intent.created_at.to_rfc3339(),
                intent.author.name,
                intent.author.email,
                format!("{:?}", intent.author.author_type),
                intent.goal,
                intent.rationale,
                serde_json::to_string(&intent.constraints)?,
                serde_json::to_string(&intent.context_links)?,
                status_str,
                status_data,
                intent.baseline_snapshot_id.map(|id| id.to_string()),
            ],
        )?;
        Ok(())
    }

    /// Get an intent by ID
    pub fn get_intent(&self, id: IntentId) -> Result<Option<IntentRecord>> {
        let result = self
            .conn
            .query_row(
                "SELECT * FROM intents WHERE id = ?1",
                [id.to_string()],
                deserialize_intent,
            )
            .optional()?;

        match result {
            Some(Ok(intent)) => Ok(Some(intent)),
            Some(Err(e)) => Err(e),
            None => Ok(None),
        }
    }

    /// Find an intent by ID prefix
    pub fn find_intent_by_prefix(&self, prefix: &str) -> Result<Option<IntentRecord>> {
        let pattern = format!("{}%", prefix);
        let result = self
            .conn
            .query_row(
                "SELECT * FROM intents WHERE id LIKE ?1 LIMIT 1",
                [pattern],
                deserialize_intent,
            )
            .optional()?;

        match result {
            Some(Ok(intent)) => Ok(Some(intent)),
            Some(Err(e)) => Err(e),
            None => Ok(None),
        }
    }

    /// List intents with optional status filter
    pub fn list_intents(&self, status: Option<IntentStatus>) -> Result<Vec<IntentRecord>> {
        let mut intents = Vec::new();

        let query = if let Some(ref status) = status {
            let (status_str, _) = serialize_status(status);
            format!(
                "SELECT * FROM intents WHERE status = '{}' ORDER BY created_at DESC",
                status_str
            )
        } else {
            "SELECT * FROM intents ORDER BY created_at DESC".to_string()
        };

        let mut stmt = self.conn.prepare(&query)?;
        let rows = stmt.query_map([], deserialize_intent)?;

        for row in rows {
            intents.push(row??);
        }

        Ok(intents)
    }

    /// Update an intent's status
    pub fn update_intent_status(&self, id: IntentId, status: IntentStatus) -> Result<()> {
        let (status_str, status_data) = serialize_status(&status);

        let updated = self.conn.execute(
            "UPDATE intents SET status = ?1, status_data = ?2 WHERE id = ?3",
            params![status_str, status_data, id.to_string()],
        )?;

        if updated == 0 {
            return Err(SmeltError::IntentNotFound(id.to_string()));
        }

        Ok(())
    }

    /// Store a semantic delta
    pub fn store_delta(&self, delta: &SemanticDelta) -> Result<()> {
        self.conn.execute(
            r#"
            INSERT OR REPLACE INTO deltas
            (id, intent_id, timestamp, from_snapshot, to_snapshot, changes, impact_summary)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
            params![
                delta.id.to_string(),
                delta.intent_id.to_string(),
                delta.timestamp.to_rfc3339(),
                delta.from_snapshot.to_string(),
                delta.to_snapshot.to_string(),
                serde_json::to_string(&delta.changes)?,
                serde_json::to_string(&delta.impact_summary)?,
            ],
        )?;
        Ok(())
    }

    /// Get a delta by ID
    pub fn get_delta(&self, id: Uuid) -> Result<Option<SemanticDelta>> {
        let result = self
            .conn
            .query_row(
                "SELECT * FROM deltas WHERE id = ?1",
                [id.to_string()],
                deserialize_delta,
            )
            .optional()?;

        match result {
            Some(Ok(delta)) => Ok(Some(delta)),
            Some(Err(e)) => Err(e),
            None => Ok(None),
        }
    }

    /// Get deltas for an intent
    pub fn get_deltas_for_intent(&self, intent_id: IntentId) -> Result<Vec<SemanticDelta>> {
        let mut deltas = Vec::new();
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM deltas WHERE intent_id = ?1 ORDER BY timestamp DESC")?;

        let rows = stmt.query_map([intent_id.to_string()], deserialize_delta)?;

        for row in rows {
            deltas.push(row??);
        }

        Ok(deltas)
    }

    /// Get all git SHAs that have been committed through Smelt
    pub fn get_committed_shas(&self) -> Result<Vec<String>> {
        let mut shas = Vec::new();
        let mut stmt = self.conn.prepare(
            "SELECT status_data FROM intents WHERE status = 'Committed' AND status_data IS NOT NULL",
        )?;

        let rows = stmt.query_map([], |row| {
            let sha: String = row.get(0)?;
            Ok(sha)
        })?;

        for row in rows {
            shas.push(row?);
        }

        Ok(shas)
    }

    /// Check if a git SHA was committed through Smelt
    pub fn is_sha_tracked(&self, sha: &str) -> Result<bool> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM intents WHERE status = 'Committed' AND status_data = ?1",
            [sha],
            |row| row.get(0),
        )?;

        Ok(count > 0)
    }

    /// Store a synthetic intent for an untracked commit
    pub fn store_synthetic_intent(&self, intent: &IntentRecord, git_sha: &str) -> Result<()> {
        // Store the intent first
        self.store_intent(intent)?;

        // Update to committed status with the git SHA
        self.update_intent_status(
            intent.id,
            IntentStatus::Committed {
                git_sha: git_sha.to_string(),
            },
        )?;

        Ok(())
    }
}

fn serialize_status(status: &IntentStatus) -> (String, Option<String>) {
    match status {
        IntentStatus::Draft => ("Draft".to_string(), None),
        IntentStatus::InProgress => ("InProgress".to_string(), None),
        IntentStatus::PendingValidation => ("PendingValidation".to_string(), None),
        IntentStatus::Validated => ("Validated".to_string(), None),
        IntentStatus::Committed { git_sha } => ("Committed".to_string(), Some(git_sha.clone())),
        IntentStatus::Rejected { violations } => (
            "Rejected".to_string(),
            Some(serde_json::to_string(violations).unwrap_or_default()),
        ),
        IntentStatus::Abandoned => ("Abandoned".to_string(), None),
    }
}

fn deserialize_intent(row: &rusqlite::Row) -> rusqlite::Result<Result<IntentRecord>> {
    use crate::types::{Author, AuthorType, Constraint, ContextLinks};
    use chrono::DateTime;

    let id_str: String = row.get(0)?;
    let created_at_str: String = row.get(1)?;
    let author_name: String = row.get(2)?;
    let author_email: String = row.get(3)?;
    let author_type_str: String = row.get(4)?;
    let goal: String = row.get(5)?;
    let rationale: Option<String> = row.get(6)?;
    let constraints_json: String = row.get(7)?;
    let context_links_json: String = row.get(8)?;
    let status_str: String = row.get(9)?;
    let status_data: Option<String> = row.get(10)?;
    let baseline_snapshot_id_str: Option<String> = row.get(11)?;

    let id = match Uuid::parse_str(&id_str) {
        Ok(id) => id,
        Err(e) => return Ok(Err(SmeltError::Parse(format!("Invalid UUID: {}", e)))),
    };

    let created_at = match DateTime::parse_from_rfc3339(&created_at_str) {
        Ok(dt) => dt.with_timezone(&chrono::Utc),
        Err(e) => return Ok(Err(SmeltError::Parse(format!("Invalid datetime: {}", e)))),
    };

    let author_type = match author_type_str.as_str() {
        "Human" => AuthorType::Human,
        "AI" => AuthorType::AI,
        "Hybrid" => AuthorType::Hybrid,
        _ => AuthorType::Human,
    };

    let constraints: Vec<Constraint> = match serde_json::from_str(&constraints_json) {
        Ok(c) => c,
        Err(e) => return Ok(Err(SmeltError::Serialization(e))),
    };

    let context_links: ContextLinks = match serde_json::from_str(&context_links_json) {
        Ok(c) => c,
        Err(e) => return Ok(Err(SmeltError::Serialization(e))),
    };

    let status = match status_str.as_str() {
        "Draft" => IntentStatus::Draft,
        "InProgress" => IntentStatus::InProgress,
        "PendingValidation" => IntentStatus::PendingValidation,
        "Validated" => IntentStatus::Validated,
        "Committed" => IntentStatus::Committed {
            git_sha: status_data.unwrap_or_default(),
        },
        "Rejected" => IntentStatus::Rejected {
            violations: status_data
                .map(|s| serde_json::from_str(&s).unwrap_or_default())
                .unwrap_or_default(),
        },
        "Abandoned" => IntentStatus::Abandoned,
        _ => IntentStatus::Draft,
    };

    let baseline_snapshot_id = baseline_snapshot_id_str.and_then(|s| Uuid::parse_str(&s).ok());

    Ok(Ok(IntentRecord {
        id,
        created_at,
        author: Author {
            name: author_name,
            email: author_email,
            author_type,
        },
        goal,
        rationale,
        constraints,
        context_links,
        status,
        baseline_snapshot_id,
    }))
}

fn deserialize_delta(row: &rusqlite::Row) -> rusqlite::Result<Result<SemanticDelta>> {
    use crate::types::{ImpactSummary, SemanticChange};
    use chrono::DateTime;

    let id_str: String = row.get(0)?;
    let intent_id_str: String = row.get(1)?;
    let timestamp_str: String = row.get(2)?;
    let from_snapshot_str: String = row.get(3)?;
    let to_snapshot_str: String = row.get(4)?;
    let changes_json: String = row.get(5)?;
    let impact_summary_json: String = row.get(6)?;

    let id = match Uuid::parse_str(&id_str) {
        Ok(id) => id,
        Err(e) => return Ok(Err(SmeltError::Parse(format!("Invalid UUID: {}", e)))),
    };

    let intent_id = match Uuid::parse_str(&intent_id_str) {
        Ok(id) => id,
        Err(e) => return Ok(Err(SmeltError::Parse(format!("Invalid UUID: {}", e)))),
    };

    let timestamp = match DateTime::parse_from_rfc3339(&timestamp_str) {
        Ok(dt) => dt.with_timezone(&chrono::Utc),
        Err(e) => return Ok(Err(SmeltError::Parse(format!("Invalid datetime: {}", e)))),
    };

    let from_snapshot = match Uuid::parse_str(&from_snapshot_str) {
        Ok(id) => id,
        Err(e) => return Ok(Err(SmeltError::Parse(format!("Invalid UUID: {}", e)))),
    };

    let to_snapshot = match Uuid::parse_str(&to_snapshot_str) {
        Ok(id) => id,
        Err(e) => return Ok(Err(SmeltError::Parse(format!("Invalid UUID: {}", e)))),
    };

    let changes: Vec<SemanticChange> = match serde_json::from_str(&changes_json) {
        Ok(c) => c,
        Err(e) => return Ok(Err(SmeltError::Serialization(e))),
    };

    let impact_summary: ImpactSummary = match serde_json::from_str(&impact_summary_json) {
        Ok(s) => s,
        Err(e) => return Ok(Err(SmeltError::Serialization(e))),
    };

    Ok(Ok(SemanticDelta {
        id,
        intent_id,
        timestamp,
        from_snapshot,
        to_snapshot,
        changes,
        impact_summary,
    }))
}
