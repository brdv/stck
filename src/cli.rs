use clap::{Parser, Subcommand};
use std::process::ExitCode;

use crate::env;
use crate::github;
use crate::gitops;
use crate::stack;
use crate::sync_state::{self, LastSyncPlan, PushState, SyncState};

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
            if let Some(first_open) =
                stack::first_open_branch_rooted_on_default(&stack, &preflight.default_branch)
            {
                let needs_sync = match gitops::branch_needs_sync_with_default(
                    &preflight.default_branch,
                    &first_open.head_ref_name,
                ) {
                    Ok(needs_sync) => needs_sync,
                    Err(message) => {
                        eprintln!("error: {message}");
                        return ExitCode::from(1);
                    }
                };

                if needs_sync {
                    if let Some(line) = report
                        .lines
                        .iter_mut()
                        .find(|line| line.branch == first_open.head_ref_name)
                    {
                        if !line.flags.contains(&"needs_sync") {
                            line.flags.push("needs_sync");
                            report.summary.needs_sync += 1;
                        }
                    }
                }
            }
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
        Commands::New { branch } => run_new(&preflight, &branch),
        Commands::Submit { base } => run_submit(&preflight, base.as_deref()),
        Commands::Sync {
            continue_sync,
            reset_sync,
        } => run_sync(&preflight, continue_sync, reset_sync),
        Commands::Push => run_push(&preflight),
    }
}

fn run_new(preflight: &env::PreflightContext, new_branch: &str) -> ExitCode {
    let current_branch = &preflight.current_branch;
    let starting_from_default = current_branch == &preflight.default_branch;
    let pr_base_branch = if starting_from_default {
        preflight.default_branch.as_str()
    } else {
        current_branch.as_str()
    };

    let local_exists = match gitops::local_branch_exists(new_branch) {
        Ok(exists) => exists,
        Err(message) => {
            eprintln!("error: {message}");
            return ExitCode::from(1);
        }
    };
    if local_exists {
        eprintln!("error: branch {new_branch} already exists locally; choose a different name");
        return ExitCode::from(1);
    }

    let remote_exists = match gitops::remote_branch_exists(new_branch) {
        Ok(exists) => exists,
        Err(message) => {
            eprintln!("error: {message}");
            return ExitCode::from(1);
        }
    };
    if remote_exists {
        eprintln!("error: branch {new_branch} already exists on origin; choose a different name");
        return ExitCode::from(1);
    }

    if !starting_from_default {
        let has_upstream = match gitops::branch_has_upstream(current_branch) {
            Ok(has_upstream) => has_upstream,
            Err(message) => {
                eprintln!("error: {message}");
                return ExitCode::from(1);
            }
        };

        if !has_upstream {
            println!("$ git push -u origin {}", current_branch);
            if let Err(message) = gitops::push_set_upstream(current_branch) {
                eprintln!("error: {message}");
                return ExitCode::from(1);
            }
        }

        let current_has_pr = match github::pr_exists_for_head(current_branch) {
            Ok(exists) => exists,
            Err(message) => {
                eprintln!("error: {message}");
                return ExitCode::from(1);
            }
        };

        if !current_has_pr {
            let bootstrap_base = match infer_bootstrap_base_for_current_branch(preflight) {
                Ok(base) => base,
                Err(message) => {
                    eprintln!("error: {message}");
                    return ExitCode::from(1);
                }
            };

            println!(
                "$ gh pr create --base {} --head {} --title {} --body \"\"",
                bootstrap_base, current_branch, current_branch
            );
            if let Err(message) = github::create_pr(&bootstrap_base, current_branch, current_branch)
            {
                eprintln!("error: {message}");
                return ExitCode::from(1);
            }
        }
    }

    println!("$ git checkout -b {}", new_branch);
    if let Err(message) = gitops::checkout_new_branch(new_branch) {
        eprintln!("error: {message}");
        return ExitCode::from(1);
    }

    println!("$ git push -u origin {}", new_branch);
    if let Err(message) = gitops::push_set_upstream(new_branch) {
        eprintln!("error: {message}");
        return ExitCode::from(1);
    }

    let has_commits = match gitops::has_commits_between(current_branch, new_branch) {
        Ok(has_commits) => has_commits,
        Err(message) => {
            eprintln!("error: {message}");
            return ExitCode::from(1);
        }
    };
    if !has_commits {
        println!(
            "No branch-only commits in {} yet. Add commits, then run: stck submit --base {}",
            new_branch, pr_base_branch
        );
        return ExitCode::SUCCESS;
    }

    println!(
        "$ gh pr create --base {} --head {} --title {} --body \"\"",
        pr_base_branch, new_branch, new_branch
    );
    if let Err(message) = github::create_pr(pr_base_branch, new_branch, new_branch) {
        eprintln!("error: {message}");
        return ExitCode::from(1);
    }

    println!(
        "Created branch {} and opened a stacked PR targeting {}.",
        new_branch, pr_base_branch
    );
    ExitCode::SUCCESS
}

fn infer_bootstrap_base_for_current_branch(
    preflight: &env::PreflightContext,
) -> Result<String, String> {
    let current_branch = &preflight.current_branch;
    let current_ref = format!("refs/heads/{current_branch}");
    let prs = github::list_pull_requests()?;

    let mut best_parent: Option<(usize, String)> = None;
    for pr in prs {
        if pr.head_ref_name == *current_branch {
            continue;
        }

        let candidate_ref = format!("refs/heads/{}", pr.head_ref_name);
        let is_ancestor = match gitops::ref_is_ancestor(&candidate_ref, &current_ref) {
            Ok(is_ancestor) => is_ancestor,
            Err(_) => continue,
        };
        if !is_ancestor {
            continue;
        }

        let distance = match gitops::commit_distance(&candidate_ref, &current_ref) {
            Ok(distance) => distance,
            Err(_) => continue,
        };
        if distance == 0 {
            continue;
        }

        match &best_parent {
            Some((best_distance, _)) if *best_distance <= distance => {}
            _ => best_parent = Some((distance, pr.head_ref_name)),
        }
    }

    Ok(best_parent
        .map(|(_, branch)| branch)
        .unwrap_or_else(|| preflight.default_branch.clone()))
}

fn run_submit(preflight: &env::PreflightContext, base_override: Option<&str>) -> ExitCode {
    let current_branch = &preflight.current_branch;
    if current_branch == &preflight.default_branch {
        eprintln!(
            "error: cannot submit PR for default branch {}; checkout a feature branch and retry",
            preflight.default_branch
        );
        return ExitCode::from(1);
    }

    let has_upstream = match gitops::branch_has_upstream(current_branch) {
        Ok(has_upstream) => has_upstream,
        Err(message) => {
            eprintln!("error: {message}");
            return ExitCode::from(1);
        }
    };
    if !has_upstream {
        println!("$ git push -u origin {}", current_branch);
        if let Err(message) = gitops::push_set_upstream(current_branch) {
            eprintln!("error: {message}");
            return ExitCode::from(1);
        }
    }

    let current_has_pr = match github::pr_exists_for_head(current_branch) {
        Ok(exists) => exists,
        Err(message) => {
            eprintln!("error: {message}");
            return ExitCode::from(1);
        }
    };
    if current_has_pr {
        println!("Branch {} already has an open PR.", current_branch);
        return ExitCode::SUCCESS;
    }

    let base = base_override.unwrap_or(&preflight.default_branch);
    if base_override.is_none() {
        println!(
            "No --base provided. Defaulting PR base to {}.",
            preflight.default_branch
        );
    }

    println!(
        "$ gh pr create --base {} --head {} --title {} --body \"\"",
        base, current_branch, current_branch
    );
    if let Err(message) = github::create_pr(base, current_branch, current_branch) {
        eprintln!("error: {message}");
        return ExitCode::from(1);
    }

    println!("Created PR for {} targeting {}.", current_branch, base);
    ExitCode::SUCCESS
}

fn run_sync(preflight: &env::PreflightContext, continue_sync: bool, reset_sync: bool) -> ExitCode {
    let original_branch = preflight.current_branch.clone();

    let mut existing_state = match sync_state::load_sync() {
        Ok(state) => state,
        Err(message) => {
            eprintln!("error: {message}");
            return ExitCode::from(1);
        }
    };

    if reset_sync {
        if existing_state.is_some() {
            if let Err(message) = sync_state::clear() {
                eprintln!("error: {message}");
                return ExitCode::from(1);
            }
            println!("Cleared previous sync state. Recomputing from scratch.");
        } else {
            println!("No existing sync state found. Computing sync plan from scratch.");
        }
        existing_state = None;
    }

    let mut state = match existing_state {
        Some(state) => {
            if !continue_sync {
                println!(
                    "Resuming previous sync operation from saved state. Use `stck sync --reset` to discard saved state and recompute."
                );
            }
            state
        }
        None => {
            if continue_sync {
                eprintln!("error: no sync state found; run `stck sync` to compute a new plan");
                return ExitCode::from(1);
            }

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
            let force_rewrite_first_open = if let Some(first_open) =
                stack::first_open_branch_rooted_on_default(&stack, &preflight.default_branch)
            {
                match gitops::branch_needs_sync_with_default(
                    &preflight.default_branch,
                    &first_open.head_ref_name,
                ) {
                    Ok(needs_sync) => needs_sync,
                    Err(message) => {
                        eprintln!("error: {message}");
                        return ExitCode::from(1);
                    }
                }
            } else {
                false
            };

            let steps = stack::build_sync_plan_with_options(
                &stack,
                &preflight.default_branch,
                force_rewrite_first_open,
            );
            if steps.is_empty() {
                println!("Stack is already up to date. No sync needed.");
                return ExitCode::SUCCESS;
            }

            let state = SyncState {
                steps,
                completed_steps: 0,
                failed_step: None,
                failed_step_branch_head: None,
            };
            if let Err(message) = sync_state::save_sync(&state) {
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

        if continue_sync {
            let step = &state.steps[failed_step];
            let branch_ref = format!("refs/heads/{}", step.branch);
            let current_head = match gitops::resolve_ref(&branch_ref) {
                Ok(sha) => sha,
                Err(message) => {
                    eprintln!("error: {message}");
                    return ExitCode::from(1);
                }
            };

            let Some(failed_head) = state.failed_step_branch_head.as_deref() else {
                eprintln!("error: sync state is missing failed-step branch head; rerun `stck sync` to retry");
                return ExitCode::from(1);
            };

            if current_head == failed_head {
                eprintln!("error: no completed rebase detected for {}; resolve with `git rebase --continue` (or rerun `stck sync` to retry the step)", step.branch);
                return ExitCode::from(1);
            }

            if state.completed_steps <= failed_step {
                state.completed_steps = failed_step + 1;
            }
        } else {
            // For plain sync retries, keep legacy behavior and continue from next step.
            if state.completed_steps <= failed_step {
                state.completed_steps = failed_step + 1;
            }
        }
        state.failed_step = None;
        state.failed_step_branch_head = None;
        if let Err(message) = sync_state::save_sync(&state) {
            eprintln!("error: {message}");
            return ExitCode::from(1);
        }
    }

    for index in state.completed_steps..state.steps.len() {
        let step = &state.steps[index];
        let branch_ref = format!("refs/heads/{}", step.branch);
        let branch_head = match gitops::resolve_ref(&branch_ref) {
            Ok(sha) => sha,
            Err(message) => {
                eprintln!("error: {message}");
                return ExitCode::from(1);
            }
        };
        let old_base_sha = match gitops::derive_rebase_boundary(
            &step.old_base_ref,
            &step.new_base_ref,
            &step.branch,
        ) {
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
            state.failed_step_branch_head = Some(branch_head);
            if let Err(save_error) = sync_state::save_sync(&state) {
                eprintln!("error: {save_error}");
                return ExitCode::from(1);
            }
            eprintln!("error: {message}");
            return ExitCode::from(1);
        }

        state.completed_steps = index + 1;
        state.failed_step = None;
        state.failed_step_branch_head = None;
        if let Err(message) = sync_state::save_sync(&state) {
            eprintln!("error: {message}");
            return ExitCode::from(1);
        }
    }

    println!("$ git checkout {}", original_branch);
    if let Err(message) = gitops::checkout_branch(&original_branch) {
        if let Err(clear_error) = sync_state::clear() {
            eprintln!("error: {clear_error}");
            return ExitCode::from(1);
        }
        eprintln!("error: {message}");
        return ExitCode::from(1);
    }

    let last_plan = LastSyncPlan {
        default_branch: preflight.default_branch.clone(),
        retargets: state
            .steps
            .iter()
            .map(|step| stack::RetargetStep {
                branch: step.branch.clone(),
                new_base_ref: step.new_base_ref.clone(),
            })
            .collect(),
    };
    if let Err(message) = sync_state::save_last_sync_plan(&last_plan) {
        eprintln!("error: {message}");
        return ExitCode::from(1);
    }

    if let Err(message) = sync_state::clear() {
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

fn run_push(preflight: &env::PreflightContext) -> ExitCode {
    if let Err(message) = gitops::fetch_origin() {
        eprintln!("error: {message}");
        return ExitCode::from(1);
    }

    let existing_state = match sync_state::load_push() {
        Ok(state) => state,
        Err(message) => {
            eprintln!("error: {message}");
            return ExitCode::from(1);
        }
    };

    let mut state = match existing_state {
        Some(state) => state,
        None => {
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
            let cached_plan = match sync_state::load_last_sync_plan() {
                Ok(plan) => plan,
                Err(message) => {
                    eprintln!("error: {message}");
                    return ExitCode::from(1);
                }
            };
            let retargets = if let Some(plan) = cached_plan {
                if plan.default_branch == preflight.default_branch {
                    plan.retargets
                } else {
                    stack::build_push_retargets(&stack, &preflight.default_branch)
                }
            } else {
                stack::build_push_retargets(&stack, &preflight.default_branch)
            };
            let mut push_branches = Vec::new();
            for branch in stack::build_push_branches(&stack) {
                let needs_push = match gitops::branch_needs_push(&branch) {
                    Ok(needs_push) => needs_push,
                    Err(message) => {
                        eprintln!("error: {message}");
                        return ExitCode::from(1);
                    }
                };

                if needs_push {
                    push_branches.push(branch);
                }
            }

            let state = PushState {
                push_branches,
                completed_pushes: 0,
                retargets,
                completed_retargets: 0,
            };
            if let Err(message) = sync_state::save_push(&state) {
                eprintln!("error: {message}");
                return ExitCode::from(1);
            }
            state
        }
    };
    let starting_completed_pushes = state.completed_pushes;
    let starting_completed_retargets = state.completed_retargets;

    for index in state.completed_pushes..state.push_branches.len() {
        let branch = &state.push_branches[index];
        println!("$ git push --force-with-lease origin {branch}");
        if let Err(message) = gitops::push_force_with_lease(branch) {
            if let Err(save_error) = sync_state::save_push(&state) {
                eprintln!("error: {save_error}");
                return ExitCode::from(1);
            }
            eprintln!("error: {message}");
            return ExitCode::from(1);
        }

        state.completed_pushes = index + 1;
        if let Err(message) = sync_state::save_push(&state) {
            eprintln!("error: {message}");
            return ExitCode::from(1);
        }
    }

    for index in state.completed_retargets..state.retargets.len() {
        let retarget = &state.retargets[index];
        println!(
            "$ gh pr edit {} --base {}",
            retarget.branch, retarget.new_base_ref
        );
        if let Err(message) = github::retarget_pr_base(&retarget.branch, &retarget.new_base_ref) {
            if let Err(save_error) = sync_state::save_push(&state) {
                eprintln!("error: {save_error}");
                return ExitCode::from(1);
            }
            eprintln!("error: {message}");
            return ExitCode::from(1);
        }

        state.completed_retargets = index + 1;
        if let Err(message) = sync_state::save_push(&state) {
            eprintln!("error: {message}");
            return ExitCode::from(1);
        }
    }

    if let Err(message) = sync_state::clear() {
        eprintln!("error: {message}");
        return ExitCode::from(1);
    }
    if let Err(message) = sync_state::clear_last_sync_plan() {
        eprintln!("error: {message}");
        return ExitCode::from(1);
    }
    let pushed_this_run = state
        .completed_pushes
        .saturating_sub(starting_completed_pushes);
    let retargeted_this_run = state
        .completed_retargets
        .saturating_sub(starting_completed_retargets);

    println!(
        "Push succeeded. Pushed {} branch(es) and applied {} PR base update(s) in this run.",
        pushed_this_run, retargeted_this_run
    );
    ExitCode::SUCCESS
}
