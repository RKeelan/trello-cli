use clap::{Parser, Subcommand};

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
    UpdateDesc {
        /// The card ID
        card_id: String,
        /// The new description
        description: String,
    },
    /// Apply a label to a card
    Label {
        /// The card ID
        card_id: String,
        /// The label name
        label_name: String,
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
    let cli = Cli::parse();

    match cli.command {
        Commands::Card { command } => match command {
            CardCommands::UpdateDesc {
                card_id,
                description,
            } => {
                println!("Update card {} description to: {}", card_id, description);
            }
            CardCommands::Label {
                card_id,
                label_name,
            } => {
                println!("Apply label {} to card {}", label_name, card_id);
            }
            CardCommands::Archive { card_id } => {
                println!("Archive card {}", card_id);
            }
            CardCommands::Move { card_id, position } => {
                println!("Move card {} to position {}", card_id, position);
            }
        },
        Commands::List { command } => match command {
            ListCommands::Move { list_id, position } => {
                println!("Move list {} to position {}", list_id, position);
            }
        },
    }
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
    fn parse_card_update_desc() {
        let cli =
            Cli::try_parse_from(["trello", "card", "update-desc", "abc123", "New description"])
                .unwrap();
        match cli.command {
            Commands::Card { command } => match command {
                CardCommands::UpdateDesc {
                    card_id,
                    description,
                } => {
                    assert_eq!(card_id, "abc123");
                    assert_eq!(description, "New description");
                }
                _ => panic!("Expected UpdateDesc command"),
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
                } => {
                    assert_eq!(card_id, "abc123");
                    assert_eq!(label_name, "In-Progress");
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
