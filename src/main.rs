//! Binary entrypoint for the `stck` CLI.

#![forbid(unsafe_code)]

use std::process::ExitCode;

fn main() -> ExitCode {
    stck::run()
}
