//! Bellman propagation for utility spreading
//!
//! Implements temporal difference-style learning to spread utility
//! from helpful episodes to similar ones, improving future retrieval.

use crate::storage::VectorStore;
use crate::types::Episode;
use std::collections::HashMap;
use uuid::Uuid;

/// Result of a propagation run
#[derive(Debug, Clone)]
pub struct PropagationResult {
    /// Number of episodes updated
    pub episodes_updated: usize,
    /// Total utility change
    pub total_change: f64,
    /// Maximum individual change
    pub max_change: f64,
}

/// Run Bellman propagation to spread utility through the memory
///
/// Uses TD-style update: Q(s) = Q(s) + α * (γ * Q(s') * sim - Q(s))
///
/// # Arguments
/// * `episodes` - Episodes with their current utilities
/// * `vectors` - Vector store for similarity lookup
/// * `learning_rate` - Alpha: how quickly to update (0.0 to 1.0)
/// * `discount` - Gamma: how much to trust similar episodes (0.0 to 1.0)
/// * `similarity_threshold` - Minimum similarity to consider
///
/// # Returns
/// Map of episode IDs to new utilities and propagation statistics
pub fn bellman_propagate(
    episodes: &[Episode],
    vectors: &VectorStore,
    learning_rate: f64,
    discount: f64,
    similarity_threshold: f64,
) -> (HashMap<Uuid, f64>, PropagationResult) {
    let mut new_utilities: HashMap<Uuid, f64> = HashMap::new();
    let mut total_change: f64 = 0.0;
    let mut max_change: f64 = 0.0;
    let mut updates = 0;

    // Build lookup for quick access
    let utility_map: HashMap<Uuid, f64> = episodes.iter().map(|e| (e.id, e.utility)).collect();

    for episode in episodes {
        // Get this episode's embedding
        let Some(embedding) = vectors.get(episode.id) else {
            new_utilities.insert(episode.id, episode.utility);
            continue;
        };

        // Find similar episodes
        let similar = vectors.search(embedding, 10);

        // Calculate utility update from similar episodes
        let mut weighted_utility = 0.0;
        let mut weight_sum = 0.0;

        for (similar_id, similarity) in similar {
            // Skip self
            if similar_id == episode.id {
                continue;
            }

            // Skip low similarity
            if similarity < similarity_threshold {
                continue;
            }

            // Get the similar episode's utility
            if let Some(&other_utility) = utility_map.get(&similar_id) {
                weighted_utility += similarity * other_utility;
                weight_sum += similarity;
            }
        }

        // Calculate new utility
        let new_utility = if weight_sum > 0.0 {
            let neighbor_contribution = weighted_utility / weight_sum;

            // TD update: Q(s) = Q(s) + α * (γ * Q(s') - Q(s))
            let target = discount * neighbor_contribution;
            let update = learning_rate * (target - episode.utility);

            // Clamp to valid range
            (episode.utility + update).clamp(0.0, 1.0)
        } else {
            episode.utility
        };

        let change = (new_utility - episode.utility).abs();
        if change > 0.001 {
            updates += 1;
            total_change += change;
            max_change = max_change.max(change);
        }

        new_utilities.insert(episode.id, new_utility);
    }

    let result = PropagationResult {
        episodes_updated: updates,
        total_change,
        max_change,
    };

    (new_utilities, result)
}

/// Run temporal credit assignment
///
/// Credits episodes that preceded successful outcomes, creating
/// a temporal chain of utility.
///
/// # Arguments
/// * `episodes` - Episodes sorted by time (oldest first)
/// * `temporal_discount` - How much to credit earlier episodes (0.0 to 1.0)
///
/// # Returns
/// Map of episode IDs to temporal credits
pub fn temporal_credit_assignment(
    episodes: &[Episode],
    temporal_discount: f64,
) -> HashMap<Uuid, f64> {
    let mut credits: HashMap<Uuid, f64> = HashMap::new();

    // Initialize with current utilities
    for episode in episodes {
        credits.insert(episode.id, episode.utility);
    }

    // Work backwards through time
    let mut prev_utility = 0.0;

    for episode in episodes.iter().rev() {
        let current_utility = episode.utility;

        // Add discounted future utility
        let temporal_bonus = temporal_discount * prev_utility;
        let new_utility = (current_utility + temporal_bonus).min(1.0);

        credits.insert(episode.id, new_utility);
        prev_utility = new_utility;
    }

    credits
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::EpisodeOutcome;

    fn make_episode(id: Uuid, utility: f64) -> Episode {
        let mut ep = Episode::new(
            format!("Episode {}", id),
            "test".to_string(),
            EpisodeOutcome::Success,
        );
        ep.id = id;
        ep.utility = utility;
        ep
    }

    #[test]
    fn test_propagation_basic() {
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();

        let ep1 = make_episode(id1, 1.0); // High utility
        let ep2 = make_episode(id2, 0.2); // Low utility

        let episodes = vec![ep1, ep2];

        // Create vector store with similar embeddings
        let mut vectors = VectorStore::new(3);
        vectors.store(id1, vec![1.0, 0.0, 0.0]).unwrap();
        vectors.store(id2, vec![0.9, 0.1, 0.0]).unwrap(); // Similar to id1

        let (utilities, result) = bellman_propagate(&episodes, &vectors, 0.5, 0.9, 0.5);

        // Low utility episode should increase due to similarity to high utility
        assert!(utilities[&id2] > 0.2);
        assert!(result.episodes_updated > 0);
    }

    #[test]
    fn test_no_propagation_for_dissimilar() {
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();

        let ep1 = make_episode(id1, 1.0);
        let ep2 = make_episode(id2, 0.2);

        let episodes = vec![ep1, ep2];

        // Create vector store with dissimilar embeddings
        let mut vectors = VectorStore::new(3);
        vectors.store(id1, vec![1.0, 0.0, 0.0]).unwrap();
        vectors.store(id2, vec![0.0, 1.0, 0.0]).unwrap(); // Orthogonal

        let (utilities, _) = bellman_propagate(&episodes, &vectors, 0.5, 0.9, 0.5);

        // Should be unchanged or minimal change due to low similarity
        assert!((utilities[&id2] - 0.2).abs() < 0.1);
    }

    #[test]
    fn test_temporal_credit() {
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        let id3 = Uuid::new_v4();

        let ep1 = make_episode(id1, 0.3); // Earlier, low utility
        let ep2 = make_episode(id2, 0.5); // Middle
        let ep3 = make_episode(id3, 1.0); // Latest, high utility

        let episodes = vec![ep1, ep2, ep3]; // Sorted by time

        let credits = temporal_credit_assignment(&episodes, 0.5);

        // Earlier episodes should get credit from later successes
        assert!(credits[&id1] > 0.3);
        assert!(credits[&id2] > 0.5);
    }
}
