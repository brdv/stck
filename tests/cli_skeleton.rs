use assert_cmd::cargo::cargo_bin_cmd;
use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use tempfile::TempDir;

fn stck_cmd() -> Command {
    cargo_bin_cmd!("stck")
}

fn write_stub(path: &Path, body: &str) {
    fs::write(path, body).expect("stub script should be written");
    let mut permissions = fs::metadata(path)
        .expect("stub metadata should be readable")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).expect("stub script should be executable");
}

fn setup_stubbed_tools() -> TempDir {
    let temp = TempDir::new().expect("tempdir should be created");
    let bin_dir = temp.path().join("bin");
    fs::create_dir_all(&bin_dir).expect("bin dir should be created");

    write_stub(
        &bin_dir.join("git"),
        r#"#!/usr/bin/env bash
set -euo pipefail

if [[ "${1:-}" == "--version" ]]; then
  echo "git version 2.0.0"
  exit 0
fi

if [[ "${1:-}" == "remote" && "${2:-}" == "get-url" && "${3:-}" == "origin" ]]; then
  if [[ "${STCK_TEST_ORIGIN_MISSING:-0}" == "1" ]]; then
    exit 2
  fi
  echo "git@github.com:example/stck.git"
  exit 0
fi

if [[ "${1:-}" == "symbolic-ref" && "${2:-}" == "--quiet" && "${3:-}" == "--short" && "${4:-}" == "HEAD" ]]; then
  if [[ "${STCK_TEST_DETACHED_HEAD:-0}" == "1" ]]; then
    exit 1
  fi
  echo "feature-branch"
  exit 0
fi

if [[ "${1:-}" == "status" && "${2:-}" == "--porcelain" ]]; then
  if [[ "${STCK_TEST_DIRTY_TREE:-0}" == "1" ]]; then
    echo " M src/main.rs"
  fi
  exit 0
fi

exit 0
"#,
    );

    write_stub(
        &bin_dir.join("gh"),
        r#"#!/usr/bin/env bash
set -euo pipefail

if [[ "${1:-}" == "--version" ]]; then
  echo "gh version 2.0.0"
  exit 0
fi

if [[ "${1:-}" == "auth" && "${2:-}" == "status" ]]; then
  if [[ "${STCK_TEST_GH_AUTH_FAIL:-0}" == "1" ]]; then
    exit 1
  fi
  exit 0
fi

if [[ "${1:-}" == "repo" && "${2:-}" == "view" ]]; then
  if [[ "${STCK_TEST_DEFAULT_BRANCH_FAIL:-0}" == "1" ]]; then
    exit 1
  fi
  echo "main"
  exit 0
fi

if [[ "${1:-}" == "pr" && "${2:-}" == "view" ]]; then
  if [[ "${STCK_TEST_PR_VIEW_FAIL:-0}" == "1" ]]; then
    echo "no pull requests found for branch" >&2
    exit 1
  fi
  echo '{"number":101,"headRefName":"feature-branch","baseRefName":"main","state":"OPEN","mergedAt":null}'
  exit 0
fi

exit 0
"#,
    );

    temp
}

fn stck_cmd_with_stubbed_tools() -> (TempDir, Command) {
    let temp = setup_stubbed_tools();
    let path = std::env::var("PATH").expect("PATH should be set");
    let full_path = format!("{}:{}", temp.path().join("bin").display(), path);

    let mut cmd = stck_cmd();
    cmd.env("PATH", full_path);
    (temp, cmd)
}

#[test]
fn help_lists_all_commands() {
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
fn commands_show_placeholder_when_preflight_passes() {
    for command in ["new", "sync", "push"] {
        let (_temp, mut cmd) = stck_cmd_with_stubbed_tools();
        if command == "new" {
            cmd.args([command, "feature-x"]);
        } else {
            cmd.arg(command);
        }

        cmd.assert()
            .code(1)
            .stderr(predicate::str::contains(format!(
                "error: `stck {command}` is not implemented yet"
            )));
    }
}

#[test]
fn status_prints_single_pr_details() {
    let (_temp, mut cmd) = stck_cmd_with_stubbed_tools();
    cmd.arg("status");

    cmd.assert().success().stdout(predicate::str::contains(
        "PR #101 state=OPEN base=main head=feature-branch",
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
fn shows_dirty_tree_remediation() {
    let (_temp, mut cmd) = stck_cmd_with_stubbed_tools();
    cmd.env("STCK_TEST_DIRTY_TREE", "1");
    cmd.arg("sync");

    cmd.assert().code(1).stderr(predicate::str::contains(
        "error: working tree is not clean; commit, stash, or discard changes before running stck",
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
fn shows_detached_head_remediation() {
    let (_temp, mut cmd) = stck_cmd_with_stubbed_tools();
    cmd.env("STCK_TEST_DETACHED_HEAD", "1");
    cmd.arg("push");

    cmd.assert().code(1).stderr(predicate::str::contains(
        "error: not on a branch (detached HEAD); checkout a branch and retry",
    ));
}

#[test]
fn status_shows_missing_pr_remediation() {
    let (_temp, mut cmd) = stck_cmd_with_stubbed_tools();
    cmd.env("STCK_TEST_PR_VIEW_FAIL", "1");
    cmd.arg("status");

    cmd.assert().code(1).stderr(predicate::str::contains(
        "error: no PR found for branch feature-branch; create a PR first",
    ));
}
