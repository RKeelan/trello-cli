# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

See README.md for build commands, usage, and configuration.

## Architecture

This is a Rust CLI application using clap for argument parsing and reqwest for HTTP requests to the Trello API.

**Module structure:**
- `main.rs` - CLI definition using clap derive macros, command dispatch, and output formatting
- `client.rs` - `TrelloClient` struct wrapping reqwest with Trello API authentication and endpoints
- `config.rs` - Credential loading from environment variables (`TRELLO_API_KEY`, `TRELLO_API_TOKEN`) or config file
- `models.rs` - Serde structs for Trello API request/response serialization

**Key patterns:**
- Environment variables take precedence over config file; both credentials must be set together
- The `CredentialSource` trait in config.rs abstracts environment access for testable credential loading
- Position values for move operations accept "top", "bottom", or numeric positions (calculated as midpoint between adjacent items)
- Board/list filters accept either 24-character hex IDs or name substrings (case-insensitive)

**IMPORTANT:**
- Do not bump the version unless directly instructed to do so.