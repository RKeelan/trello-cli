# Plan: Add `card find` Command

Add a command to search for cards by name across boards and lists.

## Task 1: Implement `card find` Command

### Requirements

**Dependencies:**

Add `regex = "1"` to `Cargo.toml`.

**Models (`src/models.rs`):**

Add `Board` struct:
```rust
#[derive(Debug, Deserialize, Clone)]
pub struct Board {
    pub id: String,
    pub name: String,
}
```

Add `Clone` derive to existing `Card` and `List` structs.

**Client (`src/client.rs`):**

Update imports to include `Board` from `crate::models`.

Add methods:
- `get_member_boards()` → `Vec<Board>` — GET `/members/me/boards?filter=open` (excludes archived boards)
- `get_board(board_id: &str)` → `Board` — GET `/boards/{board_id}`
- `get_board_cards(board_id: &str)` → `Vec<Card>` — GET `/boards/{board_id}/cards`

Note: `get_board_lists()` already exists. Error handling follows existing patterns (propagate with context).

**Main (`src/main.rs`):**

Add imports:
- `use std::collections::HashMap;`
- `use regex::RegexBuilder;`
- `use serde::Serialize;`

Add `Find` variant to `CardCommands`:
```rust
/// Find cards matching a pattern
Find {
    /// Regex pattern to match card names
    pattern: String,
    /// Filter by board name or ID
    #[arg(short, long)]
    board: Option<String>,
    /// Filter by list name or ID
    #[arg(short, long)]
    list: Option<String>,
    /// Output as JSON
    #[arg(long)]
    json: bool,
}
```

Add helper struct for output:
```rust
#[derive(Serialize)]
struct CardResult {
    id: String,
    board: String,
    list: String,
    title: String,
}
```

Add helper function:
```rust
fn looks_like_id(input: &str) -> bool {
    input.len() == 24 && input.chars().all(|c| c.is_ascii_hexdigit())
}
```

Add match arm in `run()` function to handle `CardCommands::Find` with the command logic below.

**Command logic:**

1. Compile the regex pattern using `regex::RegexBuilder` with `.case_insensitive(true)`. If compilation fails, return the error with context using `.context("Invalid regex pattern")`. Note: An empty pattern matches all cards, which is a valid use case for listing cards with optional board/list filters.

2. Fetch boards:
   - If `--board` is specified:
     - **ID detection**: If `looks_like_id(input)` returns true, fetch that single board via `get_board()`. If the API call fails, abort with contextual error (e.g., "Board ID 'abc123...' not found or inaccessible: <error>").
     - Otherwise, fetch all boards via `get_member_boards()` and filter to those whose name contains the input as a case-insensitive substring. If no boards match, print "No boards matching '<name>' found" to stderr and exit 0.
   - If `--board` is not specified, fetch all boards via `get_member_boards()`. If this fails, abort with "Failed to fetch boards: <error>".

3. For each board:
   - Fetch cards via `get_board_cards()`. If this fails, abort with "Failed to fetch cards for board '<name>': <error>".
   - Fetch lists via `get_board_lists()` and build `HashMap<String, String>` mapping list IDs to list names. If this fails, abort with "Failed to fetch lists for board '<name>': <error>".

4. Filter cards whose `name` matches the regex. If a card's `id_list` is not found in the HashMap, skip that card (defensive handling of API inconsistencies).

5. If `--list` is specified, filter to cards whose resolved list name contains the input as a case-insensitive substring. (List filtering is name-only because cards are already scoped to specific boards, making list ID lookup unnecessary for the common use case.)

6. Collect results as `CardResult` structs.

**Output:**

- Default table format:
  - Print header row: `ID\tBoard\tList\tTitle`
  - Print one row per result with fields separated by tabs
  - Sanitize Board, List, and Title fields by replacing `\t`, `\n`, `\r` with single space. IDs don't require sanitization.
- `--json`: JSON array of `CardResult` objects (no sanitization; JSON escaping handles control characters)

**Exit behavior:**
- No matches: Print "No cards found" to stderr, exit 0
- No boards found (empty account): Print "No boards found" to stderr, exit 0
- No boards matching filter: Print "No boards matching '<name>' found" to stderr, exit 0
- Invalid regex: Return error (exit 1 via main error handler)
- API errors: Return error with context (exit 1 via main error handler)

Bump version in `Cargo.toml` to 0.2.0.

### Verification

Unit tests in `src/main.rs`:
- `parse_card_find_minimal`: pattern only
- `parse_card_find_with_board`: pattern + `-b "board"`
- `parse_card_find_with_list`: pattern + `-l "list"`
- `parse_card_find_with_json`: pattern + `--json`
- `parse_card_find_full`: all flags combined
- `test_looks_like_id`: verify true for "507f1f77bcf86cd799439011", false for "My Card", "abc", "GHIJKLMNOPQRSTUVWXYZ1234"

Note: Integration tests with mocked HTTP are deferred due to API dependency complexity.

### Validation

Manual validation requires a Trello account with test data.

```bash
# Find all cards matching "bug"
trello card find "bug"

# Find cards on a specific board
trello card find "task" -b "My Board"

# Find cards in a specific list
trello card find "urgent" -l "To Do"

# Combine filters
trello card find "fix" -b "Project" -l "In Progress"

# JSON output
trello card find "test" --json

# Case-insensitive by default
trello card find "BUG"    # matches "bug", "Bug", "BUG"

# Regex patterns
trello card find "^Bug:"  # starts with "Bug:"
trello card find "v[0-9]" # contains version number

# No matches
trello card find "xyznonexistent"  # prints "No cards found"

# Invalid regex
trello card find "["  # prints error about invalid regex
```

---

## Implementation Notes

- API calls are sequential; with many boards this may be slow. The `--board` filter limits scope for large accounts. Performance is acceptable for v1.
- The Trello API's `GET /boards/{id}/cards` returns all non-archived cards.
