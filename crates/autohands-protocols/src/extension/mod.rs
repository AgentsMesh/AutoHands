//! Extension protocol definitions.
//!
//! Extensions are the fundamental building blocks of AutoHands.

mod traits;
mod manifest;
mod context;
mod event;

pub use traits::*;
pub use manifest::*;
pub use context::*;
pub use event::*;
