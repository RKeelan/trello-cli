mod client;
mod config;
mod models;

use std::collections::HashMap;
use std::io::{self, Write};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use regex::RegexBuilder;
use serde::Serialize;

use client::{TrelloClient, compute_position};
use config::Config;
use models::CreateCard;

#[derive(Parser)]
#[command(name = "trello")]
#[command(version, about = "A CLI for managing Trello cards and lists")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Save API credentials to the config file
    Login {
        /// Trello API key
        #[arg(long)]
        api_key: Option<String>,
        /// Trello API token
        #[arg(long)]
        api_token: Option<String>,
    },
    /// Manage cards
    Card {
        #[command(subcommand)]
        command: CardCommands,
    },
    /// Manage lists
    List {
        #[command(subcommand)]
        command: ListCommands,
    },
}

#[derive(Subcommand)]
enum CardCommands {
    /// Create a new card
    Create {
        /// The list ID or list name substring
        list: String,
        /// The card name
        name: String,
        /// Set the card description
        #[arg(short, long)]
        description: Option<String>,
        /// Position: "top", "bottom", or numeric ordinal
        #[arg(short, long, default_value = "bottom")]
        position: String,
        /// Filter by board name or ID when resolving list names
        #[arg(short, long)]
        board: Option<String>,
    },
    /// Update a card (description, labels, comment, archive)
    Update {
        /// The card ID
        card_id: String,
        /// Update the card's description
        #[arg(short, long)]
        description: Option<String>,
        /// Apply a label to the card (repeatable)
        #[arg(short, long)]
        label: Vec<String>,
        /// Remove a label from the card (repeatable)
        #[arg(long)]
        clear_label: Vec<String>,
        /// Add a comment to the card
        #[arg(short, long)]
        comment: Option<String>,
        /// Archive the card
        #[arg(short, long)]
        archive: bool,
        /// Restore (unarchive) the card
        #[arg(short, long)]
        restore: bool,
    },
    /// Change a card's position
    Move {
        /// The card ID
        card_id: String,
        /// Position: "top", "bottom", or a numeric value
        position: String,
    },
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
    },
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
}

#[derive(Serialize)]
struct CardResult {
    id: String,
    board: String,
    list: String,
    title: String,
}

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

fn looks_like_id(input: &str) -> bool {
    input.len() == 24 && input.chars().all(|c| c.is_ascii_hexdigit())
}

fn sanitize_field(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '\t' | '\n' | '\r' => ' ',
            _ => c,
        })
        .collect()
}

/// Format ISO 8601 date string to [YYYY-MM-DD HH:MM] format in UTC.
fn format_comment_date(iso_date: &str) -> String {
    // Parse "2020-03-09T19:41:51.396Z" -> "2020-03-09 19:41"
    // Simple parsing: take first 16 chars (YYYY-MM-DDTHH:MM), replace T with space
    if iso_date.len() >= 16 {
        let date_part = &iso_date[..10];
        let time_part = &iso_date[11..16];
        format!("{} {}", date_part, time_part)
    } else {
        iso_date.to_string()
    }
}

#[derive(Subcommand)]
enum ListCommands {
    /// Change a list's position
    Move {
        /// The list ID
        list_id: String,
        /// Position: "top", "bottom", or a numeric value
        position: String,
    },
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {e:#}");
        std::process::exit(1);
    }
}

fn prompt_value(prompt: &str) -> Result<String> {
    print!("{}", prompt);
    io::stdout().flush()?;
    let mut value = String::new();
    io::stdin()
        .read_line(&mut value)
        .context("Failed to read input")?;
    Ok(value.trim().to_string())
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    if let Commands::Login { api_key, api_token } = cli.command {
        let key = match api_key {
            Some(k) => k,
            None => prompt_value("API key: ")?,
        };
        let token = match api_token {
            Some(t) => t,
            None => {
                rpassword::prompt_password("API token: ").context("Failed to read API token")?
            }
        };
        Config::save(&key, &token)?;
        let path = Config::config_path()?;
        println!("Credentials saved to {}", path.display());
        return Ok(());
    }

    let config = Config::load()?;
    let client = TrelloClient::new(&config);

    match cli.command {
        Commands::Login { .. } => unreachable!(),
        Commands::Card { command } => match command {
            CardCommands::Create {
                list,
                name,
                description,
                position,
                board,
            } => {
                let list_id = client
                    .resolve_list(&list, board.as_deref())
                    .with_context(|| format!("Failed to resolve list '{}'", list))?;

                let pos = match position.as_str() {
                    "top" | "bottom" => position.clone(),
                    _ => {
                        if let Ok(target_pos) = position.parse::<usize>() {
                            let mut cards = client.get_list_cards(&list_id).with_context(|| {
                                format!("Failed to fetch cards for list '{}'", list_id)
                            })?;
                            cards.sort_by(|a, b| a.pos.partial_cmp(&b.pos).unwrap());
                            compute_position(&cards, target_pos)
                        } else {
                            position.clone()
                        }
                    }
                };

                let body = CreateCard {
                    name,
                    pos,
                    id_list: list_id,
                    desc: description,
                };

                let card = client.create_card(&body).context("Failed to create card")?;
                let list = client
                    .get_list(&card.id_list)
                    .with_context(|| format!("Failed to fetch list '{}'", card.id_list))?;

                println!(
                    "Created card '{}' ({}) in list '{}'",
                    card.name, card.id, list.name
                );
            }
            CardCommands::Update {
                card_id,
                description,
                label,
                clear_label,
                comment,
                archive,
                restore,
            } => {
                if description.is_none()
                    && label.is_empty()
                    && clear_label.is_empty()
                    && comment.is_none()
                    && !archive
                    && !restore
                {
                    eprintln!("Error: at least one update flag must be provided");
                    std::process::exit(1);
                }

                if archive && restore {
                    eprintln!("Error: --archive and --restore are mutually exclusive");
                    std::process::exit(1);
                }

                let needs_card = !label.is_empty()
                    || !clear_label.is_empty()
                    || comment.is_some()
                    || archive
                    || restore;

                // Update description
                let desc_card_name = if let Some(ref desc) = description {
                    let card = client
                        .update_card_description(&card_id, desc)
                        .with_context(|| {
                            format!("Failed to update description of card '{}'", card_id)
                        })?;
                    Some(card.name)
                } else {
                    None
                };

                // Fetch card once for label/comment/archive operations
                let card = if needs_card {
                    Some(
                        client
                            .get_card(&card_id)
                            .with_context(|| format!("Failed to fetch card '{}'", card_id))?,
                    )
                } else {
                    None
                };
                let card_name = card
                    .as_ref()
                    .map(|c| c.name.clone())
                    .or(desc_card_name)
                    .unwrap_or_else(|| card_id.clone());

                if description.is_some() {
                    println!("Updated description of card '{}'", card_name);
                }

                // Apply/remove labels
                if !label.is_empty() || !clear_label.is_empty() {
                    let card = card.as_ref().unwrap();
                    let board_labels =
                        client.get_board_labels(&card.id_board).with_context(|| {
                            format!("Failed to fetch labels for card '{}'", card_id)
                        })?;

                    for label_name in &label {
                        client
                            .apply_label_by_name(card, &board_labels, label_name)
                            .with_context(|| {
                                format!(
                                    "Failed to apply label '{}' to card '{}'",
                                    label_name, card.name
                                )
                            })?;
                        println!("Applied label '{}' to card '{}'", label_name, card.name);
                    }

                    for label_name in &clear_label {
                        client
                            .remove_label_by_name(card, &board_labels, label_name)
                            .with_context(|| {
                                format!(
                                    "Failed to remove label '{}' from card '{}'",
                                    label_name, card.name
                                )
                            })?;
                        println!("Removed label '{}' from card '{}'", label_name, card.name);
                    }
                }

                // Add comment
                if let Some(ref text) = comment {
                    let trimmed = text.trim();
                    if !trimmed.is_empty() {
                        client
                            .add_comment_to_card(&card_id, trimmed)
                            .with_context(|| {
                                format!("Failed to add comment to card '{}'", card_id)
                            })?;
                        println!("Added comment to card '{}'", card_name);
                    }
                }

                // Archive or restore
                if archive {
                    let card = card.as_ref().unwrap();
                    client
                        .archive_card(card)
                        .with_context(|| format!("Failed to archive card '{}'", card_id))?;
                    println!("Archived card '{}'", card_name);
                } else if restore {
                    let card = card.as_ref().unwrap();
                    client
                        .restore_card(card)
                        .with_context(|| format!("Failed to restore card '{}'", card_id))?;
                    println!("Restored card '{}'", card_name);
                }
            }
            CardCommands::Move { card_id, position } => {
                let card = client.move_card(&card_id, &position)?;
                println!("Moved card '{}' to position {}", card.name, position);
            }
            CardCommands::Find {
                pattern,
                board,
                list,
                json,
            } => {
                let regex = RegexBuilder::new(&pattern)
                    .case_insensitive(true)
                    .build()
                    .context("Invalid regex pattern")?;

                // Fetch boards
                let boards = if let Some(ref board_filter) = board {
                    if looks_like_id(board_filter) {
                        let b = client.get_board(board_filter).with_context(|| {
                            format!("Board ID '{}' not found or inaccessible", board_filter)
                        })?;
                        vec![b]
                    } else {
                        let all_boards = client
                            .get_member_boards()
                            .context("Failed to fetch boards")?;
                        let board_filter_lower = board_filter.to_lowercase();
                        let filtered: Vec<_> = all_boards
                            .into_iter()
                            .filter(|b| b.name.to_lowercase().contains(&board_filter_lower))
                            .collect();
                        if filtered.is_empty() {
                            eprintln!("No boards matching '{}' found", board_filter);
                            return Ok(());
                        }
                        filtered
                    }
                } else {
                    let all_boards = client
                        .get_member_boards()
                        .context("Failed to fetch boards")?;
                    if all_boards.is_empty() {
                        eprintln!("No boards found");
                        return Ok(());
                    }
                    all_boards
                };

                let mut results: Vec<CardResult> = Vec::new();
                let list_filter_lower = list.as_ref().map(|s| s.to_lowercase());

                for b in &boards {
                    let cards = client
                        .get_board_cards(&b.id)
                        .with_context(|| format!("Failed to fetch cards for board '{}'", b.name))?;
                    let lists = client
                        .get_board_lists(&b.id)
                        .with_context(|| format!("Failed to fetch lists for board '{}'", b.name))?;
                    let list_map: HashMap<String, String> =
                        lists.into_iter().map(|l| (l.id, l.name)).collect();

                    for card in cards {
                        if !regex.is_match(&card.name) {
                            continue;
                        }
                        let Some(list_name) = list_map.get(&card.id_list) else {
                            continue;
                        };

                        // Apply list filter if specified
                        if list_filter_lower.as_ref().is_some_and(|filter_lower| {
                            !list_name.to_lowercase().contains(filter_lower)
                        }) {
                            continue;
                        }

                        results.push(CardResult {
                            id: card.id,
                            board: b.name.clone(),
                            list: list_name.clone(),
                            title: card.name,
                        });
                    }
                }

                if results.is_empty() {
                    eprintln!("No cards found");
                    return Ok(());
                }

                if json {
                    println!(
                        "{}",
                        serde_json::to_string(&results).context("Failed to serialize results")?
                    );
                } else {
                    println!("ID\tBoard\tList\tTitle");
                    for r in &results {
                        println!(
                            "{}\t{}\t{}\t{}",
                            r.id,
                            sanitize_field(&r.board),
                            sanitize_field(&r.list),
                            sanitize_field(&r.title)
                        );
                    }
                }
            }
            CardCommands::Show {
                card_id,
                json,
                comments: include_comments,
            } => {
                let card = client
                    .get_card(&card_id)
                    .with_context(|| format!("Failed to fetch card '{}'", card_id))?;
                let board = client
                    .get_board(&card.id_board)
                    .with_context(|| format!("Failed to fetch board for card '{}'", card_id))?;
                let list = client
                    .get_list(&card.id_list)
                    .with_context(|| format!("Failed to fetch list for card '{}'", card_id))?;

                // Get board labels and filter to those on the card
                let board_labels = client.get_board_labels(&card.id_board).with_context(|| {
                    format!("Failed to fetch labels for board '{}'", board.name)
                })?;
                let labels: Vec<LabelInfo> = board_labels
                    .into_iter()
                    .filter(|l| card.id_labels.contains(&l.id))
                    .map(|l| LabelInfo {
                        name: l.name,
                        color: l.color,
                    })
                    .collect();

                // Fetch comments if requested
                let comments = if include_comments {
                    let mut actions = client.get_card_comments(&card_id).with_context(|| {
                        format!("Failed to fetch comments for card '{}'", card_id)
                    })?;
                    // Reverse to get chronological order (oldest first)
                    actions.reverse();
                    let comment_infos: Vec<CommentInfo> = actions
                        .into_iter()
                        .map(|a| {
                            let author = a
                                .member_creator
                                .full_name
                                .unwrap_or(a.member_creator.username);
                            let date = format_comment_date(&a.date);
                            CommentInfo {
                                date,
                                author,
                                text: a.data.text,
                            }
                        })
                        .collect();
                    Some(comment_infos)
                } else {
                    None
                };

                let result = ShowCardResult {
                    id: card.id,
                    name: card.name,
                    board: board.name,
                    list: list.name,
                    labels,
                    description: card.desc,
                    archived: card.closed,
                    comments,
                };

                if json {
                    println!(
                        "{}",
                        serde_json::to_string(&result).context("Failed to serialize result")?
                    );
                } else {
                    println!("Name: {}", result.name);
                    println!("ID: {}", result.id);
                    println!("Board: {}", result.board);
                    println!("List: {}", result.list);

                    if result.labels.is_empty() {
                        println!("Labels: (none)");
                    } else {
                        let label_strs: Vec<String> = result
                            .labels
                            .iter()
                            .map(|l| match (&l.name.is_empty(), &l.color) {
                                (false, Some(c)) => format!("{} ({})", l.name, c),
                                (false, None) => l.name.clone(),
                                (true, Some(c)) => format!("({})", c),
                                (true, None) => "(no color)".to_string(),
                            })
                            .collect();
                        println!("Labels: {}", label_strs.join(", "));
                    }

                    if result.archived {
                        println!("Archived: yes");
                    }

                    if !result.description.is_empty() {
                        println!("Description:");
                        for line in result.description.lines() {
                            println!("  {}", line);
                        }
                    }

                    if let Some(comments) = result.comments.as_ref().filter(|c| !c.is_empty()) {
                        println!("Comments:");
                        for c in comments {
                            println!("  [{}] {}: {}", c.date, c.author, c.text);
                        }
                    }
                }
            }
        },
        Commands::List { command } => match command {
            ListCommands::Move { list_id, position } => {
                let list = client.move_list(&list_id, &position)?;
                println!("Moved list '{}' to position {}", list.name, position);
            }
        },
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn verify_cli() {
        Cli::command().debug_assert();
    }

    #[test]
    fn parse_card_update_description() {
        let cli = Cli::try_parse_from([
            "trello",
            "card",
            "update",
            "abc123",
            "-d",
            "New description",
        ])
        .unwrap();
        match cli.command {
            Commands::Card { command } => match command {
                CardCommands::Update {
                    card_id,
                    description,
                    label,
                    clear_label,
                    comment,
                    archive,
                    restore,
                } => {
                    assert_eq!(card_id, "abc123");
                    assert_eq!(description, Some("New description".to_string()));
                    assert!(label.is_empty());
                    assert!(clear_label.is_empty());
                    assert_eq!(comment, None);
                    assert!(!archive);
                    assert!(!restore);
                }
                _ => panic!("Expected Update command"),
            },
            _ => panic!("Expected Card command"),
        }
    }

    #[test]
    fn parse_card_create_minimal() {
        let cli = Cli::try_parse_from([
            "trello",
            "card",
            "create",
            "507f1f77bcf86cd799439011",
            "Card name",
        ])
        .unwrap();
        match cli.command {
            Commands::Card { command } => match command {
                CardCommands::Create {
                    list,
                    name,
                    description,
                    position,
                    board,
                } => {
                    assert_eq!(list, "507f1f77bcf86cd799439011");
                    assert_eq!(name, "Card name");
                    assert_eq!(description, None);
                    assert_eq!(position, "bottom");
                    assert_eq!(board, None);
                }
                _ => panic!("Expected Create command"),
            },
            _ => panic!("Expected Card command"),
        }
    }

    #[test]
    fn parse_card_create_with_all_optional_flags() {
        let cli = Cli::try_parse_from([
            "trello",
            "card",
            "create",
            "list123",
            "Card name",
            "-d",
            "desc",
            "-p",
            "top",
            "-b",
            "My Board",
        ])
        .unwrap();
        match cli.command {
            Commands::Card { command } => match command {
                CardCommands::Create {
                    list,
                    name,
                    description,
                    position,
                    board,
                } => {
                    assert_eq!(list, "list123");
                    assert_eq!(name, "Card name");
                    assert_eq!(description, Some("desc".to_string()));
                    assert_eq!(position, "top");
                    assert_eq!(board, Some("My Board".to_string()));
                }
                _ => panic!("Expected Create command"),
            },
            _ => panic!("Expected Card command"),
        }
    }

    #[test]
    fn parse_card_create_list_name_substring() {
        let cli = Cli::try_parse_from(["trello", "card", "create", "To Do", "Card name"]).unwrap();
        match cli.command {
            Commands::Card { command } => match command {
                CardCommands::Create { list, .. } => {
                    assert_eq!(list, "To Do");
                }
                _ => panic!("Expected Create command"),
            },
            _ => panic!("Expected Card command"),
        }
    }

    #[test]
    fn parse_card_update_label() {
        let cli = Cli::try_parse_from(["trello", "card", "update", "abc123", "-l", "Bug"]).unwrap();
        match cli.command {
            Commands::Card { command } => match command {
                CardCommands::Update { card_id, label, .. } => {
                    assert_eq!(card_id, "abc123");
                    assert_eq!(label, vec!["Bug"]);
                }
                _ => panic!("Expected Update command"),
            },
            _ => panic!("Expected Card command"),
        }
    }

    #[test]
    fn parse_card_update_multiple_labels() {
        let cli = Cli::try_parse_from([
            "trello", "card", "update", "abc123", "-l", "Bug", "-l", "Urgent",
        ])
        .unwrap();
        match cli.command {
            Commands::Card { command } => match command {
                CardCommands::Update { card_id, label, .. } => {
                    assert_eq!(card_id, "abc123");
                    assert_eq!(label, vec!["Bug", "Urgent"]);
                }
                _ => panic!("Expected Update command"),
            },
            _ => panic!("Expected Card command"),
        }
    }

    #[test]
    fn parse_card_update_clear_label() {
        let cli =
            Cli::try_parse_from(["trello", "card", "update", "abc123", "--clear-label", "Bug"])
                .unwrap();
        match cli.command {
            Commands::Card { command } => match command {
                CardCommands::Update {
                    card_id,
                    clear_label,
                    ..
                } => {
                    assert_eq!(card_id, "abc123");
                    assert_eq!(clear_label, vec!["Bug"]);
                }
                _ => panic!("Expected Update command"),
            },
            _ => panic!("Expected Card command"),
        }
    }

    #[test]
    fn parse_card_update_multiple_clear_labels() {
        let cli = Cli::try_parse_from([
            "trello",
            "card",
            "update",
            "abc123",
            "--clear-label",
            "Bug",
            "--clear-label",
            "Urgent",
        ])
        .unwrap();
        match cli.command {
            Commands::Card { command } => match command {
                CardCommands::Update {
                    card_id,
                    clear_label,
                    ..
                } => {
                    assert_eq!(card_id, "abc123");
                    assert_eq!(clear_label, vec!["Bug", "Urgent"]);
                }
                _ => panic!("Expected Update command"),
            },
            _ => panic!("Expected Card command"),
        }
    }

    #[test]
    fn parse_card_update_comment() {
        let cli =
            Cli::try_parse_from(["trello", "card", "update", "abc123", "-c", "A comment"]).unwrap();
        match cli.command {
            Commands::Card { command } => match command {
                CardCommands::Update {
                    card_id, comment, ..
                } => {
                    assert_eq!(card_id, "abc123");
                    assert_eq!(comment, Some("A comment".to_string()));
                }
                _ => panic!("Expected Update command"),
            },
            _ => panic!("Expected Card command"),
        }
    }

    #[test]
    fn parse_card_update_archive() {
        let cli = Cli::try_parse_from(["trello", "card", "update", "abc123", "-a"]).unwrap();
        match cli.command {
            Commands::Card { command } => match command {
                CardCommands::Update {
                    card_id, archive, ..
                } => {
                    assert_eq!(card_id, "abc123");
                    assert!(archive);
                }
                _ => panic!("Expected Update command"),
            },
            _ => panic!("Expected Card command"),
        }
    }

    #[test]
    fn parse_card_update_restore() {
        let cli = Cli::try_parse_from(["trello", "card", "update", "abc123", "-r"]).unwrap();
        match cli.command {
            Commands::Card { command } => match command {
                CardCommands::Update {
                    card_id,
                    archive,
                    restore,
                    ..
                } => {
                    assert_eq!(card_id, "abc123");
                    assert!(!archive);
                    assert!(restore);
                }
                _ => panic!("Expected Update command"),
            },
            _ => panic!("Expected Card command"),
        }
    }

    #[test]
    fn parse_card_update_comment_and_archive() {
        let cli = Cli::try_parse_from(["trello", "card", "update", "abc123", "-c", "Done", "-a"])
            .unwrap();
        match cli.command {
            Commands::Card { command } => match command {
                CardCommands::Update {
                    card_id,
                    comment,
                    archive,
                    ..
                } => {
                    assert_eq!(card_id, "abc123");
                    assert_eq!(comment, Some("Done".to_string()));
                    assert!(archive);
                }
                _ => panic!("Expected Update command"),
            },
            _ => panic!("Expected Card command"),
        }
    }

    #[test]
    fn parse_card_update_label_and_clear_label() {
        let cli = Cli::try_parse_from([
            "trello",
            "card",
            "update",
            "abc123",
            "-l",
            "green",
            "--clear-label",
            "red",
        ])
        .unwrap();
        match cli.command {
            Commands::Card { command } => match command {
                CardCommands::Update {
                    card_id,
                    label,
                    clear_label,
                    ..
                } => {
                    assert_eq!(card_id, "abc123");
                    assert_eq!(label, vec!["green"]);
                    assert_eq!(clear_label, vec!["red"]);
                }
                _ => panic!("Expected Update command"),
            },
            _ => panic!("Expected Card command"),
        }
    }

    #[test]
    fn parse_card_update_no_flags_parses_ok() {
        // Clap should parse successfully even with no flags; runtime check rejects it
        let cli = Cli::try_parse_from(["trello", "card", "update", "abc123"]).unwrap();
        match cli.command {
            Commands::Card { command } => match command {
                CardCommands::Update {
                    card_id,
                    description,
                    label,
                    clear_label,
                    comment,
                    archive,
                    restore,
                } => {
                    assert_eq!(card_id, "abc123");
                    assert_eq!(description, None);
                    assert!(label.is_empty());
                    assert!(clear_label.is_empty());
                    assert_eq!(comment, None);
                    assert!(!archive);
                    assert!(!restore);
                }
                _ => panic!("Expected Update command"),
            },
            _ => panic!("Expected Card command"),
        }
    }

    #[test]
    fn parse_card_move() {
        let cli = Cli::try_parse_from(["trello", "card", "move", "abc123", "top"]).unwrap();
        match cli.command {
            Commands::Card { command } => match command {
                CardCommands::Move { card_id, position } => {
                    assert_eq!(card_id, "abc123");
                    assert_eq!(position, "top");
                }
                _ => panic!("Expected Move command"),
            },
            _ => panic!("Expected Card command"),
        }
    }

    #[test]
    fn parse_list_move() {
        let cli = Cli::try_parse_from(["trello", "list", "move", "list456", "bottom"]).unwrap();
        match cli.command {
            Commands::List { command } => match command {
                ListCommands::Move { list_id, position } => {
                    assert_eq!(list_id, "list456");
                    assert_eq!(position, "bottom");
                }
            },
            _ => panic!("Expected List command"),
        }
    }

    #[test]
    fn parse_card_find_minimal() {
        let cli = Cli::try_parse_from(["trello", "card", "find", "bug"]).unwrap();
        match cli.command {
            Commands::Card { command } => match command {
                CardCommands::Find {
                    pattern,
                    board,
                    list,
                    json,
                } => {
                    assert_eq!(pattern, "bug");
                    assert_eq!(board, None);
                    assert_eq!(list, None);
                    assert!(!json);
                }
                _ => panic!("Expected Find command"),
            },
            _ => panic!("Expected Card command"),
        }
    }

    #[test]
    fn parse_card_find_with_board() {
        let cli = Cli::try_parse_from(["trello", "card", "find", "task", "-b", "board"]).unwrap();
        match cli.command {
            Commands::Card { command } => match command {
                CardCommands::Find {
                    pattern,
                    board,
                    list,
                    json,
                } => {
                    assert_eq!(pattern, "task");
                    assert_eq!(board, Some("board".to_string()));
                    assert_eq!(list, None);
                    assert!(!json);
                }
                _ => panic!("Expected Find command"),
            },
            _ => panic!("Expected Card command"),
        }
    }

    #[test]
    fn parse_card_find_with_list() {
        let cli = Cli::try_parse_from(["trello", "card", "find", "urgent", "-l", "list"]).unwrap();
        match cli.command {
            Commands::Card { command } => match command {
                CardCommands::Find {
                    pattern,
                    board,
                    list,
                    json,
                } => {
                    assert_eq!(pattern, "urgent");
                    assert_eq!(board, None);
                    assert_eq!(list, Some("list".to_string()));
                    assert!(!json);
                }
                _ => panic!("Expected Find command"),
            },
            _ => panic!("Expected Card command"),
        }
    }

    #[test]
    fn parse_card_find_with_json() {
        let cli = Cli::try_parse_from(["trello", "card", "find", "test", "--json"]).unwrap();
        match cli.command {
            Commands::Card { command } => match command {
                CardCommands::Find {
                    pattern,
                    board,
                    list,
                    json,
                } => {
                    assert_eq!(pattern, "test");
                    assert_eq!(board, None);
                    assert_eq!(list, None);
                    assert!(json);
                }
                _ => panic!("Expected Find command"),
            },
            _ => panic!("Expected Card command"),
        }
    }

    #[test]
    fn parse_card_find_full() {
        let cli = Cli::try_parse_from([
            "trello",
            "card",
            "find",
            "fix",
            "-b",
            "project",
            "-l",
            "in-progress",
            "--json",
        ])
        .unwrap();
        match cli.command {
            Commands::Card { command } => match command {
                CardCommands::Find {
                    pattern,
                    board,
                    list,
                    json,
                } => {
                    assert_eq!(pattern, "fix");
                    assert_eq!(board, Some("project".to_string()));
                    assert_eq!(list, Some("in-progress".to_string()));
                    assert!(json);
                }
                _ => panic!("Expected Find command"),
            },
            _ => panic!("Expected Card command"),
        }
    }

    #[test]
    fn parse_card_show() {
        let cli = Cli::try_parse_from(["trello", "card", "show", "abc123"]).unwrap();
        match cli.command {
            Commands::Card { command } => match command {
                CardCommands::Show {
                    card_id,
                    json,
                    comments,
                } => {
                    assert_eq!(card_id, "abc123");
                    assert!(!json);
                    assert!(!comments);
                }
                _ => panic!("Expected Show command"),
            },
            _ => panic!("Expected Card command"),
        }
    }

    #[test]
    fn parse_card_show_with_json() {
        let cli = Cli::try_parse_from(["trello", "card", "show", "abc123", "--json"]).unwrap();
        match cli.command {
            Commands::Card { command } => match command {
                CardCommands::Show {
                    card_id,
                    json,
                    comments,
                } => {
                    assert_eq!(card_id, "abc123");
                    assert!(json);
                    assert!(!comments);
                }
                _ => panic!("Expected Show command"),
            },
            _ => panic!("Expected Card command"),
        }
    }

    #[test]
    fn parse_card_show_with_comments() {
        let cli = Cli::try_parse_from(["trello", "card", "show", "abc123", "--comments"]).unwrap();
        match cli.command {
            Commands::Card { command } => match command {
                CardCommands::Show {
                    card_id,
                    json,
                    comments,
                } => {
                    assert_eq!(card_id, "abc123");
                    assert!(!json);
                    assert!(comments);
                }
                _ => panic!("Expected Show command"),
            },
            _ => panic!("Expected Card command"),
        }
    }

    #[test]
    fn parse_card_show_with_json_and_comments() {
        let cli = Cli::try_parse_from(["trello", "card", "show", "abc123", "--json", "--comments"])
            .unwrap();
        match cli.command {
            Commands::Card { command } => match command {
                CardCommands::Show {
                    card_id,
                    json,
                    comments,
                } => {
                    assert_eq!(card_id, "abc123");
                    assert!(json);
                    assert!(comments);
                }
                _ => panic!("Expected Show command"),
            },
            _ => panic!("Expected Card command"),
        }
    }

    #[test]
    fn test_format_comment_date() {
        assert_eq!(
            format_comment_date("2020-03-09T19:41:51.396Z"),
            "2020-03-09 19:41"
        );
        assert_eq!(
            format_comment_date("2024-01-15T10:30:00.000Z"),
            "2024-01-15 10:30"
        );
        // Short string returns as-is
        assert_eq!(format_comment_date("short"), "short");
    }

    #[test]
    fn test_looks_like_id() {
        // Valid 24-character hex string
        assert!(looks_like_id("507f1f77bcf86cd799439011"));

        // Invalid cases
        assert!(!looks_like_id("My Card"));
        assert!(!looks_like_id("abc"));
        assert!(!looks_like_id("GHIJKLMNOPQRSTUVWXYZ1234"));
    }

    #[test]
    fn test_show_card_result_json_serialization() {
        let result = ShowCardResult {
            id: "507f1f77bcf86cd799439011".to_string(),
            name: "Fix login bug".to_string(),
            board: "Project Alpha".to_string(),
            list: "In Progress".to_string(),
            labels: vec![
                LabelInfo {
                    name: "Bug".to_string(),
                    color: Some("red".to_string()),
                },
                LabelInfo {
                    name: "Urgent".to_string(),
                    color: None,
                },
            ],
            description: "The login page times out".to_string(),
            archived: false,
            comments: None,
        };

        let json = serde_json::to_string(&result).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["id"], "507f1f77bcf86cd799439011");
        assert_eq!(parsed["name"], "Fix login bug");
        assert_eq!(parsed["board"], "Project Alpha");
        assert_eq!(parsed["list"], "In Progress");
        assert_eq!(parsed["labels"][0]["name"], "Bug");
        assert_eq!(parsed["labels"][0]["color"], "red");
        assert_eq!(parsed["labels"][1]["name"], "Urgent");
        assert_eq!(parsed["labels"][1]["color"], serde_json::Value::Null);
        assert_eq!(parsed["description"], "The login page times out");
        assert_eq!(parsed["archived"], false);
        // Comments field should not be present when None
        assert!(!parsed.as_object().unwrap().contains_key("comments"));
    }

    #[test]
    fn test_show_card_result_with_comments_serialization() {
        let result = ShowCardResult {
            id: "507f1f77bcf86cd799439011".to_string(),
            name: "Fix login bug".to_string(),
            board: "Project Alpha".to_string(),
            list: "In Progress".to_string(),
            labels: vec![],
            description: "".to_string(),
            archived: true,
            comments: Some(vec![
                CommentInfo {
                    date: "2024-01-15 10:30".to_string(),
                    author: "Alice".to_string(),
                    text: "I can reproduce this".to_string(),
                },
                CommentInfo {
                    date: "2024-01-15 14:45".to_string(),
                    author: "Bob".to_string(),
                    text: "Fixed in commit abc123".to_string(),
                },
            ]),
        };

        let json = serde_json::to_string(&result).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["id"], "507f1f77bcf86cd799439011");
        assert_eq!(parsed["archived"], true);
        assert_eq!(parsed["description"], "");
        assert!(parsed["labels"].as_array().unwrap().is_empty());
        assert_eq!(parsed["comments"][0]["date"], "2024-01-15 10:30");
        assert_eq!(parsed["comments"][0]["author"], "Alice");
        assert_eq!(parsed["comments"][0]["text"], "I can reproduce this");
        assert_eq!(parsed["comments"][1]["date"], "2024-01-15 14:45");
        assert_eq!(parsed["comments"][1]["author"], "Bob");
        assert_eq!(parsed["comments"][1]["text"], "Fixed in commit abc123");
    }

    #[test]
    fn test_label_info_serialization() {
        let label_with_color = LabelInfo {
            name: "Bug".to_string(),
            color: Some("red".to_string()),
        };
        let json = serde_json::to_string(&label_with_color).unwrap();
        assert!(json.contains("\"name\":\"Bug\""));
        assert!(json.contains("\"color\":\"red\""));

        let label_without_color = LabelInfo {
            name: "No Color".to_string(),
            color: None,
        };
        let json = serde_json::to_string(&label_without_color).unwrap();
        assert!(json.contains("\"name\":\"No Color\""));
        assert!(json.contains("\"color\":null"));
    }

    #[test]
    fn test_comment_info_serialization() {
        let comment = CommentInfo {
            date: "2024-01-15 10:30".to_string(),
            author: "Alice".to_string(),
            text: "Test comment".to_string(),
        };
        let json = serde_json::to_string(&comment).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["date"], "2024-01-15 10:30");
        assert_eq!(parsed["author"], "Alice");
        assert_eq!(parsed["text"], "Test comment");
    }

    #[test]
    fn test_add_comment_serialization() {
        use crate::models::AddComment;

        let add_comment = AddComment {
            text: "Task completed successfully".to_string(),
        };
        let json = serde_json::to_string(&add_comment).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["text"], "Task completed successfully");
        assert_eq!(parsed.as_object().unwrap().len(), 1);
    }
}
