use clap::{Parser, Subcommand};

/// Monitor and manage Claude Code sessions on Windows
#[derive(Parser)]
#[command(name = "agent-dashboard", version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Open the visual (tamagotchi) dashboard
    View,
    /// Print all session state as JSON
    Json,
}
