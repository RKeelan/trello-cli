mod client;
mod config;
mod models;

use std::collections::HashMap;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use regex::RegexBuilder;
use serde::Serialize;

use client::TrelloClient;
use config::Config;

#[derive(Parser)]
#[command(name = "trello")]
#[command(version, about = "A CLI for managing Trello cards and lists")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
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
    /// Update a card's description
    Update {
        /// The card ID
        card_id: String,
        /// The new description
        description: String,
    },
    /// Apply or remove a label from a card
    Label {
        /// The card ID
        card_id: String,
        /// The label name
        label_name: String,
        /// Remove the label instead of applying it
        #[arg(long)]
        clear: bool,
    },
    /// Archive a card
    Archive {
        /// The card ID
        card_id: String,
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

fn run() -> Result<()> {
    let cli = Cli::parse();
    let config = Config::load()?;
    let client = TrelloClient::new(&config);

    match cli.command {
        Commands::Card { command } => match command {
            CardCommands::Update {
                card_id,
                description,
            } => {
                let card = client.update_card_description(&card_id, &description)?;
                println!("Updated card '{}' description", card.name);
            }
            CardCommands::Label {
                card_id,
                label_name,
                clear,
            } => {
                if clear {
                    let card_name = client.remove_label_by_name(&card_id, &label_name)?;
                    println!("Removed label '{}' from card '{}'", label_name, card_name);
                } else {
                    let card_name = client.apply_label_by_name(&card_id, &label_name)?;
                    println!("Applied label '{}' to card '{}'", label_name, card_name);
                }
            }
            CardCommands::Archive { card_id } => {
                let card_name = client.archive_card(&card_id)?;
                println!("Archived card '{}'", card_name);
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
                        if let Some(ref filter_lower) = list_filter_lower
                            && !list_name.to_lowercase().contains(filter_lower)
                        {
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

                    if let Some(ref comments) = result.comments
                        && !comments.is_empty()
                    {
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
    fn parse_card_update() {
        let cli =
            Cli::try_parse_from(["trello", "card", "update", "abc123", "New description"]).unwrap();
        match cli.command {
            Commands::Card { command } => match command {
                CardCommands::Update {
                    card_id,
                    description,
                } => {
                    assert_eq!(card_id, "abc123");
                    assert_eq!(description, "New description");
                }
                _ => panic!("Expected Update command"),
            },
            _ => panic!("Expected Card command"),
        }
    }

    #[test]
    fn parse_card_label() {
        let cli =
            Cli::try_parse_from(["trello", "card", "label", "abc123", "In-Progress"]).unwrap();
        match cli.command {
            Commands::Card { command } => match command {
                CardCommands::Label {
                    card_id,
                    label_name,
                    clear,
                } => {
                    assert_eq!(card_id, "abc123");
                    assert_eq!(label_name, "In-Progress");
                    assert!(!clear);
                }
                _ => panic!("Expected Label command"),
            },
            _ => panic!("Expected Card command"),
        }
    }

    #[test]
    fn parse_card_label_clear() {
        let cli = Cli::try_parse_from([
            "trello",
            "card",
            "label",
            "abc123",
            "In-Progress",
            "--clear",
        ])
        .unwrap();
        match cli.command {
            Commands::Card { command } => match command {
                CardCommands::Label {
                    card_id,
                    label_name,
                    clear,
                } => {
                    assert_eq!(card_id, "abc123");
                    assert_eq!(label_name, "In-Progress");
                    assert!(clear);
                }
                _ => panic!("Expected Label command"),
            },
            _ => panic!("Expected Card command"),
        }
    }

    #[test]
    fn parse_card_archive() {
        let cli = Cli::try_parse_from(["trello", "card", "archive", "abc123"]).unwrap();
        match cli.command {
            Commands::Card { command } => match command {
                CardCommands::Archive { card_id } => {
                    assert_eq!(card_id, "abc123");
                }
                _ => panic!("Expected Archive command"),
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
}
