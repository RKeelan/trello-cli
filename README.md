# trello-cli

[![CI](https://github.com/RKeelan/trello-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/RKeelan/trello-cli/actions/workflows/ci.yml)

A Rust CLI for Trello, optimized for AI agents.

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

## Configuration

Set the following environment variables:

```bash
export TRELLO_API_KEY="your_api_key"
export TRELLO_API_TOKEN="your_api_token"
```

Get your API key from https://trello.com/power-ups/admin and generate a token for it.

## Usage

```bash
trello card update <CARD_ID> <DESCRIPTION>
trello card label <CARD_ID> <LABEL_NAME> [-clear]
trello card archive <CARD_ID>
trello card move <CARD_ID> <POSITION>
trello list move <LIST_ID> <POSITION>
```

Position values: `top`, `bottom`, or a numeric value.
