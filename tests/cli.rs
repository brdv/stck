mod harness;

use harness::stck_cmd;
use predicates::prelude::*;

#[test]
fn help_lists_all_commands() {
    let mut cmd = stck_cmd();
    cmd.arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("new"))
        .stdout(predicate::str::contains("submit"))
        .stdout(predicate::str::contains("status"))
        .stdout(predicate::str::contains("sync"))
        .stdout(predicate::str::contains("push"));
}

#[test]
fn submit_help_describes_parent_auto_discovery() {
    let mut cmd = stck_cmd();
    cmd.args(["submit", "--help"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains(
            "Base branch for the PR (auto-detects the stack parent when omitted)",
        ))
        .stdout(predicate::str::contains("defaults to repository default branch").not());
}

#[test]
fn version_prints_package_version() {
    let mut cmd = stck_cmd();
    cmd.arg("--version");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}
