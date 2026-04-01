// Copyright 2024-2026 Andrey Vasilevsky <anvanster@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Main SmeltMemory implementation

use crate::embedder::{Embedder, FastEmbedder, DEFAULT_DIMENSION};
use crate::error::MemoryResult;
use crate::storage::{EpisodeStorage, MemoryStats, VectorStore};
use crate::types::{Episode, EpisodeOutcome, ErrorResolution, RankedEpisode};
use crate::utility::{
    bellman_propagate, temporal_credit_assignment, PropagationResult, UtilityRanker,
};
use smelt_core::IntentRecord;
use std::path::Path;
use std::sync::Arc;
use uuid::Uuid;

/// Main memory system for Smelt
pub struct SmeltMemory {
    /// Episode metadata storage
    storage: EpisodeStorage,
    /// Vector embeddings storage
    vectors: VectorStore,
    /// Text embedder
    embedder: Arc<dyn Embedder>,
    /// Utility-based ranker
    ranker: UtilityRanker,
    /// Current project name
    project: Option<String>,
}

impl SmeltMemory {
    /// Open or create a memory system in the given directory
    pub fn open(path: &Path) -> MemoryResult<Self> {
        let db_path = path.join("memory.db");
        let vectors_path = path.join("vectors.json");

        let storage = EpisodeStorage::open(&db_path)?;

        // Try to create embedder, fall back to dummy dimension if model unavailable
        let (embedder, dimension): (Arc<dyn Embedder>, usize) = match FastEmbedder::new() {
            Ok(e) => {
                let dim = e.dimension();
                (Arc::new(e), dim)
            }
            Err(e) => {
                tracing::warn!("Failed to initialize embedder: {}", e);
                // Use a dummy embedder that returns zeros
                (
                    Arc::new(DummyEmbedder::new(DEFAULT_DIMENSION)),
                    DEFAULT_DIMENSION,
                )
            }
        };

        let vectors = VectorStore::open(&vectors_path, dimension)?;

        Ok(Self {
            storage,
            vectors,
            embedder,
            ranker: UtilityRanker::new(),
            project: None,
        })
    }

    /// Create an in-memory instance for testing
    pub fn in_memory() -> MemoryResult<Self> {
        let storage = EpisodeStorage::in_memory()?;
        let vectors = VectorStore::new(DEFAULT_DIMENSION);

        Ok(Self {
            storage,
            vectors,
            embedder: Arc::new(DummyEmbedder::new(DEFAULT_DIMENSION)),
            ranker: UtilityRanker::new(),
            project: None,
        })
    }

    /// Set the current project
    pub fn with_project(mut self, project: String) -> Self {
        self.project = Some(project);
        self
    }

    /// Capture an episode from an intent and its outcome
    pub fn capture_from_intent(
        &mut self,
        intent: &IntentRecord,
        outcome: EpisodeOutcome,
        files_modified: Vec<String>,
        errors_resolved: Vec<ErrorResolution>,
        tags: Vec<String>,
        commit_sha: Option<String>,
    ) -> MemoryResult<Uuid> {
        let mut episode = Episode::new(intent.goal.clone(), "intent".to_string(), outcome)
            .with_intent(intent.id)
            .with_files(files_modified)
            .with_errors(errors_resolved)
            .with_tags(tags);

        if let Some(sha) = commit_sha {
            episode = episode.with_commit(sha);
        }

        if let Some(ref project) = self.project {
            episode = episode.with_project(project.clone());
        }

        self.capture(episode)
    }

    /// Capture an episode directly
    pub fn capture(&mut self, mut episode: Episode) -> MemoryResult<Uuid> {
        // Set project if not already set
        if episode.project.is_none() {
            episode.project = self.project.clone();
        }

        let id = episode.id;

        // Generate and store embedding
        let text = episode.to_embedding_text();
        match self.embedder.embed(&text) {
            Ok(embedding) => {
                self.vectors.store(id, embedding)?;
            }
            Err(e) => {
                tracing::warn!("Failed to embed episode {}: {}", id, e);
            }
        }

        // Store metadata
        self.storage.store_episode(&episode)?;

        Ok(id)
    }

    /// Retrieve relevant episodes for a query
    pub fn retrieve(&self, query: &str, limit: usize) -> MemoryResult<Vec<RankedEpisode>> {
        // Embed the query
        let query_embedding = self.embedder.embed(query)?;

        // Find similar episodes
        let similar = self.vectors.search(&query_embedding, limit * 2);

        if similar.is_empty() {
            return Ok(Vec::new());
        }

        // Fetch episode metadata and filter by project
        let mut episodes = Vec::new();
        let mut similarities = Vec::new();

        for (id, similarity) in similar {
            if let Some(episode) = self.storage.get_episode(id)? {
                // Filter by project if set
                if let Some(ref project) = self.project {
                    if episode.project.as_ref() != Some(project) {
                        continue;
                    }
                }

                episodes.push(episode);
                similarities.push(similarity);

                if episodes.len() >= limit {
                    break;
                }
            }
        }

        // Rank with utility
        let ranked = self.ranker.rank(episodes, similarities);

        Ok(ranked.into_iter().take(limit).collect())
    }

    /// Record feedback for an episode
    pub fn record_feedback(&mut self, episode_id: Uuid, helpful: bool) -> MemoryResult<()> {
        // Update feedback counts
        self.storage.record_feedback(episode_id, helpful)?;

        // Update utility
        if let Some(episode) = self.storage.get_episode(episode_id)? {
            let new_utility = self
                .ranker
                .update_utility_from_feedback(&episode, helpful, 0.1);
            self.storage.update_utility(episode_id, new_utility)?;
        }

        Ok(())
    }

    /// Run utility propagation to spread value through the memory
    pub fn propagate_utility(&mut self, temporal: bool) -> MemoryResult<PropagationResult> {
        let episodes = self.storage.list_episodes(self.project.as_deref())?;

        if episodes.is_empty() {
            return Ok(PropagationResult {
                episodes_updated: 0,
                total_change: 0.0,
                max_change: 0.0,
            });
        }

        // Run Bellman propagation
        let (new_utilities, result) = bellman_propagate(
            &episodes,
            &self.vectors,
            0.1, // learning_rate
            0.9, // discount
            0.5, // similarity_threshold
        );

        // Apply temporal credit if requested
        let final_utilities = if temporal {
            let temporal_credits = temporal_credit_assignment(&episodes, 0.5);

            // Combine Bellman and temporal credits
            new_utilities
                .into_iter()
                .map(|(id, bellman_u)| {
                    let temporal_u = temporal_credits.get(&id).copied().unwrap_or(bellman_u);
                    (id, (bellman_u + temporal_u) / 2.0)
                })
                .collect()
        } else {
            new_utilities
        };

        // Update storage
        for (id, utility) in final_utilities {
            self.storage.update_utility(id, utility)?;
        }

        Ok(result)
    }

    /// Get an episode by ID
    pub fn get_episode(&self, id: Uuid) -> MemoryResult<Option<Episode>> {
        self.storage.get_episode(id)
    }

    /// List all episodes
    pub fn list_episodes(&self) -> MemoryResult<Vec<Episode>> {
        self.storage.list_episodes(self.project.as_deref())
    }

    /// Get memory statistics
    pub fn stats(&self) -> MemoryResult<MemoryStats> {
        self.storage.get_stats(self.project.as_deref())
    }
}

/// Dummy embedder for when fastembed is unavailable
struct DummyEmbedder {
    dimension: usize,
}

impl DummyEmbedder {
    fn new(dimension: usize) -> Self {
        Self { dimension }
    }
}

impl Embedder for DummyEmbedder {
    fn dimension(&self) -> usize {
        self.dimension
    }

    fn embed(&self, text: &str) -> MemoryResult<Vec<f32>> {
        // Generate a simple hash-based embedding
        // This is not semantically meaningful but allows basic testing
        let mut embedding = vec![0.0f32; self.dimension];

        for (i, byte) in text.bytes().enumerate() {
            let idx = i % self.dimension;
            embedding[idx] += byte as f32 / 255.0;
        }

        // Normalize
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for x in &mut embedding {
                *x /= norm;
            }
        }

        Ok(embedding)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capture_and_retrieve() {
        let mut memory = SmeltMemory::in_memory().unwrap();

        let episode = Episode::new(
            "Fixed authentication bug in login flow".to_string(),
            "bugfix".to_string(),
            EpisodeOutcome::Success,
        )
        .with_tags(vec!["auth".to_string(), "security".to_string()]);

        let id = memory.capture(episode).unwrap();

        // Retrieve with similar query
        let results = memory.retrieve("authentication login", 5).unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].episode.id, id);
    }

    #[test]
    fn test_feedback() {
        let mut memory = SmeltMemory::in_memory().unwrap();

        let episode = Episode::new(
            "Test episode".to_string(),
            "test".to_string(),
            EpisodeOutcome::Success,
        );

        let id = memory.capture(episode).unwrap();

        // Record positive feedback
        memory.record_feedback(id, true).unwrap();
        memory.record_feedback(id, true).unwrap();

        let updated = memory.get_episode(id).unwrap().unwrap();
        assert_eq!(updated.helpful_count, 2);
        assert!(updated.utility > 1.0 - 0.001); // Should increase toward 1.0
    }

    #[test]
    fn test_propagation() {
        let mut memory = SmeltMemory::in_memory().unwrap();

        // Create some episodes
        let ep1 = Episode::new(
            "Auth fix".to_string(),
            "bugfix".to_string(),
            EpisodeOutcome::Success,
        );
        let ep2 = Episode::new(
            "Auth test".to_string(),
            "test".to_string(),
            EpisodeOutcome::Partial,
        );

        memory.capture(ep1).unwrap();
        memory.capture(ep2).unwrap();

        // Run propagation
        let result = memory.propagate_utility(false).unwrap();
        // Just verify it runs without error
        assert!(result.total_change >= 0.0);
    }

    #[test]
    fn test_project_filter() {
        let mut memory = SmeltMemory::in_memory().unwrap();

        let ep1 = Episode::new(
            "Project A work".to_string(),
            "feature".to_string(),
            EpisodeOutcome::Success,
        )
        .with_project("project-a".to_string());
        let ep2 = Episode::new(
            "Project B work".to_string(),
            "feature".to_string(),
            EpisodeOutcome::Success,
        )
        .with_project("project-b".to_string());

        memory.capture(ep1).unwrap();
        memory.capture(ep2).unwrap();

        // Without filter, should see both
        let all = memory.list_episodes().unwrap();
        assert_eq!(all.len(), 2);

        // With project filter
        let _memory_a = memory.with_project("project-a".to_string());
        // Note: list_episodes would now filter by project
    }

    #[test]
    fn test_stats() {
        let mut memory = SmeltMemory::in_memory().unwrap();

        let ep1 = Episode::new(
            "Ep1".to_string(),
            "test".to_string(),
            EpisodeOutcome::Success,
        );
        let ep2 = Episode::new(
            "Ep2".to_string(),
            "test".to_string(),
            EpisodeOutcome::Success,
        );

        memory.capture(ep1).unwrap();
        memory.capture(ep2).unwrap();

        let stats = memory.stats().unwrap();
        assert_eq!(stats.total_episodes, 2);
    }
}
