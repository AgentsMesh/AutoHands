use super::*;
use autohands_memory_vector::SimpleHashEmbedding;

#[test]
fn test_extension_manifest() {
    let ext = HybridMemoryExtension::new();
    assert_eq!(ext.manifest().id, "memory-hybrid");
}

#[test]
fn test_config_with_embedder() {
    let embedder = Arc::new(SimpleHashEmbedding::default());
    let config = HybridMemoryExtensionConfig::with_embedder(embedder)
        .id("test-hybrid")
        .favor_semantic();

    assert_eq!(config.id, "test-hybrid");
    assert!((config.config.fusion.alpha - 0.7).abs() < 0.01);
}

#[test]
fn test_config_favor_keyword() {
    let embedder = Arc::new(SimpleHashEmbedding::default());
    let config = HybridMemoryExtensionConfig::with_embedder(embedder).favor_keyword();

    assert!((config.config.fusion.alpha - 0.3).abs() < 0.01);
}

#[test]
fn test_config_min_relevance() {
    let embedder = Arc::new(SimpleHashEmbedding::default());
    let config = HybridMemoryExtensionConfig::with_embedder(embedder).min_relevance(0.5);

    assert!((config.config.min_relevance - 0.5).abs() < 0.01);
}

#[test]
fn test_extension_default() {
    let ext = HybridMemoryExtension::default();
    assert_eq!(ext.manifest().id, "memory-hybrid");
}

#[test]
fn test_backend_initially_none() {
    let ext = HybridMemoryExtension::new();
    assert!(ext.backend().is_none());
}

#[test]
fn test_as_any() {
    let ext = HybridMemoryExtension::new();
    let any_ref = ext.as_any();
    assert!(any_ref.downcast_ref::<HybridMemoryExtension>().is_some());
}
