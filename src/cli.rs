//! CLI argument definitions and top-level command dispatch.

use clap::{Parser, Subcommand};
use std::process::ExitCode;

use crate::commands;
use crate::env;

#[derive(Debug, Parser)]
#[command(
    name = "stck",
    about = "CLI for working with stacked GitHub pull requests",
    version
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
    /// Create a PR for the current branch if missing.
    Submit {
        /// Base branch for the PR (defaults to repository default branch).
        #[arg(long)]
        base: Option<String>,
    },
    /// Show detected stack and PR state.
    Status,
    /// Restack/rebase the local stack.
    Sync {
        /// Continue a previously interrupted sync run.
        #[arg(long = "continue", conflicts_with = "reset_sync")]
        continue_sync: bool,
        /// Discard saved sync state and recompute sync from scratch.
        #[arg(long = "reset", conflicts_with = "continue_sync")]
        reset_sync: bool,
    },
    /// Push rewritten branches and update PR base targets.
    Push,
}

/// Parse CLI arguments, run preflight checks, and dispatch to a command handler.
pub fn run() -> ExitCode {
    let cli = Cli::parse();

    let preflight = match env::run_preflight() {
        Ok(preflight) => preflight,
        Err(message) => {
            eprintln!("error: {message}");
            return ExitCode::from(1);
        }
    };

    match cli.command {
        Commands::Status => commands::run_status(&preflight),
        Commands::New { branch } => commands::run_new(&preflight, &branch),
        Commands::Submit { base } => commands::run_submit(&preflight, base.as_deref()),
        Commands::Sync {
            continue_sync,
            reset_sync,
        } => commands::run_sync(&preflight, continue_sync, reset_sync),
        Commands::Push => commands::run_push(&preflight),
    }
}

#[cfg(test)]
mod tests {
    use super::Cli;
    use clap::CommandFactory;

    #[test]
    fn clap_definition_debug_asserts() {
        Cli::command().debug_assert();
    }
}
