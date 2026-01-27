//! Exponential decay for time-based utility degradation
//!
//! Implements exponential decay to reduce the utility of older episodes,
//! ensuring more recent experiences are prioritized in retrieval.

use chrono::{DateTime, Utc};

/// Parameters for exponential decay
#[derive(Debug, Clone)]
pub struct DecayParams {
    /// Daily decay rate (0.0 to 1.0)
    /// A rate of 0.01 gives approximately 69-day half-life
    pub rate: f64,

    /// Minimum utility floor (prevents complete decay)
    pub floor: f64,
}

impl Default for DecayParams {
    fn default() -> Self {
        Self {
            rate: 0.01, // ~69 day half-life
            floor: 0.1, // 10% minimum utility
        }
    }
}

impl DecayParams {
    /// Create params with a specific half-life in days
    pub fn with_half_life(days: f64) -> Self {
        // decay_factor = 0.5 after `days` days
        // 0.5 = (1 - rate)^days
        // rate = 1 - 0.5^(1/days)
        let rate = 1.0 - 0.5_f64.powf(1.0 / days);
        Self { rate, floor: 0.1 }
    }

    /// Set the minimum utility floor
    pub fn with_floor(mut self, floor: f64) -> Self {
        self.floor = floor.clamp(0.0, 1.0);
        self
    }
}

/// Apply exponential decay to a utility value
///
/// # Arguments
/// * `utility` - Current utility value
/// * `created_at` - When the episode was created
/// * `now` - Current time
/// * `params` - Decay parameters
///
/// # Returns
/// Decayed utility value (clamped to floor)
pub fn apply_decay(
    utility: f64,
    created_at: DateTime<Utc>,
    now: DateTime<Utc>,
    params: &DecayParams,
) -> f64 {
    let days_elapsed = (now - created_at).num_seconds() as f64 / 86400.0;

    if days_elapsed <= 0.0 {
        return utility;
    }

    // Exponential decay: value * (1 - rate)^days
    let decay_factor = (1.0 - params.rate).powf(days_elapsed);
    let decayed = utility * decay_factor;

    // Apply floor
    decayed.max(params.floor)
}

/// Calculate the decay factor for a given age
#[allow(dead_code)]
pub fn decay_factor(days: f64, params: &DecayParams) -> f64 {
    if days <= 0.0 {
        1.0
    } else {
        (1.0 - params.rate).powf(days).max(params.floor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeDelta as Duration;

    #[test]
    fn test_no_decay_for_new() {
        let params = DecayParams::default();
        let now = Utc::now();
        let created = now;

        let decayed = apply_decay(1.0, created, now, &params);
        assert!((decayed - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_decay_over_time() {
        let params = DecayParams::default();
        let now = Utc::now();
        let one_week_ago = now - Duration::days(7);
        let one_month_ago = now - Duration::days(30);

        let week_decay = apply_decay(1.0, one_week_ago, now, &params);
        let month_decay = apply_decay(1.0, one_month_ago, now, &params);

        // Older should decay more
        assert!(week_decay > month_decay);
        // But both should be less than original
        assert!(week_decay < 1.0);
        assert!(month_decay < 1.0);
    }

    #[test]
    fn test_floor() {
        let params = DecayParams {
            rate: 0.1,
            floor: 0.2,
        };
        let now = Utc::now();
        let long_ago = now - Duration::days(365);

        let decayed = apply_decay(1.0, long_ago, now, &params);
        assert!(decayed >= params.floor);
    }

    #[test]
    fn test_half_life() {
        let half_life_days = 30.0;
        let params = DecayParams::with_half_life(half_life_days).with_floor(0.0);
        let now = Utc::now();
        let half_life_ago = now - Duration::days(half_life_days as i64);

        let decayed = apply_decay(1.0, half_life_ago, now, &params);
        // Should be approximately 0.5 at half-life
        assert!((decayed - 0.5).abs() < 0.05);
    }

    #[test]
    fn test_decay_factor() {
        let params = DecayParams::default();

        let factor_0 = decay_factor(0.0, &params);
        let factor_7 = decay_factor(7.0, &params);
        let factor_30 = decay_factor(30.0, &params);

        assert!((factor_0 - 1.0).abs() < 0.001);
        assert!(factor_7 < 1.0);
        assert!(factor_30 < factor_7);
    }

    #[test]
    fn test_preserves_relative_utility() {
        let params = DecayParams::default();
        let now = Utc::now();
        let week_ago = now - Duration::days(7);

        let high_utility = apply_decay(1.0, week_ago, now, &params);
        let low_utility = apply_decay(0.5, week_ago, now, &params);

        // Relative ordering should be preserved
        assert!(high_utility > low_utility);
    }
}
