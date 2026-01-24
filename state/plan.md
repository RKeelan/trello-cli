# Plan: Add card update command

## Task 1: Consolidate card update/label/archive into a unified `card update` command

### Requirements

Replace the existing `card update`, `card label`, and `card archive` subcommands with a single `card update` command that accepts the following flags:

- Positional `card_id` (required)
- `-d`/`--description TEXT` - Update the card's description
- `-l`/`--label LABEL_NAME` - Apply a label to the card (repeatable for multiple labels)
- `--clear-label LABEL_NAME` - Remove a label from the card (repeatable for multiple labels)
- `-c`/`--comment TEXT` - Add a comment to the card
- `-a`/`--archive` - Archive the card

**Breaking change:** The current `card update` command takes a positional `description` argument. This is intentionally replaced by the `-d`/`--description` flag to support the multi-operation interface. No backwards compatibility shim is needed; this tool is pre-1.0 and the positional interface was only added recently.

**Flag enforcement:** At least one flag must be provided. This is enforced as a runtime check after parsing (clap's `ArgGroup` is awkward with derive macros). If no flags are provided, print an error message and exit with a non-zero status.

**Execution order:** When multiple flags are used, operations execute in this order: description, labels (apply then clear), comment, archive. This matches the existing pattern of posting a comment before archiving.

**Error handling:** Stop on first error using `?` propagation. Report which operation failed. Prior successful operations are not rolled back (each is an independent API call). This matches the existing error handling pattern throughout the codebase.

**Comment trimming:** The `-c` flag trims whitespace and skips the API call if the result is empty, matching the existing behaviour in the archive command.

**`-l` flag overlap:** The `-l` short flag means `--list` in `card find` and `--label` in `card update`. This is acceptable because clap scopes short flags per subcommand, and the meanings are contextually distinct.

**Label coalescing:** When multiple `-l` and/or `--clear-label` flags are provided, fetch the card once to resolve the board's labels, then apply/remove all requested labels using that single lookup. Refactor the existing `apply_label_by_name` and `remove_label_by_name` client methods to accept a pre-fetched `Card` rather than fetching it themselves. The dispatch logic fetches the card once and passes it to each call.

Remove the `CardCommands::Label` and `CardCommands::Archive` variants entirely, along with their dispatch logic and tests.

Output should confirm each action applied (e.g., "Updated description of card 'Card Name'", "Applied label 'Bug' to card 'Card Name'", "Added comment to card 'Card Name'", "Archived card 'Card Name'").

### Verification

- Unit tests for clap parsing of the new `card update` command:
  - Basic usage with each flag individually (`-d`, `-l`, `--clear-label`, `-c`, `-a`)
  - Multiple `-l` flags
  - Multiple `--clear-label` flags
  - Combined flags (e.g., `-c "text" -a`, `-l "green" --clear-label "red"`)
- Runtime validation test: error when no flags provided
- Remove existing parsing tests for the old `Label` and `Archive` commands

### Validation

```bash
# Apply a label
cargo run -- card update <card-id> -l "green"

# Remove a label
cargo run -- card update <card-id> --clear-label "green"

# Add a comment
cargo run -- card update <card-id> -c "Test comment"

# Archive with comment
cargo run -- card update <card-id> -c "Done" -a

# Update description
cargo run -- card update <card-id> -d "New description"

# Error: no flags provided (should print error and exit non-zero)
cargo run -- card update <card-id>
```
