mod cli;
mod env;
mod github;
mod stack;

use std::process::ExitCode;

fn main() -> ExitCode {
    cli::run()
}
