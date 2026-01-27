//! SmeltGraph - wrapper around CodeGraph with snapshot and versioning capabilities

use crate::{
    types::{GraphSnapshot, IntentId, SnapshotId},
    Result, SmeltError,
};
use codegraph::CodeGraph;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use uuid::Uuid;

/// SmeltGraph wraps CodeGraph with snapshot and versioning capabilities
pub struct SmeltGraph {
    /// The underlying CodeGraph
    inner: CodeGraph,

    /// Snapshots indexed by commit SHA
    snapshots: BTreeMap<String, GraphSnapshot>,

    /// Intent to node mapping (which nodes were affected by which intent)
    intent_map: HashMap<IntentId, Vec<u64>>,

    /// Tracked files in the graph
    tracked_files: Vec<String>,

    /// Path to graph storage
    storage_path: std::path::PathBuf,
}

impl SmeltGraph {
    /// Open or create a SmeltGraph at the given path
    pub fn open(path: &Path) -> Result<Self> {
        // Initialize CodeGraph with RocksDB storage
        let inner = CodeGraph::open(path)?;

        Ok(Self {
            inner,
            snapshots: BTreeMap::new(),
            intent_map: HashMap::new(),
            tracked_files: Vec::new(),
            storage_path: path.to_path_buf(),
        })
    }

    /// Get the number of nodes in the graph
    pub fn node_count(&self) -> usize {
        self.inner.node_count()
    }

    /// Get the number of edges in the graph
    pub fn edge_count(&self) -> usize {
        self.inner.edge_count()
    }

    /// Get node counts grouped by type
    fn nodes_by_kind(&self) -> HashMap<String, usize> {
        // Query the graph to count nodes by type
        // For now, return empty - full implementation requires CodeGraph query
        // TODO: Implement using self.inner.query() to count by NodeType
        HashMap::new()
    }

    /// Get list of tracked files
    fn files(&self) -> Vec<String> {
        self.tracked_files.clone()
    }

    /// Capture a snapshot of the current graph state
    pub fn snapshot(&self) -> Result<GraphSnapshot> {
        let node_count = self.inner.node_count();
        let edge_count = self.inner.edge_count();

        // Get node counts by kind
        let nodes_by_kind = self.nodes_by_kind();

        // Get file list from graph
        let files = self.files();

        // Compute checksum for integrity
        let checksum = self.compute_checksum();

        Ok(GraphSnapshot {
            id: Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            git_sha: None,
            node_count,
            edge_count,
            nodes_by_kind,
            files,
            checksum,
        })
    }

    /// Create a snapshot for an intent (baseline capture)
    pub fn snapshot_for_intent(&mut self, intent_id: IntentId) -> Result<SnapshotId> {
        let snapshot = self.snapshot()?;
        let snapshot_id = snapshot.id;

        // Initialize intent tracking
        self.intent_map.insert(intent_id, Vec::new());

        Ok(snapshot_id)
    }

    /// Store a snapshot associated with a git commit
    pub fn store_snapshot(&mut self, git_sha: String, mut snapshot: GraphSnapshot) {
        snapshot.git_sha = Some(git_sha.clone());
        self.snapshots.insert(git_sha, snapshot);
    }

    /// Get a snapshot by git SHA
    pub fn get_snapshot(&self, git_sha: &str) -> Option<&GraphSnapshot> {
        self.snapshots.get(git_sha)
    }

    /// Get a snapshot by ID
    pub fn get_snapshot_by_id(&self, id: SnapshotId) -> Option<&GraphSnapshot> {
        self.snapshots.values().find(|s| s.id == id)
    }

    /// Query the graph state at a specific commit
    pub fn query_at_commit(&self, commit: &str) -> Result<&GraphSnapshot> {
        self.snapshots
            .get(commit)
            .ok_or_else(|| SmeltError::SnapshotNotFound(commit.to_string()))
    }

    /// Index files into the graph
    /// Note: This is a placeholder - full implementation requires CodeGraph parser integration
    pub fn index_files(&mut self, files: &[std::path::PathBuf]) -> Result<()> {
        // TODO: Integrate with codegraph-parser-api to parse files and add nodes/edges
        // For now, just track the files
        for file in files {
            if let Some(path_str) = file.to_str() {
                if !self.tracked_files.contains(&path_str.to_string()) {
                    self.tracked_files.push(path_str.to_string());
                }
            }
        }
        Ok(())
    }

    /// Get nodes affected by an intent
    pub fn nodes_for_intent(&self, intent_id: IntentId) -> Option<&Vec<u64>> {
        self.intent_map.get(&intent_id)
    }

    /// Record that nodes were modified by an intent
    pub fn record_intent_nodes(&mut self, intent_id: IntentId, node_ids: Vec<u64>) {
        self.intent_map
            .entry(intent_id)
            .or_default()
            .extend(node_ids);
    }

    /// Access the underlying CodeGraph
    pub fn inner(&self) -> &CodeGraph {
        &self.inner
    }

    /// Compute a checksum of the current graph state
    fn compute_checksum(&self) -> String {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.inner.node_count().hash(&mut hasher);
        self.inner.edge_count().hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_open_and_snapshot() {
        let dir = tempdir().unwrap();
        let graph = SmeltGraph::open(dir.path()).unwrap();

        let snapshot = graph.snapshot().unwrap();
        assert_eq!(snapshot.node_count, 0);
        assert_eq!(snapshot.edge_count, 0);
    }

    #[test]
    fn test_snapshot_for_intent() {
        let dir = tempdir().unwrap();
        let mut graph = SmeltGraph::open(dir.path()).unwrap();

        let intent_id = Uuid::new_v4();
        let snapshot_id = graph.snapshot_for_intent(intent_id).unwrap();

        assert!(graph.intent_map.contains_key(&intent_id));
        assert!(!snapshot_id.is_nil());
    }
}
