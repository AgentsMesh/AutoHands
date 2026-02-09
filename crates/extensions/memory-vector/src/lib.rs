//! Vector memory backend for AutoHands.
//!
//! Provides semantic search over memories using vector embeddings.
//! Uses cosine similarity for finding relevant memories.

mod backend;
mod embedding;
mod extension;
mod index;

pub use backend::VectorMemoryBackend;
pub use embedding::{Embedding, EmbeddingError, EmbeddingProvider, SimpleHashEmbedding};
pub use extension::VectorMemoryExtension;
pub use index::{SearchResult, VectorIndex};
