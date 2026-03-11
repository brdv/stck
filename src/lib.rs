#![forbid(unsafe_code)]

mod cli;
mod commands;
mod env;
mod github;
mod gitops;
mod stack;
mod sync_state;
mod util;

use std::process::ExitCode;

pub fn run() -> ExitCode {
    cli::run()
}
