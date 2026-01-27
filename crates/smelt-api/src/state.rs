//! Application state for the API server

use std::path::PathBuf;

/// Shared application state - just paths, connections created per-request
#[derive(Clone)]
pub struct AppState {
    pub smelt_dir: PathBuf,
    pub db_path: PathBuf,
    pub graph_path: PathBuf,
    pub memory_path: PathBuf,
}

impl AppState {
    pub fn new(smelt_dir: &std::path::Path) -> Self {
        Self {
            smelt_dir: smelt_dir.to_path_buf(),
            db_path: smelt_dir.join("smelt.db"),
            graph_path: smelt_dir.join("graph"),
            memory_path: smelt_dir.join("memory"),
        }
    }
}
