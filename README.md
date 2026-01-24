# trello-cli

[![CI](https://github.com/RKeelan/trello-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/RKeelan/trello-cli/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/trello-cli)](https://crates.io/crates/trello-cli)

A Rust CLI for Trello, optimized for AI agents.

## Usage

```bash
trello login [--api-key <KEY>] [--api-token <TOKEN>]
trello card update <CARD_ID> <DESCRIPTION>
trello card label <CARD_ID> <LABEL_NAME> [--clear]
trello card archive <CARD_ID> [--comment <TEXT>]
trello card move <CARD_ID> <POSITION>
trello card find <PATTERN> [-b <BOARD>] [-l <LIST>] [--json]
trello card show <CARD_ID> [--json] [--comments]
trello list move <LIST_ID> <POSITION>
```

Position values: `top`, `bottom`, or a numeric value.

## Configuration

Get an API key from https://trello.com/power-ups/admin and generate a token for it.

### Option 1: Login command

```bash
trello login
```

This prompts for your API key and token interactively (the token is hidden during input) and saves them to the config file. You can also pass them as flags:

```bash
trello login --api-key YOUR_KEY --api-token YOUR_TOKEN
```

### Option 2: Environment variables

```bash
export TRELLO_API_KEY="your_api_key"
export TRELLO_API_TOKEN="your_api_token"
```

Environment variables take precedence over the config file when both are set.

## Development

Clone the repository:
```bash
git clone https://github.com/RKeelan/trello-cli.git
cd trello-cli
```

Build:
```bash
cargo build
```

Test:
```bash
cargo test
```

Format and lint:
```bash
cargo fmt --all && cargo clippy --fix --all-targets -- -D warnings
```

Run:
```bash
cargo run -- --help
```

Install from source:
```bash
cargo install --path .
```

This installs the `trello` binary to `~/.cargo/bin/`. Ensure this directory is in your `PATH`.
