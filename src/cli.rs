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

    match cli.command {
        Commands::Status => {
            let stack = match github::discover_linear_stack(
                &preflight.current_branch,
                &preflight.default_branch,
            ) {
                Ok(stack) => stack,
                Err(message) => {
                    eprintln!("error: {message}");
                    return ExitCode::from(1);
                }
            };

            let branch_chain = stack
                .iter()
                .map(|pr| pr.head_ref_name.as_str())
                .collect::<Vec<_>>()
                .join(" <- ");
            println!("Stack: {} <- {}", preflight.default_branch, branch_chain);

            for pr in stack {
                let merged_at = pr.merged_at.as_deref().unwrap_or("none");
                println!(
                    "PR #{} state={} base={} head={} merged_at={}",
                    pr.number, pr.state, pr.base_ref_name, pr.head_ref_name, merged_at
                );
            }

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
