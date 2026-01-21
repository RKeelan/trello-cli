# Plan: Config File Authentication

Implement credential loading from a config file with environment variable fallback.

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
