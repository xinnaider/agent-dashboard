mod app;
mod cli;
mod model;
mod session;
mod ui;
mod view_ui;

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
