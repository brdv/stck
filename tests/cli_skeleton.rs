use assert_cmd::cargo::cargo_bin_cmd;
use assert_cmd::Command;
use predicates::prelude::*;

fn stck_cmd() -> Command {
    cargo_bin_cmd!("stck")
}

#[test]
fn help_lists_all_milestone_zero_commands() {
    let mut cmd = stck_cmd();
    cmd.arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("new"))
        .stdout(predicate::str::contains("status"))
        .stdout(predicate::str::contains("sync"))
        .stdout(predicate::str::contains("push"));
}

#[test]
fn new_placeholder_is_clear() {
    let mut cmd = stck_cmd();
    cmd.args(["new", "feature-x"]);

    cmd.assert().code(1).stderr(predicate::str::contains(
        "error: `stck new` is not implemented yet",
    ));
}

#[test]
fn status_placeholder_is_clear() {
    let mut cmd = stck_cmd();
    cmd.arg("status");

    cmd.assert().code(1).stderr(predicate::str::contains(
        "error: `stck status` is not implemented yet",
    ));
}

#[test]
fn sync_placeholder_is_clear() {
    let mut cmd = stck_cmd();
    cmd.arg("sync");

    cmd.assert().code(1).stderr(predicate::str::contains(
        "error: `stck sync` is not implemented yet",
    ));
}

#[test]
fn push_placeholder_is_clear() {
    let mut cmd = stck_cmd();
    cmd.arg("push");

    cmd.assert().code(1).stderr(predicate::str::contains(
        "error: `stck push` is not implemented yet",
    ));
}
