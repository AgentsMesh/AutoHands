//! HTTP interface module.
//!
//! Provides REST API endpoints for:
//! - Task submission and management
//! - Agent execution
//! - Admin operations
//! - Health checks and monitoring

pub mod handlers;
pub mod routes;

// Internal modules (not publicly exported)
pub(crate) mod admin;
pub(crate) mod monitoring;
pub(crate) mod openai_compat;
