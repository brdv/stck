//! Core implementation for `stck`, a CLI for working with stacked GitHub pull requests.
//!
//! The crate intentionally stays close to native `git` and `gh` workflows. It keeps
//! CLI parsing, subprocess-backed integrations, stack planning, and resumable
//! operation state separate so the user-facing commands remain predictable and
//! testable.

#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(rustdoc::missing_crate_level_docs)]

mod cli;
mod commands;
mod env;
mod github;
mod gitops;
mod stack;
mod sync_state;
mod util;

use std::process::ExitCode;

/// Parse CLI arguments, run preflight checks, and execute the selected command.
pub fn run() -> ExitCode {
    cli::run()
}
