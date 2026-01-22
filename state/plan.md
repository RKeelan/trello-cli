# Plan: Add `card show` command

## Task 1: Implement `card show` command

### Requirements

Add a new `card show` subcommand that displays detailed information about a single Trello card. The command should:

1. Accept a card ID as a required positional argument
2. Fetch the card details from the Trello API
3. Resolve related entities (board name, list name, labels) for context
4. Display the following information:
   - Card ID
   - Card name
   - Board name
   - List name
   - Labels (with names and colors)
   - Description (omit the field entirely if empty in human-readable output; empty string in JSON)
   - Archived status (only show in human-readable output if true)
5. Support a `--json` flag for machine-readable output
6. Support a `--comments` flag to include card comments

### Human-Readable Output Format

Use a multi-line key-value format:

```
Name: Fix login timeout bug
ID: 507f1f77bcf86cd799439011
Board: Project Alpha
List: In Progress
Labels: Bug (red), Urgent (orange)
Description:
  The login page times out after 30 seconds.
  Need to increase the timeout or show a loading indicator.
```

If description is empty, omit the "Description:" line entirely.
If no labels are applied, show "Labels: (none)".
If card is archived, add "Archived: yes" after Labels.

**With `--comments` flag:**

```
Name: Fix login timeout bug
ID: 507f1f77bcf86cd799439011
Board: Project Alpha
List: In Progress
Labels: Bug (red), Urgent (orange)
Description:
  The login page times out after 30 seconds.
Comments:
  [2024-01-15 10:30] Alice: I can reproduce this on Chrome.
  [2024-01-15 14:45] Bob: Fixed in commit abc123.
```

Comments are displayed in chronological order (oldest first).
If there are no comments, omit the "Comments:" section entirely.

**Date/time format for comments:**
- Use UTC timezone
- Format: `[YYYY-MM-DD HH:MM]` (24-hour, minutes only)
- Example: Parse ISO 8601 `"2020-03-09T19:41:51.396Z"` → `"[2020-03-09 19:41]"`

### Implementation

**Add to `CardCommands` enum in `src/main.rs`:**
```rust
/// Show detailed information about a card
Show {
    /// The card ID
    card_id: String,
    /// Output as JSON
    #[arg(long)]
    json: bool,
    /// Include comments
    #[arg(long)]
    comments: bool,
},
```

**Add a `ShowCardResult` struct for JSON output:**
```rust
#[derive(Serialize)]
struct ShowCardResult {
    id: String,
    name: String,
    board: String,
    list: String,
    labels: Vec<LabelInfo>,
    description: String,
    archived: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    comments: Option<Vec<CommentInfo>>,
}

#[derive(Serialize)]
struct LabelInfo {
    name: String,
    color: Option<String>,
}

#[derive(Serialize)]
struct CommentInfo {
    date: String,
    author: String,
    text: String,
}
```

Note: `LabelInfo` is a simplified view of `Label` that omits the `id` field, as the ID is not useful for display purposes.

**Add Comment model to `src/models.rs`:**
```rust
/// Represents a Trello action (used for comments)
#[derive(Debug, Deserialize)]
pub struct Action {
    pub id: String,
    #[serde(rename = "type")]
    pub action_type: String,
    pub date: String, // ISO 8601 timestamp, e.g., "2020-03-09T19:41:51.396Z"
    pub data: ActionData,
    #[serde(rename = "memberCreator")]
    pub member_creator: ActionMember, // Always present for commentCard actions
}

#[derive(Debug, Deserialize)]
pub struct ActionData {
    #[serde(default)]
    pub text: String, // Comment text; empty string if not present
}

#[derive(Debug, Deserialize)]
pub struct ActionMember {
    #[serde(rename = "fullName")]
    pub full_name: Option<String>,
    pub username: String,
}
```

**Add `get_card_comments` method to `TrelloClient` in `src/client.rs`:**
```rust
pub fn get_card_comments(&self, card_id: &str) -> Result<Vec<Action>> {
    let mut all_comments = Vec::new();
    let limit = 1000;
    let mut before: Option<String> = None;

    loop {
        let path = match &before {
            Some(id) => format!(
                "/cards/{}/actions?filter=commentCard&limit={}&before={}",
                card_id, limit, id
            ),
            None => format!(
                "/cards/{}/actions?filter=commentCard&limit={}",
                card_id, limit
            ),
        };

        let batch: Vec<Action> = self.get(&path)?;
        if batch.is_empty() {
            break;
        }

        before = batch.last().map(|a| a.id.clone());
        all_comments.extend(batch);

        // If we got fewer than limit, we've reached the end
        if all_comments.len() < limit {
            break;
        }
    }

    Ok(all_comments)
}
```

Notes:
- Trello API returns up to 1000 actions per request; pagination uses the `before` parameter with the last action's ID
- Results are returned in reverse chronological order by the API; reverse them for display (oldest first)

**Transformation from Action to CommentInfo:**
1. Parse ISO 8601 date and format as `[YYYY-MM-DD HH:MM]` UTC
2. Extract author: use `full_name` if present, otherwise fall back to `username`
3. Use `data.text` directly (empty string if not present)

**Label Resolution Strategy:**

The Trello API does not provide an endpoint to fetch individual labels by ID. The approach is to fetch all board labels via `client.get_board_labels()` and filter to those matching `card.id_labels`. This is the same pattern used in `apply_label_by_name` and `remove_label_by_name`.

**Error Handling:**

Fail fast if any API call fails. The command fetches card, board, list, labels, and optionally comments in sequence. If any call fails, propagate the error with context using `anyhow::Context`. This matches the error handling pattern used throughout the codebase.

**Add the match arm in `run()`:**
- Fetch the card using `client.get_card()`
- Fetch board name using `client.get_board()`
- Fetch list name using `client.get_list()`
- Fetch board labels using `client.get_board_labels()` and filter to those in `card.id_labels`
- If `--comments` flag is set, fetch comments using `client.get_card_comments()` and reverse for chronological order
- Build output and print (human-readable or JSON based on `--json` flag)

### Verification

1. Add CLI parsing tests for `card show`:
   - `parse_card_show`: Parses `card show <id>` correctly
   - `parse_card_show_with_json`: Parses `card show <id> --json` correctly
   - `parse_card_show_with_comments`: Parses `card show <id> --comments` correctly
   - `parse_card_show_with_json_and_comments`: Parses `card show <id> --json --comments` correctly

2. Existing `Cli::command().debug_assert()` test will verify the new command structure.

### Validation

Run the command against real Trello cards:

**Basic functionality:**
```bash
# Human-readable output
cargo run -- card show <real-card-id>

# JSON output
cargo run -- card show <real-card-id> --json

# With comments
cargo run -- card show <real-card-id> --comments

# JSON with comments
cargo run -- card show <real-card-id> --json --comments
```

**Edge cases:**
```bash
# Card with no comments (verify "Comments:" section is omitted)
cargo run -- card show <card-without-comments> --comments

# Card with empty description (verify "Description:" line is omitted)
cargo run -- card show <card-without-description>

# Card with no labels (verify "Labels: (none)" is shown)
cargo run -- card show <card-without-labels>

# Archived card (verify "Archived: yes" is shown)
cargo run -- card show <archived-card-id>

# Invalid card ID (verify error message)
cargo run -- card show invalid-id-12345
```

Verify:
- Card details are displayed in the specified format
- Labels show both name and color in parentheses
- Description is displayed with proper indentation (or omitted if empty)
- JSON output is valid and includes all fields
- Archived status displays correctly
- Comments are displayed in chronological order with date, author, and text
- Comments are omitted from JSON when `--comments` flag is not set
- Error handling works for invalid card IDs
