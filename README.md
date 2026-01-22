# trello-cli

[![CI](https://github.com/RKeelan/trello-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/RKeelan/trello-cli/actions/workflows/ci.yml)

A Rust CLI for Trello, optimized for AI agents.

## Usage

```bash
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

Get your API key from https://trello.com/power-ups/admin and generate a token for it.

### Option 1: Config file

Create a config file at `~/.config/trello-cli/config.toml` (Linux) or `~/Library/Application Support/trello-cli/config.toml` (macOS):

```toml
api_key = "your_api_key"
api_token = "your_api_token"
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

Run:
```bash
cargo run -- --help
```

Install from source:
```bash
cargo install --path .
```

This installs the `trello` binary to `~/.cargo/bin/`. Ensure this directory is in your `PATH`.
```