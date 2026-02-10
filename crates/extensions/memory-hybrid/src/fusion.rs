//! Result fusion algorithms.

use std::collections::HashMap;

/// Configuration for result fusion.
#[derive(Debug, Clone)]
pub struct FusionConfig {
    /// Weight for vector results (0.0 - 1.0).
    /// Keyword results get weight (1.0 - alpha).
    pub alpha: f32,
    /// RRF parameter k (typically 60).
    pub k: f32,
}

impl Default for FusionConfig {
    fn default() -> Self {
        Self {
            alpha: 0.5, // Equal weight
            k: 60.0,
        }
    }
}

impl FusionConfig {
    /// Create config favoring vector (semantic) results.
    pub fn favor_semantic() -> Self {
        Self {
            alpha: 0.7,
            k: 60.0,
        }
    }

    /// Create config favoring keyword results.
    pub fn favor_keyword() -> Self {
        Self {
            alpha: 0.3,
            k: 60.0,
        }
    }
}

/// Reciprocal Rank Fusion (RRF) algorithm.
///
/// Combines results from multiple ranked lists using:
/// score = sum(1 / (k + rank_i))
///
/// This is a well-known rank aggregation method that:
/// - Doesn't require score normalization
/// - Handles missing items gracefully
/// - Produces good results empirically
pub fn rrf_fusion(
    vector_results: &[(String, f32)],
    keyword_results: &[(String, f32)],
    config: &FusionConfig,
) -> Vec<(String, f32)> {
    let mut scores: HashMap<String, f32> = HashMap::new();

    // Calculate RRF scores for vector results
    for (rank, (id, _original_score)) in vector_results.iter().enumerate() {
        let rrf_score = config.alpha / (config.k + rank as f32 + 1.0);
        *scores.entry(id.clone()).or_insert(0.0) += rrf_score;
    }

    // Calculate RRF scores for keyword results
    let keyword_weight = 1.0 - config.alpha;
    for (rank, (id, _original_score)) in keyword_results.iter().enumerate() {
        let rrf_score = keyword_weight / (config.k + rank as f32 + 1.0);
        *scores.entry(id.clone()).or_insert(0.0) += rrf_score;
    }

    // Sort by combined score
    let mut results: Vec<(String, f32)> = scores.into_iter().collect();
    results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    results
}

/// Simple linear combination of scores.
///
/// Requires normalized scores (0-1 range).
pub fn linear_fusion(
    vector_results: &[(String, f32)],
    keyword_results: &[(String, f32)],
    alpha: f32,
) -> Vec<(String, f32)> {
    let mut scores: HashMap<String, f32> = HashMap::new();

    // Add vector scores
    for (id, score) in vector_results {
        *scores.entry(id.clone()).or_insert(0.0) += alpha * score;
    }

    // Add keyword scores
    for (id, score) in keyword_results {
        *scores.entry(id.clone()).or_insert(0.0) += (1.0 - alpha) * score;
    }

    let mut results: Vec<(String, f32)> = scores.into_iter().collect();
    results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    results
}

#[cfg(test)]
#[path = "fusion_tests.rs"]
mod tests;
