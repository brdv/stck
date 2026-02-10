use clap::{Parser, Subcommand};
use std::process::ExitCode;

use crate::env;

#[derive(Debug, Parser)]
#[command(
    name = "stck",
    about = "CLI for working with stacked GitHub pull requests"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Create the next branch in a stack.
    New {
        /// Name of the branch to create.
        branch: String,
    },
    /// Show detected stack and PR state.
    Status,
    /// Restack/rebase the local stack.
    Sync,
    /// Push rewritten branches and update PR base targets.
    Push,
}

pub fn run() -> ExitCode {
    let cli = Cli::parse();

    let command = match cli.command {
        Commands::New { .. } => "new",
        Commands::Status => "status",
        Commands::Sync => "sync",
        Commands::Push => "push",
    };

    let preflight = match env::run_preflight() {
        Ok(preflight) => preflight,
        Err(message) => {
            eprintln!("error: {message}");
            return ExitCode::from(1);
        }
    };
    let _ = preflight.default_branch;

    eprintln!("error: `stck {command}` is not implemented yet");
    ExitCode::from(1)
}
