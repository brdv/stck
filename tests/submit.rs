mod harness;

use harness::{log_path, stck_cmd_with_stubbed_tools};
use predicates::prelude::*;
use std::fs;

#[test]
fn submit_creates_pr_with_base_override() {
    let (temp, mut cmd) = stck_cmd_with_stubbed_tools();
    let log_path = log_path(&temp, "stck-submit-base-override.log");
    cmd.env("STCK_TEST_LOG", log_path.as_os_str());
    cmd.args(["submit", "--base", "feature-base"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains(
            "$ gh pr create --base feature-base --head feature-branch --title feature-branch --body \"\"",
        ))
        .stdout(predicate::str::contains(
            "Created PR for feature-branch targeting feature-base.",
        ));

    let log = fs::read_to_string(&log_path).expect("submit log should exist");
    assert!(log.contains("pr create --base feature-base --head feature-branch"));
}

#[test]
fn submit_defaults_base_to_default_branch() {
    let (temp, mut cmd) = stck_cmd_with_stubbed_tools();
    let log_path = log_path(&temp, "stck-submit-default-base.log");
    cmd.env("STCK_TEST_LOG", log_path.as_os_str());
    cmd.arg("submit");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains(
            "No --base provided. Defaulting PR base to main.",
        ))
        .stdout(predicate::str::contains(
            "$ gh pr create --base main --head feature-branch --title feature-branch --body \"\"",
        ));
}

#[test]
fn submit_falls_back_to_default_when_no_parent_pr() {
    let (_temp, mut cmd) = stck_cmd_with_stubbed_tools();
    cmd.env("STCK_TEST_OPEN_PRS_JSON", "[]");
    cmd.arg("submit");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains(
            "No --base provided. Defaulting PR base to main.",
        ))
        .stdout(predicate::str::contains(
            "$ gh pr create --base main --head feature-branch",
        ));
}

#[test]
fn submit_fails_when_parent_discovery_errors() {
    let (_temp, mut cmd) = stck_cmd_with_stubbed_tools();
    cmd.env("STCK_TEST_PR_LIST_FAIL", "1");
    cmd.arg("submit");

    cmd.assert().code(1).stderr(predicate::str::contains(
        "error: could not auto-detect stack parent for feature-branch: failed to list open pull requests from GitHub; stderr: failed to list pull requests; retry or pass `--base <branch>` explicitly",
    ));
}

#[test]
fn submit_explicit_base_overrides_parent_discovery() {
    let (temp, mut cmd) = stck_cmd_with_stubbed_tools();
    let log_path = log_path(&temp, "stck-submit-explicit-override.log");
    cmd.env("STCK_TEST_LOG", log_path.as_os_str());
    cmd.env(
        "STCK_TEST_OPEN_PRS_JSON",
        r#"[{"number":100,"headRefName":"feature-base","baseRefName":"main","state":"OPEN","mergedAt":null}]"#,
    );
    cmd.args(["submit", "--base", "main"]);

    cmd.assert().success().stdout(predicate::str::contains(
        "$ gh pr create --base main --head feature-branch",
    ));

    let log = fs::read_to_string(&log_path).expect("submit log should exist");
    assert!(log.contains("pr create --base main --head feature-branch"));
}

#[test]
fn submit_discovers_parent_base_for_stacked_branch() {
    let (temp, mut cmd) = stck_cmd_with_stubbed_tools();
    let log_path = log_path(&temp, "stck-submit-stacked.log");
    cmd.env("STCK_TEST_LOG", log_path.as_os_str());
    cmd.env(
        "STCK_TEST_OPEN_PRS_JSON",
        r#"[{"number":100,"headRefName":"feature-base","baseRefName":"main","state":"OPEN","mergedAt":null}]"#,
    );
    cmd.arg("submit");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains(
            "No --base provided. Detected stack parent: feature-base.",
        ))
        .stdout(predicate::str::contains(
            "$ gh pr create --base feature-base --head feature-branch",
        ));

    let log = fs::read_to_string(&log_path).expect("submit log should exist");
    assert!(log.contains("pr create --base feature-base --head feature-branch"));
}

#[test]
fn submit_noops_when_pr_exists() {
    let (_temp, mut cmd) = stck_cmd_with_stubbed_tools();
    cmd.env("STCK_TEST_HAS_CURRENT_PR", "1");
    cmd.arg("submit");

    cmd.assert().success().stdout(predicate::str::contains(
        "Branch feature-branch already has an open PR.",
    ));
}

#[test]
fn submit_rejects_default_branch() {
    let (_temp, mut cmd) = stck_cmd_with_stubbed_tools();
    cmd.env("STCK_TEST_CURRENT_BRANCH", "main");
    cmd.arg("submit");

    cmd.assert().code(1).stderr(predicate::str::contains(
        "error: cannot submit PR for default branch main; checkout a feature branch and retry",
    ));
}
