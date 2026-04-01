// Copyright 2024-2026 Andrey Vasilevsky <anvanster@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Wilson score calculation for Bayesian confidence
//!
//! Wilson score provides a lower bound confidence interval for a Bernoulli
//! parameter. It's particularly useful for ranking with few observations,
//! as it handles uncertainty better than simple success rate.

/// Calculate Wilson score lower bound
///
/// # Arguments
/// * `positive` - Number of positive feedback events
/// * `total` - Total number of feedback events
/// * `confidence` - Confidence level (e.g., 0.95 for 95% confidence)
///
/// # Returns
/// Lower bound of the Wilson score interval (0.0 to 1.0)
pub fn wilson_score(positive: u32, total: u32, confidence: f64) -> f64 {
    if total == 0 {
        return 0.5; // Return neutral score with no data
    }

    let n = total as f64;
    let p = positive as f64 / n;

    // Z-score for confidence level (using normal approximation)
    // For common confidence levels:
    // 0.90 -> 1.645
    // 0.95 -> 1.96
    // 0.99 -> 2.576
    let z = z_score(confidence);
    let z2 = z * z;

    // Wilson score formula
    let denominator = 1.0 + z2 / n;
    let center = p + z2 / (2.0 * n);
    let spread = z * ((p * (1.0 - p) + z2 / (4.0 * n)) / n).sqrt();

    let lower_bound = (center - spread) / denominator;

    // Clamp to valid range
    lower_bound.clamp(0.0, 1.0)
}

/// Calculate Wilson score with default 95% confidence
pub fn wilson_score_default(positive: u32, total: u32) -> f64 {
    wilson_score(positive, total, 0.95)
}

/// Get z-score for a given confidence level
fn z_score(confidence: f64) -> f64 {
    // Using common z-scores
    // For more precision, could use a proper inverse normal CDF
    match confidence {
        c if c >= 0.99 => 2.576,
        c if c >= 0.975 => 2.241,
        c if c >= 0.95 => 1.96,
        c if c >= 0.90 => 1.645,
        c if c >= 0.85 => 1.44,
        c if c >= 0.80 => 1.282,
        _ => 1.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_data() {
        // With no data, should return neutral 0.5
        assert_eq!(wilson_score(0, 0, 0.95), 0.5);
    }

    #[test]
    fn test_perfect_score() {
        // All positive should still be less than 1.0 due to uncertainty
        let score = wilson_score(10, 10, 0.95);
        assert!(score < 1.0);
        // Wilson lower bound for 10/10 at 95% confidence is ~0.72
        assert!(score > 0.7);
    }

    #[test]
    fn test_all_negative() {
        // All negative has lower bound at or near 0
        let score = wilson_score(0, 10, 0.95);
        // Lower bound can be 0 for all-negative case
        assert!(score >= 0.0);
        assert!(score < 0.1);
    }

    #[test]
    fn test_low_sample_uncertainty() {
        // With few samples, score should be lower due to uncertainty
        let low_sample = wilson_score(1, 1, 0.95); // 1/1 = 100%
        let high_sample = wilson_score(100, 100, 0.95); // 100/100 = 100%

        // Higher samples = more confidence = higher Wilson score
        assert!(high_sample > low_sample);
    }

    #[test]
    fn test_same_ratio_different_samples() {
        // Same success rate, different sample sizes
        let small = wilson_score(5, 10, 0.95); // 50% with 10 samples
        let large = wilson_score(50, 100, 0.95); // 50% with 100 samples

        // Larger sample = narrower confidence interval = higher lower bound
        assert!(large > small);
    }

    #[test]
    fn test_default_confidence() {
        let with_95 = wilson_score(5, 10, 0.95);
        let default = wilson_score_default(5, 10);

        assert!((with_95 - default).abs() < 0.001);
    }

    #[test]
    fn test_higher_confidence_lower_bound() {
        // Higher confidence = wider interval = lower lower bound
        let conf_90 = wilson_score(5, 10, 0.90);
        let conf_95 = wilson_score(5, 10, 0.95);
        let conf_99 = wilson_score(5, 10, 0.99);

        assert!(conf_90 > conf_95);
        assert!(conf_95 > conf_99);
    }
}
