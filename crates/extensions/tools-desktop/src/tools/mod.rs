//! Desktop automation tools.

mod clipboard;
mod keyboard;
mod mouse;
mod screenshot;

pub use clipboard::*;
pub use keyboard::*;
pub use mouse::*;
pub use screenshot::*;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

use autohands_protocols::error::ToolError;

// Helper to run blocking code in a spawned task
pub(crate) async fn run_blocking<F, T>(f: F) -> Result<T, ToolError>
where
    F: FnOnce() -> Result<T, String> + Send + 'static,
    T: Send + 'static,
{
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?
        .map_err(ToolError::ExecutionFailed)
}
