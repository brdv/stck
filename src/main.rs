mod cli;
mod env;
mod github;
mod gitops;
mod stack;
mod sync_state;

use std::process::ExitCode;

fn main() -> ExitCode {
    cli::run()
}
