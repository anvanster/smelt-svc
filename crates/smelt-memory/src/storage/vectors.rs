// Copyright 2024-2026 Andrey Vasilevsky <anvanster@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Vector storage for episode embeddings using vectrust

use crate::error::{MemoryError, MemoryResult};
use std::path::{Path, PathBuf};
use uuid::Uuid;
use vectrust::{CreateIndexConfig, DistanceMetric, LocalIndex, VectorItem};

/// Vector store backed by vectrust (RocksDB + HNSW)
pub struct VectorStore {
    /// Index directory path (None for in-memory/temp)
    index_path: PathBuf,
    /// Embedding dimension
    dimension: usize,
    /// Whether this is a temporary store (for in-memory mode)
    _temp_dir: Option<tempfile::TempDir>,
    /// Owned runtime for sync contexts (kept alive to prevent handle invalidation)
    _runtime: Option<tokio::runtime::Runtime>,
}

impl VectorStore {
    /// Create a fallback runtime if no tokio runtime is active
    fn ensure_runtime() -> Option<tokio::runtime::Runtime> {
        if tokio::runtime::Handle::try_current().is_err() {
            Some(tokio::runtime::Runtime::new().expect("Failed to create tokio runtime"))
        } else {
            None
        }
    }

    /// Create a new in-memory vector store (uses a temp directory)
    pub fn new(dimension: usize) -> Self {
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temp directory");
        let index_path = temp_dir.path().to_path_buf();
        Self {
            index_path,
            dimension,
            _temp_dir: Some(temp_dir),
            _runtime: Self::ensure_runtime(),
        }
    }

    /// Open or create a persistent vector store
    pub fn open(path: &Path, dimension: usize) -> MemoryResult<Self> {
        // Use the parent directory of the old vectors.json path as the index directory
        // e.g. .smelt/memory/vectors.json -> .smelt/memory/vectors.vectrust/
        let index_path = path.with_extension("vectrust");

        if let Some(parent) = index_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let runtime = Self::ensure_runtime();

        // Migrate from old JSON format if it exists and index doesn't
        if path.exists() && !index_path.exists() {
            let store = Self {
                index_path: index_path.clone(),
                dimension,
                _temp_dir: None,
                _runtime: None, // Temporary, uses parent's runtime context
            };
            if let Err(e) = store.migrate_from_json(path) {
                tracing::warn!("Failed to migrate vectors.json to vectrust: {}", e);
            }
        }

        Ok(Self {
            index_path,
            dimension,
            _temp_dir: None,
            _runtime: runtime,
        })
    }

    /// Open the vectrust index on demand (releases RocksDB lock when dropped)
    fn open_index(&self) -> MemoryResult<LocalIndex> {
        let index = LocalIndex::new(&self.index_path, Some("episodes".into()))
            .map_err(|e| MemoryError::Storage(format!("Failed to open vector index: {}", e)))?;

        // Ensure index is created
        if !self.block_on(index.is_index_created()) {
            self.block_on(index.create_index(Some(CreateIndexConfig {
                distance_metric: DistanceMetric::Cosine,
                ..Default::default()
            })))
            .map_err(|e| MemoryError::Storage(format!("Failed to create index: {}", e)))?;
        }

        Ok(index)
    }

    /// Get a tokio runtime handle (reuse stored or current runtime)
    fn runtime(&self) -> tokio::runtime::Handle {
        if let Some(ref rt) = self._runtime {
            rt.handle().clone()
        } else {
            tokio::runtime::Handle::current()
        }
    }

    /// Run an async future synchronously, handling both sync and async caller contexts.
    /// When called from within a tokio runtime (e.g. async tests), runs on a scoped
    /// thread to avoid "Cannot start a runtime from within a runtime" panic.
    fn block_on<F>(&self, f: F) -> F::Output
    where
        F: std::future::Future + Send,
        F::Output: Send,
    {
        let handle = self.runtime();
        if self._runtime.is_some() {
            // We own the runtime — safe to block_on directly
            handle.block_on(f)
        } else {
            // We're inside an existing runtime — run on a scoped thread
            std::thread::scope(|s| s.spawn(|| handle.block_on(f)).join().unwrap())
        }
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

        let index = self.open_index()?;

        let item = VectorItem {
            id: episode_id,
            vector,
            metadata: serde_json::json!({}),
            ..Default::default()
        };

        self.block_on(async {
            index
                .begin_update()
                .await
                .map_err(|e| MemoryError::Storage(format!("Failed to begin update: {}", e)))?;
            // Delete first in case it exists (upsert pattern)
            let _ = index.delete_item(&episode_id).await;
            index
                .insert_item(item)
                .await
                .map_err(|e| MemoryError::Storage(format!("Failed to insert vector: {}", e)))?;
            index
                .end_update()
                .await
                .map_err(|e| MemoryError::Storage(format!("Failed to end update: {}", e)))?;
            Ok::<_, MemoryError>(())
        })?;

        Ok(())
    }

    /// Get a vector by episode ID
    pub fn get(&self, episode_id: Uuid) -> Option<Vec<f32>> {
        let index = self.open_index().ok()?;

        self.block_on(async {
            let item: Option<VectorItem> = index.get_item(&episode_id).await.ok()?;
            item.map(|i| i.vector)
        })
    }

    /// Remove a vector
    pub fn remove(&mut self, episode_id: Uuid) -> MemoryResult<()> {
        let index = self.open_index()?;

        self.block_on(async {
            index
                .begin_update()
                .await
                .map_err(|e| MemoryError::Storage(format!("Failed to begin update: {}", e)))?;
            let _ = index.delete_item(&episode_id).await;
            index
                .end_update()
                .await
                .map_err(|e| MemoryError::Storage(format!("Failed to end update: {}", e)))?;
            Ok::<_, MemoryError>(())
        })?;

        Ok(())
    }

    /// Search for similar vectors using cosine similarity
    pub fn search(&self, query: &[f32], limit: usize) -> Vec<(Uuid, f64)> {
        if query.len() != self.dimension {
            return Vec::new();
        }

        let index = match self.open_index() {
            Ok(i) => i,
            Err(_) => return Vec::new(),
        };

        self.block_on(async {
            let results: Vec<vectrust::QueryResult> = index
                .query_items(query.to_vec(), Some(limit as u32), None)
                .await
                .unwrap_or_default();

            results
                .into_iter()
                .map(|r| (r.item.id, r.score as f64))
                .collect()
        })
    }

    /// Get the number of stored vectors
    pub fn len(&self) -> usize {
        let index = match self.open_index() {
            Ok(i) => i,
            Err(_) => return 0,
        };

        self.block_on(async { index.get_stats().await.map(|s| s.items).unwrap_or(0) })
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get all episode IDs
    pub fn episode_ids(&self) -> Vec<Uuid> {
        let index = match self.open_index() {
            Ok(i) => i,
            Err(_) => return Vec::new(),
        };

        self.block_on(async {
            let items: Vec<VectorItem> = index.list_items(None).await.unwrap_or_default();
            items.into_iter().map(|i| i.id).collect()
        })
    }

    /// Migrate data from old JSON format to vectrust
    fn migrate_from_json(&self, json_path: &Path) -> MemoryResult<()> {
        use serde::Deserialize;

        #[derive(Deserialize)]
        struct VectorStoreData {
            #[allow(dead_code)]
            dimension: usize,
            vectors: Vec<(String, Vec<f32>)>,
        }

        let data = std::fs::read_to_string(json_path)?;
        let stored: VectorStoreData = serde_json::from_str(&data)?;

        if stored.vectors.is_empty() {
            return Ok(());
        }

        let count = stored.vectors.len();
        let index = self.open_index()?;

        self.block_on(async {
            index
                .begin_update()
                .await
                .map_err(|e| MemoryError::Storage(format!("Failed to begin update: {}", e)))?;

            for (id_str, vector) in stored.vectors {
                if let Ok(id) = Uuid::parse_str(&id_str) {
                    let item = VectorItem {
                        id,
                        vector,
                        metadata: serde_json::json!({}),
                        ..Default::default()
                    };
                    let _ = index.insert_item(item).await;
                }
            }

            index
                .end_update()
                .await
                .map_err(|e| MemoryError::Storage(format!("Failed to end update: {}", e)))?;

            Ok::<_, MemoryError>(())
        })?;

        tracing::info!(
            "Migrated {} vectors from {} to vectrust",
            count,
            json_path.display()
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_and_get() {
        let mut store = VectorStore::new(3);
        let id = Uuid::new_v4();
        let vec = vec![1.0, 2.0, 3.0];

        store.store(id, vec.clone()).unwrap();

        let retrieved = store.get(id).unwrap();
        assert_eq!(retrieved, vec);
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
        let dir = tempfile::tempdir().unwrap();
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
            assert_eq!(retrieved, vec);
        }
    }

    #[test]
    fn test_dimension_mismatch() {
        let mut store = VectorStore::new(3);
        let id = Uuid::new_v4();

        let result = store.store(id, vec![1.0, 2.0]); // Wrong dimension
        assert!(result.is_err());
    }
}
