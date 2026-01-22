# Plan: Add --comment option to card archive command

Add a `--comment` option to `card archive` that posts a comment to the card before archiving it.

## Task 1: Add comment functionality

### Requirements

1. Add `AddComment` struct to `models.rs`:
   - Single field: `text: String` for the comment content
   - The Trello API endpoint is `POST /cards/{id}/actions/comments`

2. Add `add_comment_to_card` method to `TrelloClient` in `client.rs`:
   - Takes `card_id: &str` and `text: &str`
   - Returns `Result<Action>` (reusing the existing Action struct)

3. Add optional `--comment` argument to the `Archive` variant in `CardCommands`:
   - Type: `Option<String>`

4. Update the archive command handler in `main.rs`:
   - If `--comment` is provided with non-empty content, post the comment first
   - If comment posting fails, abort and propagate the error (do not archive)
   - Empty or whitespace-only comments are treated as no comment (skip posting)
   - Output: "Archived card 'X'" (unchanged) - the comment is a side effect, not the focus

5. Add CLI parsing tests for the new `--comment` option

### Verification

- Unit test: Parse `card archive abc123 --comment "Done"` and verify the comment field is captured
- Unit test: Parse `card archive abc123` (without comment) and verify comment is None

### Validation

Run against a real test card:
```bash
trello card archive <card-id> --comment "Archiving: task complete"
trello card show <card-id> --comments
```
Verify:
- The comment appears in the output
- The card shows `archived: yes` (or equivalent archived status)

**Trello card ID:** 696fbfc902bce412b6762b9b
