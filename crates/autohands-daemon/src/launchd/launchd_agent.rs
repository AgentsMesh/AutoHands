//! LaunchAgent core management: plist generation, install/uninstall.

use std::fs;
use std::io::Write;
use std::path::PathBuf;

use crate::error::DaemonError;
use super::LaunchAgentConfig;

/// macOS LaunchAgent manager.
#[derive(Debug)]
pub struct LaunchAgent {
    pub(super) config: LaunchAgentConfig,
}

impl LaunchAgent {
    /// Create a new LaunchAgent manager.
    pub fn new(config: LaunchAgentConfig) -> Self {
        Self { config }
    }

    /// Get the plist file path.
    pub fn plist_path(&self) -> PathBuf {
        dirs::home_dir()
            .map(|h| h.join("Library").join("LaunchAgents").join(format!("{}.plist", self.config.label)))
            .unwrap_or_else(|| PathBuf::from(format!("/tmp/{}.plist", self.config.label)))
    }

    /// Generate the plist XML content.
    pub fn generate_plist(&self) -> String {
        let mut plist = String::new();
        plist.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        plist.push_str("<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n");
        plist.push_str("<plist version=\"1.0\">\n");
        plist.push_str("<dict>\n");

        plist.push_str("    <key>Label</key>\n");
        plist.push_str(&format!("    <string>{}</string>\n", self.config.label));

        plist.push_str("    <key>Program</key>\n");
        plist.push_str(&format!("    <string>{}</string>\n", self.config.program.display()));

        if !self.config.program_arguments.is_empty() {
            plist.push_str("    <key>ProgramArguments</key>\n");
            plist.push_str("    <array>\n");
            plist.push_str(&format!("        <string>{}</string>\n", self.config.program.display()));
            for arg in &self.config.program_arguments {
                plist.push_str(&format!("        <string>{}</string>\n", escape_xml(arg)));
            }
            plist.push_str("    </array>\n");
        }

        if let Some(ref dir) = self.config.working_directory {
            plist.push_str("    <key>WorkingDirectory</key>\n");
            plist.push_str(&format!("    <string>{}</string>\n", dir.display()));
        }

        plist.push_str("    <key>RunAtLoad</key>\n");
        plist.push_str(&format!("    <{}/>", if self.config.run_at_load { "true" } else { "false" }));
        plist.push('\n');

        plist.push_str("    <key>KeepAlive</key>\n");
        plist.push_str(&format!("    <{}/>", if self.config.keep_alive { "true" } else { "false" }));
        plist.push('\n');

        plist.push_str("    <key>StandardOutPath</key>\n");
        plist.push_str(&format!("    <string>{}</string>\n", self.config.standard_out_path.display()));

        plist.push_str("    <key>StandardErrorPath</key>\n");
        plist.push_str(&format!("    <string>{}</string>\n", self.config.standard_error_path.display()));

        plist.push_str("    <key>ThrottleInterval</key>\n");
        plist.push_str(&format!("    <integer>{}</integer>\n", self.config.throttle_interval));

        plist.push_str("    <key>ProcessType</key>\n");
        plist.push_str(&format!("    <string>{}</string>\n", self.config.process_type));

        if !self.config.environment_variables.is_empty() {
            plist.push_str("    <key>EnvironmentVariables</key>\n");
            plist.push_str("    <dict>\n");
            for (key, value) in &self.config.environment_variables {
                plist.push_str(&format!("        <key>{}</key>\n", escape_xml(key)));
                plist.push_str(&format!("        <string>{}</string>\n", escape_xml(value)));
            }
            plist.push_str("    </dict>\n");
        }

        if let Some(nice) = self.config.nice {
            plist.push_str("    <key>Nice</key>\n");
            plist.push_str(&format!("    <integer>{}</integer>\n", nice));
        }

        if self.config.low_priority_io {
            plist.push_str("    <key>LowPriorityIO</key>\n");
            plist.push_str("    <true/>\n");
        }

        plist.push_str("</dict>\n");
        plist.push_str("</plist>\n");

        plist
    }

    /// Install the LaunchAgent.
    pub fn install(&self) -> Result<(), DaemonError> {
        if let Some(parent) = self.config.standard_out_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                DaemonError::Custom(format!("Failed to create log directory: {}", e))
            })?;
        }
        if let Some(parent) = self.config.standard_error_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                DaemonError::Custom(format!("Failed to create log directory: {}", e))
            })?;
        }

        let plist_path = self.plist_path();
        if let Some(parent) = plist_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                DaemonError::Custom(format!("Failed to create LaunchAgents directory: {}", e))
            })?;
        }

        let content = self.generate_plist();
        let mut file = fs::File::create(&plist_path).map_err(|e| {
            DaemonError::Custom(format!("Failed to create plist file: {}", e))
        })?;
        file.write_all(content.as_bytes()).map_err(|e| {
            DaemonError::Custom(format!("Failed to write plist file: {}", e))
        })?;

        tracing::info!("Created LaunchAgent plist at: {}", plist_path.display());
        self.load()?;

        Ok(())
    }

    /// Uninstall the LaunchAgent.
    pub fn uninstall(&self) -> Result<(), DaemonError> {
        let _ = self.unload();

        let plist_path = self.plist_path();
        if plist_path.exists() {
            fs::remove_file(&plist_path).map_err(|e| {
                DaemonError::Custom(format!("Failed to remove plist file: {}", e))
            })?;
            tracing::info!("Removed LaunchAgent plist: {}", plist_path.display());
        }

        Ok(())
    }

    /// Check if the LaunchAgent is installed.
    pub fn is_installed(&self) -> bool {
        self.plist_path().exists()
    }
}

/// Escape special characters for XML.
pub(super) fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
