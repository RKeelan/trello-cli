use clap::Parser;

#[derive(Parser)]
#[command(name = "trello")]
#[command(version, about = "A CLI for managing Trello cards and lists")]
struct Cli {}

fn main() {
    let _cli = Cli::parse();
}
