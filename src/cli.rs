use clap::{Parser, Subcommand};
use std::process::ExitCode;

use crate::env;
use crate::github;

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

    let preflight = match env::run_preflight() {
        Ok(preflight) => preflight,
        Err(message) => {
            eprintln!("error: {message}");
            return ExitCode::from(1);
        }
    };
    let _ = preflight.default_branch;

    match cli.command {
        Commands::Status => {
            let pr = match github::pr_for_head(&preflight.current_branch) {
                Ok(pr) => pr,
                Err(message) => {
                    eprintln!("error: {message}");
                    return ExitCode::from(1);
                }
            };

            let _ = pr.merged_at;
            println!(
                "PR #{} state={} base={} head={}",
                pr.number, pr.state, pr.base_ref_name, pr.head_ref_name
            );
            ExitCode::SUCCESS
        }
        Commands::New { .. } => {
            eprintln!("error: `stck new` is not implemented yet");
            ExitCode::from(1)
        }
        Commands::Sync => {
            eprintln!("error: `stck sync` is not implemented yet");
            ExitCode::from(1)
        }
        Commands::Push => {
            eprintln!("error: `stck push` is not implemented yet");
            ExitCode::from(1)
        }
    }
}
