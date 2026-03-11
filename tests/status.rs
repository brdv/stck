mod harness;

use harness::stck_cmd_with_stubbed_tools;
use predicates::prelude::*;

#[test]
fn status_discovers_linear_stack_in_order() {
    let (_temp, mut cmd) = stck_cmd_with_stubbed_tools();
    cmd.arg("status");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains(
            "Stack: main <- feature-base <- feature-branch <- feature-child",
        ))
        .stdout(predicate::str::contains(
            "feature-base PR #100 MERGED base=main",
        ))
        .stdout(predicate::str::contains(
            "* feature-branch PR #101 OPEN base=feature-base [needs_sync]",
        ))
        .stdout(predicate::str::contains(
            "feature-child PR #102 OPEN base=feature-branch",
        ))
        .stdout(predicate::str::contains(
            "Summary: 1 needs_sync, 0 needs_push, 0 base_mismatch",
        ));
}

#[test]
fn shows_auth_remediation_when_gh_is_not_authenticated() {
    let (_temp, mut cmd) = stck_cmd_with_stubbed_tools();
    cmd.env("STCK_TEST_GH_AUTH_FAIL", "1");
    cmd.arg("status");

    cmd.assert().code(1).stderr(predicate::str::contains(
        "error: GitHub CLI is not authenticated; run `gh auth login` and retry",
    ));
}

#[test]
fn shows_missing_origin_remediation() {
    let (_temp, mut cmd) = stck_cmd_with_stubbed_tools();
    cmd.env("STCK_TEST_ORIGIN_MISSING", "1");
    cmd.arg("status");

    cmd.assert().code(1).stderr(predicate::str::contains(
        "error: `origin` remote is missing; add it with `git remote add origin <url>`",
    ));
}

#[test]
fn status_shows_missing_pr_remediation() {
    let (_temp, mut cmd) = stck_cmd_with_stubbed_tools();
    cmd.env("STCK_TEST_MISSING_CURRENT_PR", "1");
    cmd.arg("status");

    cmd.assert().code(1).stderr(predicate::str::contains(
        "error: no PR found for branch feature-branch; create a PR first",
    ));
}

#[test]
fn status_fails_on_non_linear_stack() {
    let (_temp, mut cmd) = stck_cmd_with_stubbed_tools();
    cmd.env("STCK_TEST_NON_LINEAR", "1");
    cmd.arg("status");

    cmd.assert().code(1).stderr(predicate::str::contains(
        "error: non-linear stack detected at feature-branch; child candidates: feature-child-a, feature-child-b",
    ));
}

#[test]
fn status_reports_needs_push_when_branch_diverges_from_origin() {
    let (_temp, mut cmd) = stck_cmd_with_stubbed_tools();
    cmd.env("STCK_TEST_NEEDS_PUSH_BRANCH", "feature-child");
    cmd.arg("status");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains(
            "feature-child PR #102 OPEN base=feature-branch [needs_push]",
        ))
        .stdout(predicate::str::contains(
            "Summary: 1 needs_sync, 1 needs_push, 0 base_mismatch",
        ));
}

#[test]
fn status_skips_needs_push_for_merged_branches() {
    let (_temp, mut cmd) = stck_cmd_with_stubbed_tools();
    cmd.env("STCK_TEST_MISSING_REMOTE_BRANCH", "feature-base");
    cmd.arg("status");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains(
            "feature-base PR #100 MERGED base=main",
        ))
        .stdout(predicate::str::contains(
            "Summary: 1 needs_sync, 0 needs_push, 0 base_mismatch",
        ));
}

#[test]
fn status_reports_needs_sync_when_default_branch_has_advanced() {
    let (_temp, mut cmd) = stck_cmd_with_stubbed_tools();
    cmd.env("STCK_TEST_SYNC_NOOP", "1");
    cmd.env("STCK_TEST_DEFAULT_ADVANCED", "1");
    cmd.arg("status");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains(
            "feature-base PR #100 OPEN base=main [needs_sync]",
        ))
        .stdout(predicate::str::contains(
            "Summary: 1 needs_sync, 0 needs_push, 0 base_mismatch",
        ));
}

#[test]
fn status_from_default_branch_shows_helpful_message() {
    let (_temp, mut cmd) = stck_cmd_with_stubbed_tools();
    cmd.env("STCK_TEST_CURRENT_BRANCH", "main");
    cmd.arg("status");

    cmd.assert().success().stdout(predicate::str::contains(
        "On default branch (main). Run `stck new <branch>` to start a new stack.",
    ));
}

#[test]
fn status_shows_fetch_failure_remediation() {
    let (_temp, mut cmd) = stck_cmd_with_stubbed_tools();
    cmd.env("STCK_TEST_FETCH_FAIL", "1");
    cmd.arg("status");

    cmd.assert().code(1).stderr(predicate::str::contains(
        "error: failed to fetch from `origin`; check remote connectivity and permissions",
    ));
}
