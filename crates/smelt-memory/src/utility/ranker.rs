//! Utility-based ranking for episode retrieval

use super::decay::{apply_decay, DecayParams};
use super::wilson::wilson_score_default;
use crate::types::{Episode, RankedEpisode};
use chrono::Utc;

/// Configuration for utility-based ranking
#[derive(Debug, Clone)]
pub struct RankerConfig {
    /// Weight for semantic similarity (0.0 to 1.0)
    pub similarity_weight: f64,
    /// Weight for utility score (0.0 to 1.0)
    pub utility_weight: f64,
    /// Weight for feedback score (0.0 to 1.0)
    pub feedback_weight: f64,
    /// Decay parameters
    pub decay_params: DecayParams,
}

impl Default for RankerConfig {
    fn default() -> Self {
        Self {
            similarity_weight: 0.5,
            utility_weight: 0.3,
            feedback_weight: 0.2,
            decay_params: DecayParams::default(),
        }
    }
}

/// Utility-based ranker for episodes
pub struct UtilityRanker {
    config: RankerConfig,
}

impl UtilityRanker {
    /// Create a new ranker with default configuration
    pub fn new() -> Self {
        Self {
            config: RankerConfig::default(),
        }
    }

    /// Create a ranker with custom configuration
    pub fn with_config(config: RankerConfig) -> Self {
        Self { config }
    }

    /// Rank episodes based on similarity and utility
    ///
    /// # Arguments
    /// * `episodes` - Episodes to rank
    /// * `similarities` - Semantic similarity scores (0.0 to 1.0)
    ///
    /// # Returns
    /// Ranked episodes sorted by combined score (descending)
    pub fn rank(&self, episodes: Vec<Episode>, similarities: Vec<f64>) -> Vec<RankedEpisode> {
        let now = Utc::now();

        let mut ranked: Vec<RankedEpisode> = episodes
            .into_iter()
            .zip(similarities)
            .map(|(episode, similarity)| {
                let score = self.compute_score(&episode, similarity, now);
                RankedEpisode {
                    episode,
                    similarity,
                    score,
                }
            })
            .collect();

        // Sort by score descending
        ranked.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        ranked
    }

    /// Compute combined score for an episode
    fn compute_score(&self, episode: &Episode, similarity: f64, now: chrono::DateTime<Utc>) -> f64 {
        // Apply decay to utility
        let decayed_utility = apply_decay(
            episode.utility,
            episode.created_at,
            now,
            &self.config.decay_params,
        );

        // Calculate feedback score using Wilson
        let feedback_score = wilson_score_default(episode.helpful_count, episode.feedback_count);

        // Combine scores
        self.config.similarity_weight * similarity
            + self.config.utility_weight * decayed_utility
            + self.config.feedback_weight * feedback_score
    }

    /// Update utility based on feedback
    ///
    /// Simple update rule: new_utility = old_utility + α * (feedback_value - old_utility)
    pub fn update_utility_from_feedback(
        &self,
        episode: &Episode,
        helpful: bool,
        learning_rate: f64,
    ) -> f64 {
        let feedback_value = if helpful { 1.0 } else { 0.0 };
        let new_utility = episode.utility + learning_rate * (feedback_value - episode.utility);
        new_utility.clamp(0.0, 1.0)
    }
}

impl Default for UtilityRanker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::EpisodeOutcome;

    fn make_episode(summary: &str, utility: f64, helpful: u32, total: u32) -> Episode {
        let mut ep = Episode::new(
            summary.to_string(),
            "test".to_string(),
            EpisodeOutcome::Success,
        );
        ep.utility = utility;
        ep.helpful_count = helpful;
        ep.feedback_count = total;
        ep
    }

    #[test]
    fn test_ranking_by_similarity() {
        let ranker = UtilityRanker::new();

        let ep1 = make_episode("Episode 1", 0.5, 0, 0);
        let ep2 = make_episode("Episode 2", 0.5, 0, 0);

        let episodes = vec![ep1, ep2];
        let similarities = vec![0.9, 0.5]; // First is more similar

        let ranked = ranker.rank(episodes, similarities);

        assert_eq!(ranked[0].episode.summary, "Episode 1");
        assert!(ranked[0].score > ranked[1].score);
    }

    #[test]
    fn test_ranking_considers_utility() {
        let ranker = UtilityRanker::new();

        let ep1 = make_episode("Low utility", 0.1, 0, 0);
        let ep2 = make_episode("High utility", 0.9, 0, 0);

        let episodes = vec![ep1, ep2];
        let similarities = vec![0.8, 0.8]; // Same similarity

        let ranked = ranker.rank(episodes, similarities);

        // Higher utility should rank first
        assert_eq!(ranked[0].episode.summary, "High utility");
    }

    #[test]
    fn test_ranking_considers_feedback() {
        let ranker = UtilityRanker::new();

        let ep1 = make_episode("Poor feedback", 0.5, 1, 10); // 10% helpful
        let ep2 = make_episode("Good feedback", 0.5, 9, 10); // 90% helpful

        let episodes = vec![ep1, ep2];
        let similarities = vec![0.8, 0.8]; // Same similarity

        let ranked = ranker.rank(episodes, similarities);

        // Better feedback should rank first
        assert_eq!(ranked[0].episode.summary, "Good feedback");
    }

    #[test]
    fn test_utility_update() {
        let ranker = UtilityRanker::new();

        let episode = make_episode("Test", 0.5, 0, 0);

        // Positive feedback should increase utility
        let new_utility = ranker.update_utility_from_feedback(&episode, true, 0.1);
        assert!(new_utility > 0.5);

        // Negative feedback should decrease utility
        let new_utility = ranker.update_utility_from_feedback(&episode, false, 0.1);
        assert!(new_utility < 0.5);
    }

    #[test]
    fn test_utility_bounds() {
        let ranker = UtilityRanker::new();

        let high = make_episode("High", 0.99, 0, 0);
        let low = make_episode("Low", 0.01, 0, 0);

        // Should not exceed 1.0
        let new_high = ranker.update_utility_from_feedback(&high, true, 0.5);
        assert!(new_high <= 1.0);

        // Should not go below 0.0
        let new_low = ranker.update_utility_from_feedback(&low, false, 0.5);
        assert!(new_low >= 0.0);
    }
}
