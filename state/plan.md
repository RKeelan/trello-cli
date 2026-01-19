# Remaining Implementation Plan

## Step 7: Implement card position change

### Models
Add to `src/models.rs`:
```rust
#[derive(Debug, Serialize)]
pub struct UpdateCardPosition {
    pub pos: String,
}
```

### Client
Add to `src/client.rs`:
```rust
pub fn move_card(&self, card_id: &str, position: &str) -> Result<Card> {
    let path = format!("/cards/{}", card_id);
    let body = UpdateCardPosition { pos: position.to_string() };
    self.put(&path, &body)
}
```

### Main
Update `CardCommands::Move` handler:
```rust
CardCommands::Move { card_id, position } => {
    let card = client.move_card(&card_id, &position)?;
    println!("Moved card '{}' to position {}", card.name, position);
}
```

---

## Step 8: Implement list position change

### Models
Add to `src/models.rs`:
```rust
#[derive(Debug, Deserialize)]
pub struct List {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct UpdateListPosition {
    pub pos: String,
}
```

### Client
Add to `src/client.rs`:
```rust
pub fn move_list(&self, list_id: &str, position: &str) -> Result<List> {
    let path = format!("/lists/{}", list_id);
    let body = UpdateListPosition { pos: position.to_string() };
    self.put(&path, &body)
}
```

### Main
Update `ListCommands::Move` handler:
```rust
ListCommands::Move { list_id, position } => {
    let list = client.move_list(&list_id, &position)?;
    println!("Moved list '{}' to position {}", list.name, position);
}
```

### README
Finalize with all commands documented (already done).

---

## Checklist for each step
- [ ] Add types to models.rs
- [ ] Add method to client.rs
- [ ] Wire up CLI handler in main.rs
- [ ] Run `cargo test`
- [ ] Run `cargo fmt`
- [ ] Run `cargo clippy --all-targets -- -D warnings`
- [ ] Commit
