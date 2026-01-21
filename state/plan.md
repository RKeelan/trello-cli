# Plan: Config File Authentication

Implement credential loading from a config file with environment variable fallback.

## Task 1: Add config module with credential loading

### Requirements

Create a new `src/config.rs` module that:

1. Defines a public `Config` struct with private `api_key` and `api_token` fields, deriving `serde::Deserialize`
2. Adds getter methods `api_key(&self)` and `api_token(&self)` returning `&str`
3. Implements `Config::load()` that tries sources in order:
   - Environment variables: if **both** `TRELLO_API_KEY` and `TRELLO_API_TOKEN` are set, use them
   - Config file: if file exists and contains both fields, use it
   - Error with clear message listing all sources checked
4. Uses platform-specific config paths via `dirs` crate:
   - macOS: `~/Library/Application Support/trello-cli/config.toml`
   - Linux: `~/.config/trello-cli/config.toml`
   - Windows: `%APPDATA%\trello-cli\config.toml`
5. Add `toml` and `dirs` crates to dependencies

**Credential source precedence (no mixing):**
- If both env vars are set → use env vars exclusively
- If only one env var is set → ignore it, try config file
- If config file exists → use it (must contain both fields)
- Otherwise → error

**Edge case handling:**
- Malformed TOML: Error with file path and parse error details
- File not readable (permissions): Error indicating permission issue
- Partial credentials in file: Error listing which field is missing
- **Credential values never appear in error messages**

**Error message format:**
```
Failed to load Trello credentials.
Checked:
  - Environment variables TRELLO_API_KEY and TRELLO_API_TOKEN: not set
  - Config file /path/to/config.toml: not found
```

Config file format:
```toml
api_key = "your-api-key"
api_token = "your-api-token"
```

### Verification

Unit tests:
- `config_from_env_vars`: Config loads when both env vars are set
- `config_from_file`: Config loads from TOML file (use tempfile)
- `env_vars_override_file`: Both env vars set takes precedence over file
- `partial_env_vars_uses_file`: One env var set → falls through to config file
- `missing_credentials_error`: Error message matches specified format
- `malformed_toml_error`: Parse error includes file path and error details
- `partial_credentials_in_file_error`: Error when file has only one field
- `toml_format_parsing`: The documented TOML format deserializes correctly

### Validation

1. Remove env vars and create config file → credentials load from file
2. Set both env vars with config file present → env vars take precedence
3. Remove both → error message shows both sources checked
4. Create malformed config file → error shows file path and parse error

---

## Task 2: Integrate config into TrelloClient

### Requirements

1. Add `TrelloClient::new(config: &Config)` constructor that uses the getter methods
2. Keep `TrelloClient::from_env()` as a convenience method that calls `Config::load()` then `TrelloClient::new()`
3. Update `main.rs` to use `Config::load()` followed by `TrelloClient::new()`

### Verification

- All existing tests pass unchanged (they use direct struct construction)
- New test: `client_new_from_config` creates client using Config struct
- New test: `from_env_wrapper_works` verifies `from_env()` delegates to Config::load()

### Validation

1. Run CLI commands with credentials from config file only (no env vars set):
   ```bash
   unset TRELLO_API_KEY TRELLO_API_TOKEN
   trello card move FQqwMtsu top
   ```
2. Run CLI with no credentials → verify error message propagates correctly

---

## Version Bump

Bump version to 0.2.0 after Task 2. This is backward compatible: env vars still work, `from_env()` retained. Error message format changes but API signature unchanged.
