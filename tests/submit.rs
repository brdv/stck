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
            "$ gh pr create --base feature-base --head feature-branch --title feature-branch --body \"<stack context>\"",
        ))
        .stdout(predicate::str::contains(
            "Created PR for feature-branch targeting feature-base.",
        ));

    let log = fs::read_to_string(&log_path).expect("submit log should exist");
    assert!(log.contains("pr create --base feature-base --head feature-branch"));
    assert!(log.contains(
        "pr body --head feature-branch\nThis pull request is part of a stack.\n\n- **Position:** Child\n- **Base:** `feature-base`"
    ));
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
            "$ gh pr create --base main --head feature-branch --title feature-branch --body \"<stack context>\"",
        ));

    let log = fs::read_to_string(&log_path).expect("submit log should exist");
    assert!(log.contains(
        "pr body --head feature-branch\nThis pull request is part of a stack.\n\n- **Position:** Root\n- **Base:** `main`"
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
fn submit_fails_when_targeted_parent_query_errors() {
    let (_temp, mut cmd) = stck_cmd_with_stubbed_tools();
    cmd.env("STCK_TEST_PR_LIST_FAIL", "1");
    cmd.arg("submit");

    cmd.assert().code(1).stderr(predicate::str::contains(
        "error: could not auto-detect stack parent for feature-branch: failed to query open pull request for branch feature-base; stderr: failed to list pull requests; retry or pass `--base <branch>` explicitly",
    ));
}

#[test]
fn submit_fails_when_origin_branches_cannot_be_listed() {
    let (_temp, mut cmd) = stck_cmd_with_stubbed_tools();
    cmd.env("STCK_TEST_LIST_ORIGIN_BRANCHES_FAIL", "1");
    cmd.arg("submit");

    cmd.assert().code(1).stderr(predicate::str::contains(
        "error: could not auto-detect stack parent for feature-branch: failed to list branches from `origin`; stderr: failed to enumerate remote refs; retry or pass `--base <branch>` explicitly",
    ));
}

#[test]
fn submit_fails_when_parent_candidate_refs_are_missing() {
    let (temp, mut cmd) = stck_cmd_with_stubbed_tools();
    let log_path = log_path(&temp, "stck-submit-missing-parent-refs.log");
    cmd.env("STCK_TEST_LOG", log_path.as_os_str());
    cmd.env("STCK_TEST_MISSING_LOCAL_BRANCH_REF", "feature-base");
    cmd.env("STCK_TEST_MISSING_REMOTE_BRANCH", "feature-base");
    cmd.env(
        "STCK_TEST_OPEN_PRS_JSON",
        r#"[{"headRefName":"feature-base","isCrossRepository":false}]"#,
    );
    cmd.arg("submit");

    cmd.assert().code(1).stderr(predicate::str::contains(
        "error: could not auto-detect stack parent for feature-branch: could not resolve branch `feature-base` from `origin` or local refs; retry or pass `--base <branch>` explicitly",
    ));

    let log = fs::read_to_string(&log_path).expect("submit log should exist");
    assert!(
        !log.contains("pr create"),
        "submit must not fall back to main when a parent candidate cannot be resolved"
    );
}

#[test]
fn submit_fails_when_parent_ancestry_check_errors() {
    let (temp, mut cmd) = stck_cmd_with_stubbed_tools();
    let log_path = log_path(&temp, "stck-submit-ancestry-error.log");
    cmd.env("STCK_TEST_LOG", log_path.as_os_str());
    cmd.env(
        "STCK_TEST_ANCESTRY_ERROR_PAIRS",
        "feature-base:feature-branch",
    );
    cmd.env(
        "STCK_TEST_OPEN_PRS_JSON",
        r#"[{"headRefName":"feature-base","isCrossRepository":false}]"#,
    );
    cmd.arg("submit");

    cmd.assert().code(1).stderr(predicate::str::contains(
        "error: could not auto-detect stack parent for feature-branch: failed to check ancestry between `refs/remotes/origin/feature-base` and `refs/heads/feature-branch`; retry or pass `--base <branch>` explicitly",
    ));

    let log = fs::read_to_string(&log_path).expect("submit log should exist");
    assert!(
        !log.contains("pr create"),
        "submit must not fall back to main when ancestry cannot be verified"
    );
}

#[test]
fn submit_ignores_cross_repository_parent_candidates() {
    let (_temp, mut cmd) = stck_cmd_with_stubbed_tools();
    cmd.env(
        "STCK_TEST_OPEN_PRS_JSON",
        r#"[{"headRefName":"fork-feature","isCrossRepository":true}]"#,
    );
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
    assert!(log.contains(
        "pr list --head feature-base --state open --limit 1 --json headRefName,isCrossRepository"
    ));
    assert!(
        !log.contains("pr list --state open --limit 100"),
        "submit must use targeted parent queries"
    );
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
fn submit_auto_pushes_when_upstream_exists_but_branch_needs_push() {
    let (temp, mut cmd) = stck_cmd_with_stubbed_tools();
    let log_path = log_path(&temp, "stck-submit-auto-push.log");
    cmd.env("STCK_TEST_LOG", log_path.as_os_str());
    cmd.env("STCK_TEST_HAS_UPSTREAM", "1");
    cmd.env("STCK_TEST_NEEDS_PUSH_BRANCH", "feature-branch");
    cmd.args(["submit", "--base", "main"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("$ git push origin feature-branch"));

    let log = fs::read_to_string(&log_path).expect("submit log should exist");
    assert!(
        log.contains("push origin feature-branch"),
        "submit should auto-push current branch when it has upstream but needs push"
    );
    assert!(
        !log.contains("push -u origin feature-branch"),
        "submit should use regular push, not push -u, when upstream already exists"
    );
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
