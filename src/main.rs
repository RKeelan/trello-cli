mod client;
mod models;

use anyhow::Result;
use clap::{Parser, Subcommand};

use client::TrelloClient;

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
    let client = TrelloClient::from_env()?;

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
        },
        Commands::List { command } => match command {
            ListCommands::Move {
                list_id: _,
                position: _,
            } => {
                todo!("Move list")
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
}
