# AGENTS.md

This file provides guidance to coding agents working in this repository.

See @README.md for build commands, usage, and configuration.

## Architecture

This is a Rust CLI application using `clap` for argument parsing and `reqwest` for HTTP requests to the Trello API.

- `src/main.rs`: CLI definition with clap derive macros, command dispatch, and output formatting
- `src/client.rs`: `TrelloClient` wrapper around reqwest with Trello API authentication and endpoints
- `src/config.rs`: Credential loading from environment variables (`TRELLO_API_KEY`, `TRELLO_API_TOKEN`) or config file
- `src/models.rs`: serde structs for Trello API request/response serialisation

## Key Patterns

- Environment variables take precedence over the config file; both credentials must be set together
- The `CredentialSource` trait in `src/config.rs` abstracts environment access for testable credential loading
- Position values for move operations accept `top`, `bottom`, or numeric positions (calculated as midpoint between adjacent items)
- Board/list filters accept either 24-character hex IDs or name substrings (case-insensitive)

## Manual Testing

Use the Trello sandbox board for manual integration tests:

- Board short link: `KYvuPThI`
- Board name/alias: `AI` (`ai`)

When a change needs manual Trello verification, prefer running it against this sandbox board.

## Important

- Do not bump the version unless directly instructed.
