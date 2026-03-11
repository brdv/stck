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
fn version_prints_package_version() {
    let mut cmd = stck_cmd();
    cmd.arg("--version");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}
