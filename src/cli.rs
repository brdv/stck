use clap::{Parser, Subcommand};
use std::process::ExitCode;

use crate::env;
use crate::github;
use crate::gitops;
use crate::stack;

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
            if let Err(message) = gitops::fetch_origin() {
                eprintln!("error: {message}");
                return ExitCode::from(1);
            }

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
            let mut report = stack::build_status_report(&stack, &preflight.default_branch);
            for line in &mut report.lines {
                let needs_push = match gitops::branch_needs_push(&line.branch) {
                    Ok(needs_push) => needs_push,
                    Err(message) => {
                        eprintln!("error: {message}");
                        return ExitCode::from(1);
                    }
                };

                if needs_push {
                    line.flags.push("needs_push");
                    report.summary.needs_push += 1;
                }
            }

            let branch_chain = stack
                .iter()
                .map(|pr| pr.head_ref_name.as_str())
                .collect::<Vec<_>>()
                .join(" <- ");

            println!("Stack: {} <- {}", preflight.default_branch, branch_chain);

            for line in report.lines {
                let flags = if line.flags.is_empty() {
                    "none".to_string()
                } else {
                    line.flags.join(",")
                };
                println!(
                    "{} PR #{} [{}] base={} head={} flags={}",
                    line.branch, line.number, line.state, line.base, line.head, flags
                );
            }

            println!(
                "Summary: needs_sync={} needs_push={} base_mismatch={}",
                report.summary.needs_sync, report.summary.needs_push, report.summary.base_mismatch
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
