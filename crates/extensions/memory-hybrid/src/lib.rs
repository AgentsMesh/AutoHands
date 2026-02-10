//! Hybrid memory backend for AutoHands.
//!
//! Combines vector semantic search with keyword-based FTS5 full-text search
//! using Reciprocal Rank Fusion (RRF) to merge results.
//!
//! ## Features
//!
//! - **Vector Search**: Semantic similarity using embeddings
//! - **Keyword Search**: SQLite FTS5 for exact keyword matching
//! - **RRF Fusion**: Combines results from both methods for better recall
//! - **Real Embeddings**: Supports OpenAI and other embedding providers
//!
//! ## How It Works
//!
//! 1. Query is sent to both vector and keyword backends in parallel
//! 2. Results are fused using RRF algorithm with configurable alpha weight
//! 3. Top-k results are returned based on combined scores

mod backend;
mod embedding;
mod extension;
mod fts;
mod fusion;

pub use backend::HybridMemoryBackend;
pub use embedding::{CachedEmbeddingProvider, OpenAIEmbedding, OpenAIEmbeddingConfig};
pub use extension::HybridMemoryExtension;
pub use fts::FTSBackend;
pub use fusion::{linear_fusion, rrf_fusion, FusionConfig};
