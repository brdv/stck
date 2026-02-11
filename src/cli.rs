use clap::{Parser, Subcommand};
use std::process::ExitCode;

use crate::env;
use crate::github;
use crate::gitops;
use crate::stack;
use crate::sync_state::{self, SyncState};

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
    Sync {
        /// Continue a previously interrupted sync run.
        #[arg(long = "continue")]
        continue_sync: bool,
    },
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
        Commands::Sync { continue_sync } => run_sync(&preflight, continue_sync),
        Commands::Push => {
            eprintln!("error: `stck push` is not implemented yet");
            ExitCode::from(1)
        }
    }
}

fn run_sync(preflight: &env::PreflightContext, continue_sync: bool) -> ExitCode {
    let existing_state = match sync_state::load() {
        Ok(state) => state,
        Err(message) => {
            eprintln!("error: {message}");
            return ExitCode::from(1);
        }
    };

    let mut state = match existing_state {
        Some(state) => {
            if !continue_sync {
                println!("Resuming previous sync operation from saved state.");
            }
            state
        }
        None => {
            if continue_sync {
                eprintln!("error: no sync state found; run `stck sync` first");
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
            let steps = stack::build_sync_plan(&stack, &preflight.default_branch);
            if steps.is_empty() {
                println!("Stack is already up to date. No sync needed.");
                return ExitCode::SUCCESS;
            }

            let state = SyncState {
                steps,
                completed_steps: 0,
                failed_step: None,
            };
            if let Err(message) = sync_state::save(&state) {
                eprintln!("error: {message}");
                return ExitCode::from(1);
            }
            state
        }
    };

    if let Some(failed_step) = state.failed_step {
        let rebase_in_progress = match gitops::rebase_in_progress() {
            Ok(in_progress) => in_progress,
            Err(message) => {
                eprintln!("error: {message}");
                return ExitCode::from(1);
            }
        };

        if rebase_in_progress {
            eprintln!("error: rebase is still in progress; run `git rebase --continue` (or `git rebase --abort`) before rerunning `stck sync`");
            return ExitCode::from(1);
        }

        // Previous step failed and user resolved it manually; continue from the next step.
        if state.completed_steps <= failed_step {
            state.completed_steps = failed_step + 1;
        }
        state.failed_step = None;
        if let Err(message) = sync_state::save(&state) {
            eprintln!("error: {message}");
            return ExitCode::from(1);
        }
    }

    for index in state.completed_steps..state.steps.len() {
        let step = &state.steps[index];
        let old_base_ref = format!("refs/heads/{}", step.old_base_ref);
        let old_base_sha = match gitops::resolve_ref(&old_base_ref) {
            Ok(sha) => sha,
            Err(message) => {
                eprintln!("error: {message}");
                return ExitCode::from(1);
            }
        };

        println!(
            "$ git rebase --onto {} {} {}",
            step.new_base_ref, old_base_sha, step.branch
        );
        if let Err(message) = gitops::rebase_onto(&step.new_base_ref, &old_base_sha, &step.branch) {
            state.failed_step = Some(index);
            if let Err(save_error) = sync_state::save(&state) {
                eprintln!("error: {save_error}");
                return ExitCode::from(1);
            }
            eprintln!("error: {message}");
            return ExitCode::from(1);
        }

        state.completed_steps = index + 1;
        state.failed_step = None;
        if let Err(message) = sync_state::save(&state) {
            eprintln!("error: {message}");
            return ExitCode::from(1);
        }
    }

    if let Err(message) = sync_state::remove() {
        eprintln!("error: {message}");
        return ExitCode::from(1);
    }
    if state.steps.is_empty() {
        println!("Stack is already up to date. No sync needed.");
    } else {
        println!("Sync succeeded locally. Run `stck push` to update remotes + PR bases.");
    }
    ExitCode::SUCCESS
}
