//! LLM Provider protocol definitions.
//!
//! Providers connect to LLM APIs (Anthropic, OpenAI, etc.) and provide
//! completion capabilities.

mod traits;
mod request;
mod response;
mod model;

pub use traits::*;
pub use request::*;
pub use response::*;
pub use model::*;
