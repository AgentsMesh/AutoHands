//! Ark (ByteDance Volcengine) LLM provider for AutoHands.
//!
//! This provider supports the Ark API (火山引擎方舟平台), which is compatible
//! with the OpenAI API format.

pub mod api;
pub mod converter;
pub mod extension;
pub mod models;
pub mod parser;
pub mod provider;

pub use extension::ArkExtension;
pub use provider::ArkProvider;
