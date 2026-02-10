//! Window management tools.

mod close;
mod list_focus_move;
mod resize_minimize_maximize;

pub use close::*;
pub use list_focus_move::*;
pub use resize_minimize_maximize::*;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

use autohands_protocols::error::ToolError;

// Helper to run blocking code
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
