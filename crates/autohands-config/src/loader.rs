//! Configuration loader.

use std::fs;
use std::path::Path;

use crate::error::ConfigError;
use crate::schema::Config;

/// Configuration loader with environment variable substitution.
pub struct ConfigLoader;

impl ConfigLoader {
    /// Load configuration from a TOML file.
    pub fn load(path: &Path) -> Result<Config, ConfigError> {
        let content = fs::read_to_string(path)?;
        let expanded = Self::expand_env_vars(&content)?;
        let config: Config = toml::from_str(&expanded)?;
        Ok(config)
    }

    /// Load configuration from a string.
    pub fn load_str(content: &str) -> Result<Config, ConfigError> {
        let expanded = Self::expand_env_vars(content)?;
        let config: Config = toml::from_str(&expanded)?;
        Ok(config)
    }

    /// Expand environment variables in the format `${VAR}`.
    fn expand_env_vars(content: &str) -> Result<String, ConfigError> {
        let mut result = content.to_string();
        let re = regex::Regex::new(r"\$\{([^}]+)\}").unwrap();

        for cap in re.captures_iter(content) {
            let var_name = &cap[1];
            let var_value = std::env::var(var_name).map_err(|_| {
                ConfigError::EnvVarNotSet(var_name.to_string())
            })?;
            result = result.replace(&cap[0], &var_value);
        }

        Ok(result)
    }

    /// Expand shell-style paths (e.g., `~/.config`).
    pub fn expand_path(path: &str) -> String {
        shellexpand::tilde(path).to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_load_empty_config() {
        let config = ConfigLoader::load_str("").unwrap();
        assert_eq!(config.server.port, 8080);
    }

    #[test]
    fn test_expand_path() {
        let expanded = ConfigLoader::expand_path("~/.autohands");
        assert!(!expanded.starts_with('~'));
    }

    #[test]
    fn test_load_basic_config() {
        let content = r#"
            [server]
            host = "0.0.0.0"
            port = 3000
        "#;
        let config = ConfigLoader::load_str(content).unwrap();
        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.port, 3000);
    }

    #[test]
    fn test_load_full_config() {
        let content = r#"
            [server]
            host = "localhost"
            port = 9000

            [agent]
            default = "custom"
            max_turns = 100
            timeout_seconds = 600

            [memory]
            backend = "vector"
        "#;
        let config = ConfigLoader::load_str(content).unwrap();
        assert_eq!(config.server.host, "localhost");
        assert_eq!(config.agent.default, "custom");
        assert_eq!(config.memory.backend, "vector");
    }

    #[test]
    fn test_load_from_file() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "[server]").unwrap();
        writeln!(file, "port = 5000").unwrap();

        let config = ConfigLoader::load(file.path()).unwrap();
        assert_eq!(config.server.port, 5000);
    }

    #[test]
    fn test_load_nonexistent_file() {
        let result = ConfigLoader::load(Path::new("/nonexistent/path/config.toml"));
        assert!(result.is_err());
    }

    #[test]
    fn test_load_invalid_toml() {
        let content = "invalid = [unclosed";
        let result = ConfigLoader::load_str(content);
        assert!(result.is_err());
    }

    #[test]
    fn test_expand_env_vars() {
        // SAFETY: This test runs in isolation and sets a unique test-only env var
        unsafe {
            std::env::set_var("TEST_CONFIG_VAR", "test_value");
        }
        let content = "value = \"${TEST_CONFIG_VAR}\"";
        let expanded = ConfigLoader::expand_env_vars(content).unwrap();
        assert!(expanded.contains("test_value"));
        unsafe {
            std::env::remove_var("TEST_CONFIG_VAR");
        }
    }

    #[test]
    fn test_expand_env_vars_not_set() {
        let content = "value = \"${NONEXISTENT_TEST_VAR_12345}\"";
        let result = ConfigLoader::expand_env_vars(content);
        assert!(result.is_err());
    }

    #[test]
    fn test_expand_env_vars_no_vars() {
        let content = "value = \"no variables here\"";
        let expanded = ConfigLoader::expand_env_vars(content).unwrap();
        assert_eq!(expanded, content);
    }

    #[test]
    fn test_expand_path_no_tilde() {
        let path = "/usr/local/bin";
        let expanded = ConfigLoader::expand_path(path);
        assert_eq!(expanded, path);
    }

    #[test]
    fn test_expand_path_with_tilde() {
        let expanded = ConfigLoader::expand_path("~/test");
        assert!(!expanded.starts_with('~'));
        assert!(expanded.ends_with("/test"));
    }

    #[test]
    fn test_load_with_providers() {
        let content = r#"
            [providers.openai]
            api_key = "sk-test"
            base_url = "https://api.openai.com"
        "#;
        let config = ConfigLoader::load_str(content).unwrap();
        assert!(config.providers.contains_key("openai"));
        let openai = &config.providers["openai"];
        assert_eq!(openai.api_key.as_ref().unwrap(), "sk-test");
    }

    #[test]
    fn test_load_with_extensions() {
        let content = r#"
            [extensions]
            enabled = ["tools-shell", "tools-filesystem"]
            disabled = ["tools-browser"]
        "#;
        let config = ConfigLoader::load_str(content).unwrap();
        assert_eq!(config.extensions.enabled.len(), 2);
        assert_eq!(config.extensions.disabled.len(), 1);
    }

    #[test]
    fn test_load_with_skills() {
        let content = r#"
            [skills]
            enabled = ["coding", "research"]
        "#;
        let config = ConfigLoader::load_str(content).unwrap();
        assert_eq!(config.skills.enabled.len(), 2);
    }
}
