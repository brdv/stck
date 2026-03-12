mod harness;

use harness::{log_path, stck_cmd_with_stubbed_tools};
use predicates::prelude::*;
use std::fs;

#[test]
fn commands_show_placeholder_when_preflight_passes() {
    let command = "new";
    let (_temp, mut cmd) = stck_cmd_with_stubbed_tools();
    cmd.args([command, "feature-x"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains(
            "No branch-only commits in feature-x yet.",
        ))
        .stdout(predicate::str::contains(
            "stck submit --base feature-branch",
        ));
}

#[test]
fn new_bootstraps_current_branch_then_creates_stacked_branch_and_pr() {
    let (temp, mut cmd) = stck_cmd_with_stubbed_tools();
    let log_path = log_path(&temp, "stck-new-bootstrap.log");
    cmd.env("STCK_TEST_LOG", log_path.as_os_str());
    cmd.env("STCK_TEST_NEW_BRANCH_HAS_COMMITS", "1");
    cmd.arg("new");
    cmd.arg("feature-next");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("$ git push -u origin feature-branch"))
        .stdout(predicate::str::contains(
            "$ gh pr create --base main --head feature-branch --title feature-branch --body \"\"",
        ))
        .stdout(predicate::str::contains("$ git checkout -b feature-next"))
        .stdout(predicate::str::contains("$ git push -u origin feature-next"))
        .stdout(predicate::str::contains(
            "$ gh pr create --base feature-branch --head feature-next --title feature-next --body \"\"",
        ))
        .stdout(predicate::str::contains(
            "Created branch feature-next and opened a stacked PR targeting feature-branch.",
        ));
}

#[test]
fn new_skips_bootstrap_when_current_branch_has_upstream_and_pr() {
    let (temp, mut cmd) = stck_cmd_with_stubbed_tools();
    let log_path = log_path(&temp, "stck-new-skip-bootstrap.log");
    cmd.env("STCK_TEST_LOG", log_path.as_os_str());
    cmd.env("STCK_TEST_HAS_UPSTREAM", "1");
    cmd.env("STCK_TEST_HAS_CURRENT_PR", "1");
    cmd.env("STCK_TEST_NEW_BRANCH_HAS_COMMITS", "1");
    cmd.args(["new", "feature-next"]);

    cmd.assert().success();

    let log = fs::read_to_string(&log_path).expect("new log should exist");
    assert!(!log.contains("push -u origin feature-branch"));
    assert!(!log.contains("pr create --base main --head feature-branch"));
    assert!(log.contains("checkout -b feature-next"));
    assert!(log.contains("push -u origin feature-next"));
    assert!(log.contains("pr create --base feature-branch --head feature-next"));
}

#[test]
fn new_surfaces_checkout_failure() {
    let (_temp, mut cmd) = stck_cmd_with_stubbed_tools();
    cmd.env("STCK_TEST_CHECKOUT_FAIL_BRANCH", "feature-next");
    cmd.args(["new", "feature-next"]);

    cmd.assert().code(1).stderr(predicate::str::contains(
        "error: failed to create and checkout branch feature-next; ensure the branch name is valid and does not already exist",
    ));
}

#[test]
fn new_reports_no_changes_for_new_branch_when_no_commits_exist() {
    let (_temp, mut cmd) = stck_cmd_with_stubbed_tools();
    cmd.args(["new", "feature-next"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains(
            "No branch-only commits in feature-next yet.",
        ))
        .stdout(predicate::str::contains(
            "stck submit --base feature-branch",
        ));
}

#[test]
fn new_from_stacked_branch_discovers_parent_base() {
    let (temp, mut cmd) = stck_cmd_with_stubbed_tools();
    let log_path = log_path(&temp, "stck-new-stacked-parent.log");
    cmd.env("STCK_TEST_LOG", log_path.as_os_str());
    cmd.env("STCK_TEST_NEW_BRANCH_HAS_COMMITS", "1");
    cmd.env(
        "STCK_TEST_OPEN_PRS_JSON",
        r#"[{"number":100,"headRefName":"feature-base","baseRefName":"main","state":"OPEN","mergedAt":null}]"#,
    );
    cmd.args(["new", "feature-next"]);

    cmd.assert().success().stdout(predicate::str::contains(
        "$ gh pr create --base feature-base --head feature-branch",
    ));

    let log = fs::read_to_string(&log_path).expect("new log should exist");
    assert!(
        log.contains("pr create --base feature-base --head feature-branch"),
        "bootstrap PR should target feature-base (parent), not main"
    );
}

#[test]
fn new_fails_when_parent_discovery_errors() {
    let (_temp, mut cmd) = stck_cmd_with_stubbed_tools();
    cmd.env("STCK_TEST_PR_LIST_FAIL", "1");
    cmd.args(["new", "feature-next"]);

    cmd.assert().code(1).stderr(predicate::str::contains(
        "error: could not auto-detect stack parent for feature-branch: failed to list open pull requests from GitHub; stderr: failed to list pull requests; retry or pass `--base <branch>` explicitly",
    ));
}

#[test]
fn new_from_default_branch_skips_default_branch_bootstrap() {
    let (temp, mut cmd) = stck_cmd_with_stubbed_tools();
    let log_path = log_path(&temp, "stck-new-from-default.log");
    cmd.env("STCK_TEST_LOG", log_path.as_os_str());
    cmd.env("STCK_TEST_CURRENT_BRANCH", "main");
    cmd.env("STCK_TEST_NEW_BRANCH_HAS_COMMITS", "1");
    cmd.args(["new", "feature-next"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("$ git checkout -b feature-next"))
        .stdout(predicate::str::contains(
            "$ git push -u origin feature-next",
        ))
        .stdout(predicate::str::contains(
            "$ gh pr create --base main --head feature-next --title feature-next --body \"\"",
        ))
        .stdout(predicate::str::contains(
            "Created branch feature-next and opened a stacked PR targeting main.",
        ));

    let log = fs::read_to_string(&log_path).expect("new log should exist");
    assert!(!log.contains("push -u origin main"));
    assert!(!log.contains("pr create --base main --head main"));
}

#[test]
fn new_fails_when_new_branch_exists_locally() {
    let (temp, mut cmd) = stck_cmd_with_stubbed_tools();
    let log_path = log_path(&temp, "stck-new-local-exists.log");
    cmd.env("STCK_TEST_LOG", log_path.as_os_str());
    cmd.env("STCK_TEST_LOCAL_BRANCH_EXISTS", "feature-next");
    cmd.args(["new", "feature-next"]);

    cmd.assert().code(1).stderr(predicate::str::contains(
        "error: branch feature-next already exists locally; choose a different name",
    ));

    let log = fs::read_to_string(&log_path).unwrap_or_default();
    assert!(
        log.is_empty(),
        "new should fail before running side-effecting commands"
    );
}

#[test]
fn new_fails_when_new_branch_exists_on_origin() {
    let (temp, mut cmd) = stck_cmd_with_stubbed_tools();
    let log_path = log_path(&temp, "stck-new-remote-exists.log");
    cmd.env("STCK_TEST_LOG", log_path.as_os_str());
    cmd.env("STCK_TEST_REMOTE_BRANCH_EXISTS", "feature-next");
    cmd.args(["new", "feature-next"]);

    cmd.assert().code(1).stderr(predicate::str::contains(
        "error: branch feature-next already exists on origin; choose a different name",
    ));

    let log = fs::read_to_string(&log_path).unwrap_or_default();
    assert!(
        log.is_empty(),
        "new should fail before running side-effecting commands"
    );
}

#[test]
fn new_rejects_invalid_branch_name() {
    let (_temp, mut cmd) = stck_cmd_with_stubbed_tools();
    cmd.args(["new", "feature branch"]);

    cmd.assert().code(1).stderr(predicate::str::contains(
        "error: `feature branch` is not a valid branch name",
    ));
}

#[test]
fn new_auto_pushes_when_upstream_exists_but_branch_needs_push() {
    let (temp, mut cmd) = stck_cmd_with_stubbed_tools();
    let log_path = log_path(&temp, "stck-new-auto-push.log");
    cmd.env("STCK_TEST_LOG", log_path.as_os_str());
    cmd.env("STCK_TEST_HAS_UPSTREAM", "1");
    cmd.env("STCK_TEST_NEEDS_PUSH_BRANCH", "feature-branch");
    cmd.env("STCK_TEST_NEW_BRANCH_HAS_COMMITS", "1");
    cmd.args(["new", "feature-next"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("$ git push origin feature-branch"));

    let log = fs::read_to_string(&log_path).expect("new log should exist");
    assert!(
        log.contains("push origin feature-branch"),
        "new should auto-push current branch when it has upstream but needs push"
    );
    assert!(
        !log.contains("push -u origin feature-branch"),
        "new should use regular push, not push -u, when upstream already exists"
    );
}

#[test]
fn new_skips_push_when_upstream_exists_and_branch_is_up_to_date() {
    let (temp, mut cmd) = stck_cmd_with_stubbed_tools();
    let log_path = log_path(&temp, "stck-new-no-push.log");
    cmd.env("STCK_TEST_LOG", log_path.as_os_str());
    cmd.env("STCK_TEST_HAS_UPSTREAM", "1");
    cmd.env("STCK_TEST_HAS_CURRENT_PR", "1");
    cmd.env("STCK_TEST_NEW_BRANCH_HAS_COMMITS", "1");
    cmd.args(["new", "feature-next"]);

    cmd.assert().success();

    let log = fs::read_to_string(&log_path).expect("new log should exist");
    assert!(
        !log.contains("push origin feature-branch"),
        "new should not push when branch is already up to date with remote"
    );
    assert!(
        !log.contains("push -u origin feature-branch"),
        "new should not push -u when upstream already exists"
    );
}

#[test]
fn new_fails_when_pr_presence_check_errors() {
    let (temp, mut cmd) = stck_cmd_with_stubbed_tools();
    let log_path = log_path(&temp, "stck-new-pr-view-error.log");
    cmd.env("STCK_TEST_LOG", log_path.as_os_str());
    cmd.env("STCK_TEST_HAS_UPSTREAM", "1");
    cmd.env("STCK_TEST_PR_VIEW_ERROR", "1");
    cmd.args(["new", "feature-next"]);

    cmd.assert().code(1).stderr(predicate::str::contains(
        "error: failed to check PR for branch feature-branch; ensure `gh auth status` succeeds and retry",
    ));

    let log = fs::read_to_string(&log_path).unwrap_or_default();
    assert!(
        !log.contains("pr create"),
        "new should not create PRs when PR presence check fails"
    );
}
