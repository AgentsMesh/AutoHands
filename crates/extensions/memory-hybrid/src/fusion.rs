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
mod tests {
    use super::*;

    #[test]
    fn test_rrf_fusion_basic() {
        let vector = vec![
            ("a".to_string(), 0.9),
            ("b".to_string(), 0.8),
            ("c".to_string(), 0.7),
        ];
        let keyword = vec![
            ("b".to_string(), 0.9),
            ("a".to_string(), 0.8),
            ("d".to_string(), 0.7),
        ];

        let config = FusionConfig::default();
        let results = rrf_fusion(&vector, &keyword, &config);

        // Both "a" and "b" appear in both lists, should be ranked higher
        assert!(!results.is_empty());

        // Find positions
        let a_pos = results.iter().position(|(id, _)| id == "a");
        let d_pos = results.iter().position(|(id, _)| id == "d");

        // "a" should rank higher than "d" which only appears in keyword
        assert!(a_pos.unwrap() < d_pos.unwrap());
    }

    #[test]
    fn test_rrf_fusion_empty_lists() {
        let config = FusionConfig::default();

        let results = rrf_fusion(&[], &[], &config);
        assert!(results.is_empty());

        let results = rrf_fusion(&[("a".to_string(), 1.0)], &[], &config);
        assert_eq!(results.len(), 1);

        let results = rrf_fusion(&[], &[("a".to_string(), 1.0)], &config);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_rrf_fusion_weights() {
        let vector = vec![("a".to_string(), 1.0)];
        let keyword = vec![("b".to_string(), 1.0)];

        // Favor vector
        let config = FusionConfig::favor_semantic();
        let results = rrf_fusion(&vector, &keyword, &config);
        let a_score = results.iter().find(|(id, _)| id == "a").unwrap().1;
        let b_score = results.iter().find(|(id, _)| id == "b").unwrap().1;
        assert!(a_score > b_score);

        // Favor keyword
        let config = FusionConfig::favor_keyword();
        let results = rrf_fusion(&vector, &keyword, &config);
        let a_score = results.iter().find(|(id, _)| id == "a").unwrap().1;
        let b_score = results.iter().find(|(id, _)| id == "b").unwrap().1;
        assert!(b_score > a_score);
    }

    #[test]
    fn test_linear_fusion_basic() {
        let vector = vec![("a".to_string(), 1.0), ("b".to_string(), 0.5)];
        let keyword = vec![("b".to_string(), 1.0), ("c".to_string(), 0.5)];

        let results = linear_fusion(&vector, &keyword, 0.5);

        // "b" appears in both, should have highest score
        assert_eq!(results[0].0, "b");
    }

    #[test]
    fn test_linear_fusion_alpha_weight() {
        let vector = vec![("a".to_string(), 1.0)];
        let keyword = vec![("b".to_string(), 1.0)];

        // alpha = 1.0 means only vector
        let results = linear_fusion(&vector, &keyword, 1.0);
        assert_eq!(results[0].0, "a");
        assert!(results[0].1 > results[1].1);

        // alpha = 0.0 means only keyword
        let results = linear_fusion(&vector, &keyword, 0.0);
        assert_eq!(results[0].0, "b");
    }

    #[test]
    fn test_fusion_config_default() {
        let config = FusionConfig::default();
        assert!((config.alpha - 0.5).abs() < 0.01);
        assert!((config.k - 60.0).abs() < 0.01);
    }

    #[test]
    fn test_fusion_config_clone() {
        let config = FusionConfig::favor_semantic();
        let cloned = config.clone();
        assert!((cloned.alpha - 0.7).abs() < 0.01);
    }

    #[test]
    fn test_rrf_produces_sorted_results() {
        let vector = vec![
            ("a".to_string(), 0.9),
            ("b".to_string(), 0.8),
            ("c".to_string(), 0.7),
        ];
        let keyword = vec![
            ("d".to_string(), 0.9),
            ("e".to_string(), 0.8),
            ("f".to_string(), 0.7),
        ];

        let config = FusionConfig::default();
        let results = rrf_fusion(&vector, &keyword, &config);

        // Verify results are sorted by score descending
        for i in 1..results.len() {
            assert!(results[i - 1].1 >= results[i].1);
        }
    }
}
