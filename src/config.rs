use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::env;
use std::fs;
use std::path::PathBuf;

/// Trait for abstracting environment variable access, enabling testability.
pub(crate) trait CredentialSource {
    fn get(&self, key: &str) -> Option<String>;
}

/// Default implementation that reads from actual environment variables.
struct EnvSource;

impl CredentialSource for EnvSource {
    fn get(&self, key: &str) -> Option<String> {
        env::var(key).ok()
    }
}

#[derive(Debug, Deserialize)]
pub struct Config {
    api_key: String,
    api_token: String,
}

impl Config {
    pub fn api_key(&self) -> &str {
        &self.api_key
    }

    pub fn api_token(&self) -> &str {
        &self.api_token
    }

    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;
        Self::load_from_path(config_path)
    }

    pub fn load_from_path(config_path: PathBuf) -> Result<Self> {
        Self::load_from_source(&EnvSource, config_path)
    }

    pub(crate) fn load_from_source(
        source: &impl CredentialSource,
        config_path: PathBuf,
    ) -> Result<Self> {
        // Try environment variables first (both must be set)
        let key_env = source.get("TRELLO_API_KEY");
        let token_env = source.get("TRELLO_API_TOKEN");

        match (key_env, token_env) {
            (Some(api_key), Some(api_token)) => Ok(Config { api_key, api_token }),
            (Some(_), None) | (None, Some(_)) => {
                Self::load_from_file(config_path, "only one set (both required)")
            }
            (None, None) => Self::load_from_file(config_path, "not set"),
        }
    }

    fn load_from_file(config_path: PathBuf, env_status: &str) -> Result<Self> {
        if config_path.exists() {
            let contents = fs::read_to_string(&config_path).with_context(|| {
                format!(
                    "Failed to read config file {}: permission denied or file not readable",
                    config_path.display()
                )
            })?;

            let config: Config = toml::from_str(&contents).with_context(|| {
                format!("Failed to parse config file {}", config_path.display())
            })?;

            // Validate both fields are present (non-empty)
            if config.api_key.is_empty() {
                bail!(
                    "Config file {} is missing api_key field",
                    config_path.display()
                );
            }
            if config.api_token.is_empty() {
                bail!(
                    "Config file {} is missing api_token field",
                    config_path.display()
                );
            }

            return Ok(config);
        }

        bail!(
            "Failed to load Trello credentials.\nChecked:\n  \
             - Environment variables TRELLO_API_KEY and TRELLO_API_TOKEN: {}\n  \
             - Config file {}: not found",
            env_status,
            config_path.display()
        );
    }

    fn config_path() -> Result<PathBuf> {
        let config_dir =
            dirs::config_dir().context("Could not determine config directory for this platform")?;
        Ok(config_dir.join("trello-cli").join("config.toml"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::io::Write;
    use tempfile::TempDir;

    /// Mock credential source for testing without environment variable manipulation.
    struct MockSource(HashMap<String, String>);

    impl MockSource {
        fn new() -> Self {
            MockSource(HashMap::new())
        }

        fn with(vars: &[(&str, &str)]) -> Self {
            let map = vars
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();
            MockSource(map)
        }
    }

    impl CredentialSource for MockSource {
        fn get(&self, key: &str) -> Option<String> {
            self.0.get(key).cloned()
        }
    }

    #[test]
    fn config_from_env_vars() {
        let source = MockSource::with(&[
            ("TRELLO_API_KEY", "env_key"),
            ("TRELLO_API_TOKEN", "env_token"),
        ]);
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let config = Config::load_from_source(&source, config_path).unwrap();
        assert_eq!(config.api_key(), "env_key");
        assert_eq!(config.api_token(), "env_token");
    }

    #[test]
    fn config_from_file() {
        let source = MockSource::new();
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        let mut file = fs::File::create(&config_path).unwrap();
        writeln!(file, "api_key = \"file_key\"").unwrap();
        writeln!(file, "api_token = \"file_token\"").unwrap();

        let config = Config::load_from_source(&source, config_path).unwrap();
        assert_eq!(config.api_key(), "file_key");
        assert_eq!(config.api_token(), "file_token");
    }

    #[test]
    fn env_vars_override_file() {
        // When both env vars are set, they take precedence even if file exists
        let source = MockSource::with(&[
            ("TRELLO_API_KEY", "env_key"),
            ("TRELLO_API_TOKEN", "env_token"),
        ]);
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        let mut file = fs::File::create(&config_path).unwrap();
        writeln!(file, "api_key = \"file_key\"").unwrap();
        writeln!(file, "api_token = \"file_token\"").unwrap();

        let config = Config::load_from_source(&source, config_path).unwrap();
        assert_eq!(config.api_key(), "env_key");
        assert_eq!(config.api_token(), "env_token");
    }

    #[test]
    fn partial_env_vars_uses_file() {
        // When only one env var is set, it falls through to file
        let source = MockSource::with(&[("TRELLO_API_KEY", "only_key")]);
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        let mut file = fs::File::create(&config_path).unwrap();
        writeln!(file, "api_key = \"file_key\"").unwrap();
        writeln!(file, "api_token = \"file_token\"").unwrap();

        // Should use file credentials, not the partial env var
        let config = Config::load_from_source(&source, config_path).unwrap();
        assert_eq!(config.api_key(), "file_key");
        assert_eq!(config.api_token(), "file_token");
    }

    #[test]
    fn missing_credentials_error() {
        let source = MockSource::new();
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        // Don't create the file - test the "not found" case

        let result = Config::load_from_source(&source, config_path.clone());
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("Failed to load Trello credentials"),
            "Error was: {}",
            err
        );
        assert!(
            err.contains("Environment variables TRELLO_API_KEY and TRELLO_API_TOKEN: not set"),
            "Error was: {}",
            err
        );
        assert!(
            err.contains(&config_path.display().to_string()),
            "Error was: {}",
            err
        );
        assert!(err.contains("not found"), "Error was: {}", err);
    }

    #[test]
    fn malformed_toml_error() {
        let source = MockSource::new();
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        fs::write(&config_path, "this is not valid toml {{{").unwrap();

        let result = Config::load_from_source(&source, config_path.clone());
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        // Error should include file path
        assert!(
            err.contains(&config_path.display().to_string()),
            "Error was: {}",
            err
        );
        // TOML parse errors include details about what went wrong
        assert!(
            err.contains("parse") || err.contains("expected") || err.contains("invalid"),
            "Error was: {}",
            err
        );
    }

    #[test]
    fn partial_credentials_in_file_error() {
        let source = MockSource::new();
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        // Only api_key, missing api_token (empty string means missing)
        fs::write(&config_path, "api_key = \"only_key\"\napi_token = \"\"").unwrap();

        let result = Config::load_from_source(&source, config_path.clone());
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains(&config_path.display().to_string()),
            "Error was: {}",
            err
        );
        assert!(err.contains("api_token"), "Error was: {}", err);
        assert!(err.contains("missing"), "Error was: {}", err);
    }

    #[test]
    fn partial_credentials_in_file_error_missing_key() {
        let source = MockSource::new();
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        // Only api_token, missing api_key (empty string means missing)
        fs::write(&config_path, "api_key = \"\"\napi_token = \"only_token\"").unwrap();

        let result = Config::load_from_source(&source, config_path.clone());
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains(&config_path.display().to_string()),
            "Error was: {}",
            err
        );
        assert!(err.contains("api_key"), "Error was: {}", err);
        assert!(err.contains("missing"), "Error was: {}", err);
    }

    #[test]
    fn toml_format_parsing() {
        let toml_content = r#"
api_key = "your-api-key"
api_token = "your-api-token"
"#;
        let config: Config = toml::from_str(toml_content).unwrap();
        assert_eq!(config.api_key(), "your-api-key");
        assert_eq!(config.api_token(), "your-api-token");
    }
}
