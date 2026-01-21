#![allow(dead_code)] // Config will be used in Task 2 integration

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::env;
use std::fs;
use std::path::PathBuf;

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

    fn load_from_path(config_path: PathBuf) -> Result<Self> {
        // Try environment variables first (both must be set)
        let key_env = env::var("TRELLO_API_KEY").ok();
        let token_env = env::var("TRELLO_API_TOKEN").ok();

        if let (Some(api_key), Some(api_token)) = (key_env.as_ref(), token_env.as_ref()) {
            return Ok(Config {
                api_key: api_key.clone(),
                api_token: api_token.clone(),
            });
        }

        // Try config file
        let env_status = if key_env.is_some() || token_env.is_some() {
            "only one set (both required)"
        } else {
            "not set"
        };

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

/// Note: Config tests modify environment variables and must be run single-threaded:
/// `cargo test -- --test-threads=1`
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn with_env_vars<F, R>(vars: &[(&str, Option<&str>)], f: F) -> R
    where
        F: FnOnce() -> R,
    {
        // Save original values
        let originals: Vec<_> = vars.iter().map(|(k, _)| (*k, env::var(*k).ok())).collect();

        // Set new values
        // SAFETY: Tests are run with --test-threads=1 to avoid concurrent env var modifications
        for (key, value) in vars {
            match value {
                Some(v) => unsafe { env::set_var(key, v) },
                None => unsafe { env::remove_var(key) },
            }
        }

        let result = f();

        // Restore original values
        // SAFETY: Tests are run with --test-threads=1 to avoid concurrent env var modifications
        for (key, original) in originals {
            match original {
                Some(v) => unsafe { env::set_var(key, v) },
                None => unsafe { env::remove_var(key) },
            }
        }

        result
    }

    #[test]
    fn config_from_env_vars() {
        with_env_vars(
            &[
                ("TRELLO_API_KEY", Some("env_key")),
                ("TRELLO_API_TOKEN", Some("env_token")),
            ],
            || {
                let config = Config::load().unwrap();
                assert_eq!(config.api_key(), "env_key");
                assert_eq!(config.api_token(), "env_token");
            },
        );
    }

    #[test]
    fn config_from_file() {
        with_env_vars(
            &[("TRELLO_API_KEY", None), ("TRELLO_API_TOKEN", None)],
            || {
                let temp_dir = TempDir::new().unwrap();
                let config_path = temp_dir.path().join("config.toml");
                let mut file = fs::File::create(&config_path).unwrap();
                writeln!(file, "api_key = \"file_key\"").unwrap();
                writeln!(file, "api_token = \"file_token\"").unwrap();

                let config = Config::load_from_path(config_path).unwrap();
                assert_eq!(config.api_key(), "file_key");
                assert_eq!(config.api_token(), "file_token");
            },
        );
    }

    #[test]
    fn env_vars_override_file() {
        // When both env vars are set, they take precedence even if file exists
        with_env_vars(
            &[
                ("TRELLO_API_KEY", Some("env_key")),
                ("TRELLO_API_TOKEN", Some("env_token")),
            ],
            || {
                let temp_dir = TempDir::new().unwrap();
                let config_path = temp_dir.path().join("config.toml");
                let mut file = fs::File::create(&config_path).unwrap();
                writeln!(file, "api_key = \"file_key\"").unwrap();
                writeln!(file, "api_token = \"file_token\"").unwrap();

                let config = Config::load_from_path(config_path).unwrap();
                assert_eq!(config.api_key(), "env_key");
                assert_eq!(config.api_token(), "env_token");
            },
        );
    }

    #[test]
    fn partial_env_vars_uses_file() {
        // When only one env var is set, it falls through to file
        with_env_vars(
            &[
                ("TRELLO_API_KEY", Some("only_key")),
                ("TRELLO_API_TOKEN", None),
            ],
            || {
                let temp_dir = TempDir::new().unwrap();
                let config_path = temp_dir.path().join("config.toml");
                let mut file = fs::File::create(&config_path).unwrap();
                writeln!(file, "api_key = \"file_key\"").unwrap();
                writeln!(file, "api_token = \"file_token\"").unwrap();

                // Should use file credentials, not the partial env var
                let config = Config::load_from_path(config_path).unwrap();
                assert_eq!(config.api_key(), "file_key");
                assert_eq!(config.api_token(), "file_token");
            },
        );
    }

    #[test]
    fn missing_credentials_error() {
        with_env_vars(
            &[("TRELLO_API_KEY", None), ("TRELLO_API_TOKEN", None)],
            || {
                let temp_dir = TempDir::new().unwrap();
                let config_path = temp_dir.path().join("config.toml");
                // Don't create the file - test the "not found" case

                let result = Config::load_from_path(config_path.clone());
                assert!(result.is_err());
                let err = result.unwrap_err().to_string();
                assert!(
                    err.contains("Failed to load Trello credentials"),
                    "Error was: {}",
                    err
                );
                assert!(
                    err.contains(
                        "Environment variables TRELLO_API_KEY and TRELLO_API_TOKEN: not set"
                    ),
                    "Error was: {}",
                    err
                );
                assert!(
                    err.contains(&config_path.display().to_string()),
                    "Error was: {}",
                    err
                );
                assert!(err.contains("not found"), "Error was: {}", err);
            },
        );
    }

    #[test]
    fn malformed_toml_error() {
        with_env_vars(
            &[("TRELLO_API_KEY", None), ("TRELLO_API_TOKEN", None)],
            || {
                let temp_dir = TempDir::new().unwrap();
                let config_path = temp_dir.path().join("config.toml");
                fs::write(&config_path, "this is not valid toml {{{").unwrap();

                let result = Config::load_from_path(config_path.clone());
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
            },
        );
    }

    #[test]
    fn partial_credentials_in_file_error() {
        with_env_vars(
            &[("TRELLO_API_KEY", None), ("TRELLO_API_TOKEN", None)],
            || {
                let temp_dir = TempDir::new().unwrap();
                let config_path = temp_dir.path().join("config.toml");
                // Only api_key, missing api_token (empty string means missing)
                fs::write(&config_path, "api_key = \"only_key\"\napi_token = \"\"").unwrap();

                let result = Config::load_from_path(config_path.clone());
                assert!(result.is_err());
                let err = result.unwrap_err().to_string();
                assert!(
                    err.contains(&config_path.display().to_string()),
                    "Error was: {}",
                    err
                );
                assert!(err.contains("api_token"), "Error was: {}", err);
                assert!(err.contains("missing"), "Error was: {}", err);
            },
        );
    }

    #[test]
    fn partial_credentials_in_file_error_missing_key() {
        with_env_vars(
            &[("TRELLO_API_KEY", None), ("TRELLO_API_TOKEN", None)],
            || {
                let temp_dir = TempDir::new().unwrap();
                let config_path = temp_dir.path().join("config.toml");
                // Only api_token, missing api_key (empty string means missing)
                fs::write(&config_path, "api_key = \"\"\napi_token = \"only_token\"").unwrap();

                let result = Config::load_from_path(config_path.clone());
                assert!(result.is_err());
                let err = result.unwrap_err().to_string();
                assert!(
                    err.contains(&config_path.display().to_string()),
                    "Error was: {}",
                    err
                );
                assert!(err.contains("api_key"), "Error was: {}", err);
                assert!(err.contains("missing"), "Error was: {}", err);
            },
        );
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
