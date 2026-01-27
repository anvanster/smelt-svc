//! Vector storage for episode embeddings

use crate::error::{MemoryError, MemoryResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// In-memory vector store with persistence
pub struct VectorStore {
    /// Path for persistence (None for in-memory only)
    path: Option<PathBuf>,
    /// Vectors indexed by episode ID
    vectors: HashMap<Uuid, Vec<f32>>,
    /// Embedding dimension
    dimension: usize,
}

/// Serializable format for persistence
#[derive(Serialize, Deserialize)]
struct VectorStoreData {
    dimension: usize,
    vectors: Vec<(String, Vec<f32>)>,
}

impl VectorStore {
    /// Create a new vector store
    pub fn new(dimension: usize) -> Self {
        Self {
            path: None,
            vectors: HashMap::new(),
            dimension,
        }
    }

    /// Open or create a persistent vector store
    pub fn open(path: &Path, dimension: usize) -> MemoryResult<Self> {
        let mut store = Self {
            path: Some(path.to_path_buf()),
            vectors: HashMap::new(),
            dimension,
        };

        // Load existing data if file exists
        if path.exists() {
            let data = fs::read_to_string(path)?;
            let stored: VectorStoreData = serde_json::from_str(&data)?;

            if stored.dimension != dimension {
                return Err(MemoryError::InvalidConfig(format!(
                    "Dimension mismatch: stored={}, expected={}",
                    stored.dimension, dimension
                )));
            }

            for (id_str, vec) in stored.vectors {
                if let Ok(id) = Uuid::parse_str(&id_str) {
                    store.vectors.insert(id, vec);
                }
            }
        }

        Ok(store)
    }

    /// Store a vector for an episode
    pub fn store(&mut self, episode_id: Uuid, vector: Vec<f32>) -> MemoryResult<()> {
        if vector.len() != self.dimension {
            return Err(MemoryError::InvalidConfig(format!(
                "Vector dimension mismatch: got={}, expected={}",
                vector.len(),
                self.dimension
            )));
        }

        self.vectors.insert(episode_id, vector);
        self.persist()?;
        Ok(())
    }

    /// Get a vector by episode ID
    pub fn get(&self, episode_id: Uuid) -> Option<&Vec<f32>> {
        self.vectors.get(&episode_id)
    }

    /// Remove a vector
    pub fn remove(&mut self, episode_id: Uuid) -> MemoryResult<()> {
        self.vectors.remove(&episode_id);
        self.persist()?;
        Ok(())
    }

    /// Search for similar vectors using cosine similarity
    pub fn search(&self, query: &[f32], limit: usize) -> Vec<(Uuid, f64)> {
        if query.len() != self.dimension {
            return Vec::new();
        }

        let mut results: Vec<(Uuid, f64)> = self
            .vectors
            .iter()
            .map(|(id, vec)| (*id, cosine_similarity(query, vec)))
            .collect();

        // Sort by similarity (descending)
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        results.truncate(limit);
        results
    }

    /// Get the number of stored vectors
    pub fn len(&self) -> usize {
        self.vectors.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.vectors.is_empty()
    }

    /// Get all episode IDs
    pub fn episode_ids(&self) -> Vec<Uuid> {
        self.vectors.keys().copied().collect()
    }

    /// Persist to disk
    fn persist(&self) -> MemoryResult<()> {
        if let Some(ref path) = self.path {
            let data = VectorStoreData {
                dimension: self.dimension,
                vectors: self
                    .vectors
                    .iter()
                    .map(|(id, vec)| (id.to_string(), vec.clone()))
                    .collect(),
            };

            let json = serde_json::to_string_pretty(&data)?;

            // Ensure parent directory exists
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }

            fs::write(path, json)?;
        }
        Ok(())
    }
}

/// Calculate cosine similarity between two vectors
fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    (dot_product / (norm_a * norm_b)) as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_store_and_get() {
        let mut store = VectorStore::new(3);
        let id = Uuid::new_v4();
        let vec = vec![1.0, 2.0, 3.0];

        store.store(id, vec.clone()).unwrap();

        let retrieved = store.get(id).unwrap();
        assert_eq!(retrieved, &vec);
    }

    #[test]
    fn test_search() {
        let mut store = VectorStore::new(3);

        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        let id3 = Uuid::new_v4();

        store.store(id1, vec![1.0, 0.0, 0.0]).unwrap();
        store.store(id2, vec![0.9, 0.1, 0.0]).unwrap();
        store.store(id3, vec![0.0, 1.0, 0.0]).unwrap();

        let query = vec![1.0, 0.0, 0.0];
        let results = store.search(&query, 2);

        assert_eq!(results.len(), 2);
        // First result should be exact match
        assert_eq!(results[0].0, id1);
        assert!((results[0].1 - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_persistence() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("vectors.json");

        let id = Uuid::new_v4();
        let vec = vec![1.0, 2.0, 3.0];

        // Store
        {
            let mut store = VectorStore::open(&path, 3).unwrap();
            store.store(id, vec.clone()).unwrap();
        }

        // Load
        {
            let store = VectorStore::open(&path, 3).unwrap();
            let retrieved = store.get(id).unwrap();
            assert_eq!(retrieved, &vec);
        }
    }

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.001);

        let c = vec![0.0, 1.0, 0.0];
        assert!(cosine_similarity(&a, &c).abs() < 0.001);

        let d = vec![-1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &d) + 1.0).abs() < 0.001);
    }

    #[test]
    fn test_dimension_mismatch() {
        let mut store = VectorStore::new(3);
        let id = Uuid::new_v4();

        let result = store.store(id, vec![1.0, 2.0]); // Wrong dimension
        assert!(result.is_err());
    }
}
