#![forbid(unsafe_code)]

mod cli;
mod env;
mod github;
mod gitops;
mod stack;
mod sync_state;
mod util;

use std::process::ExitCode;

fn main() -> ExitCode {
    cli::run()
}
