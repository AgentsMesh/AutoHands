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
