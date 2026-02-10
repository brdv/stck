mod cli;
mod env;

use std::process::ExitCode;

fn main() -> ExitCode {
    cli::run()
}
