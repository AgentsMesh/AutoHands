//! # AutoHands Config
//!
//! Configuration management for the AutoHands framework.

mod error;
mod loader;
mod schema;
mod validator;

pub use error::ConfigError;
pub use loader::ConfigLoader;
pub use schema::*;
pub use validator::{ConfigValidator, ValidationError, ValidationResult, ValidationWarning};
