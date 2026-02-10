mod cli;
mod env;
mod github;

use std::process::ExitCode;

fn main() -> ExitCode {
    cli::run()
}
