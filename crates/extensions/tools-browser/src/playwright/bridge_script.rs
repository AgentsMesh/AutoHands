//! Bridge JavaScript code loader.
//!
//! Loads the Playwright bridge script from an embedded JavaScript file.

/// Load the bridge JavaScript code.
pub(super) fn generate_bridge_script() -> String {
    include_str!("bridge_script.js").to_string()
}
