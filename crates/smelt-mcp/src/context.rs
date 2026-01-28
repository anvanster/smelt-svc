//! SmeltContext - Shared state for MCP server operations

use crate::error::{McpError, McpResult};
use smelt_core::{SmeltGraph, SqliteStorage};
use smelt_memory::SmeltMemory;
use smelt_validator::SmeltValidator;
use std::path::{Path, PathBuf};

/// Smelt directory name
const SMELT_DIR: &str = ".smelt";

/// Shared context for MCP server operations
pub struct SmeltContext {
    /// Current working directory
    working_dir: PathBuf,

    /// Path to .smelt directory (if initialized)
    smelt_dir: Option<PathBuf>,

    /// SQLite storage for intents and deltas
    storage: Option<SqliteStorage>,

    /// Code graph
    graph: Option<SmeltGraph>,

    /// Episodic memory
    memory: Option<SmeltMemory>,

    /// Validator
    validator: Option<SmeltValidator>,

    /// Current project name
    project: Option<String>,
}

impl SmeltContext {
    /// Create a new uninitialized context
    pub fn new() -> Self {
        let working_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let project = working_dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string());

        Self {
            working_dir,
            smelt_dir: None,
            storage: None,
            graph: None,
            memory: None,
            validator: None,
            project,
        }
    }

    /// Check if Smelt is initialized in the given directory
    pub fn is_initialized(&self) -> bool {
        self.smelt_dir.is_some() && self.storage.is_some()
    }

    /// Get the working directory
    pub fn working_dir(&self) -> &Path {
        &self.working_dir
    }

    /// Get the project name
    pub fn project(&self) -> Option<&str> {
        self.project.as_deref()
    }

    /// Get the smelt directory path
    pub fn smelt_dir(&self) -> Option<&Path> {
        self.smelt_dir.as_deref()
    }

    /// Try to load context from an existing .smelt directory
    pub fn try_load(&mut self) -> McpResult<bool> {
        let smelt_dir = self.working_dir.join(SMELT_DIR);

        if !smelt_dir.exists() {
            return Ok(false);
        }

        self.load_from_path(&smelt_dir)?;
        Ok(true)
    }

    /// Initialize Smelt in the given path
    pub fn initialize(&mut self, path: Option<&Path>) -> McpResult<()> {
        let repo_path = path
            .map(PathBuf::from)
            .unwrap_or_else(|| self.working_dir.clone());
        let smelt_dir = repo_path.join(SMELT_DIR);

        // Create .smelt directory
        std::fs::create_dir_all(&smelt_dir)?;

        // Initialize components
        self.load_from_path(&smelt_dir)?;

        Ok(())
    }

    /// Load context from an existing .smelt directory
    fn load_from_path(&mut self, smelt_dir: &Path) -> McpResult<()> {
        // Load storage
        let db_path = smelt_dir.join("smelt.db");
        let storage = SqliteStorage::open(&db_path)?;
        self.storage = Some(storage);

        // Load graph
        let graph_path = smelt_dir.join("graph");
        std::fs::create_dir_all(&graph_path)?;
        let graph = SmeltGraph::open(&graph_path)?;
        self.graph = Some(graph);

        // Load memory
        let memory_path = smelt_dir.join("memory");
        std::fs::create_dir_all(&memory_path)?;
        let memory = SmeltMemory::open(&memory_path)?;
        self.memory = if let Some(ref project) = self.project {
            Some(memory.with_project(project.clone()))
        } else {
            Some(memory)
        };

        // Load validator
        let validator = SmeltValidator::from_smelt_dir(smelt_dir);
        self.validator = Some(validator);

        self.smelt_dir = Some(smelt_dir.to_path_buf());

        Ok(())
    }

    /// Get the storage (requires initialization)
    pub fn storage(&self) -> McpResult<&SqliteStorage> {
        self.storage.as_ref().ok_or(McpError::NotInitialized)
    }

    /// Get mutable storage (requires initialization)
    pub fn storage_mut(&mut self) -> McpResult<&mut SqliteStorage> {
        self.storage.as_mut().ok_or(McpError::NotInitialized)
    }

    /// Get the graph (requires initialization)
    pub fn graph(&self) -> McpResult<&SmeltGraph> {
        self.graph.as_ref().ok_or(McpError::NotInitialized)
    }

    /// Get mutable graph (requires initialization)
    pub fn graph_mut(&mut self) -> McpResult<&mut SmeltGraph> {
        self.graph.as_mut().ok_or(McpError::NotInitialized)
    }

    /// Get the memory system (requires initialization)
    pub fn memory(&self) -> McpResult<&SmeltMemory> {
        self.memory.as_ref().ok_or(McpError::NotInitialized)
    }

    /// Get mutable memory system (requires initialization)
    pub fn memory_mut(&mut self) -> McpResult<&mut SmeltMemory> {
        self.memory.as_mut().ok_or(McpError::NotInitialized)
    }

    /// Get the validator (requires initialization)
    pub fn validator(&self) -> McpResult<&SmeltValidator> {
        self.validator.as_ref().ok_or(McpError::NotInitialized)
    }

    /// Ensure context is initialized
    pub fn ensure_initialized(&self) -> McpResult<()> {
        if !self.is_initialized() {
            return Err(McpError::NotInitialized);
        }
        Ok(())
    }
}

impl Default for SmeltContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_new_context() {
        let ctx = SmeltContext::new();
        assert!(!ctx.is_initialized());
        assert!(ctx.project().is_some());
    }

    #[test]
    fn test_initialize() {
        let dir = tempdir().unwrap();
        let mut ctx = SmeltContext::new();

        // Override working dir for test
        ctx.working_dir = dir.path().to_path_buf();
        ctx.project = Some("test-project".to_string());

        ctx.initialize(None).unwrap();

        assert!(ctx.is_initialized());
        assert!(ctx.smelt_dir().unwrap().exists());
        assert!(ctx.storage().is_ok());
        assert!(ctx.graph().is_ok());
        assert!(ctx.memory().is_ok());
        assert!(ctx.validator().is_ok());
    }

    #[test]
    fn test_try_load_not_initialized() {
        let dir = tempdir().unwrap();
        let mut ctx = SmeltContext::new();
        ctx.working_dir = dir.path().to_path_buf();

        let loaded = ctx.try_load().unwrap();
        assert!(!loaded);
        assert!(!ctx.is_initialized());
    }

    #[test]
    fn test_try_load_existing() {
        let dir = tempdir().unwrap();

        // First initialize
        {
            let mut ctx = SmeltContext::new();
            ctx.working_dir = dir.path().to_path_buf();
            ctx.initialize(None).unwrap();
        }

        // Then try to load
        let mut ctx = SmeltContext::new();
        ctx.working_dir = dir.path().to_path_buf();
        let loaded = ctx.try_load().unwrap();

        assert!(loaded);
        assert!(ctx.is_initialized());
    }
}
