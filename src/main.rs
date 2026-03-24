mod cli;
mod model;

use clap::Parser;
use cli::{Cli, Command};

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Command::Json) => {
            println!("{{}}");
        }
        Some(Command::View) | None => {
            println!("TUI not implemented yet");
        }
    }
}
