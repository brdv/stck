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

if [[ "${1:-}" == "fetch" && "${2:-}" == "origin" ]]; then
  if [[ "${STCK_TEST_FETCH_FAIL:-0}" == "1" ]]; then
    exit 1
  fi
  exit 0
fi

if [[ "${1:-}" == "rev-parse" && "${2:-}" == "--verify" ]]; then
  ref="${3:-}"

  if [[ "${ref}" == refs/heads/* ]]; then
    branch="${ref#refs/heads/}"
    case "${branch}" in
      feature-base) echo "1111111111111111111111111111111111111111" ;;
      feature-branch) echo "2222222222222222222222222222222222222222" ;;
      feature-child) echo "3333333333333333333333333333333333333333" ;;
      *) echo "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" ;;
    esac
    exit 0
  fi

  if [[ "${ref}" == refs/remotes/origin/* ]]; then
    branch="${ref#refs/remotes/origin/}"
    if [[ "${STCK_TEST_MISSING_REMOTE_BRANCH:-}" == "${branch}" ]]; then
      exit 1
    fi
    if [[ "${STCK_TEST_NEEDS_PUSH_BRANCH:-}" == "${branch}" ]]; then
      echo "ffffffffffffffffffffffffffffffffffffffff"
      exit 0
    fi
    case "${branch}" in
      feature-base) echo "1111111111111111111111111111111111111111" ;;
      feature-branch) echo "2222222222222222222222222222222222222222" ;;
      feature-child) echo "3333333333333333333333333333333333333333" ;;
      *) echo "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" ;;
    esac
    exit 0
  fi
fi

if [[ "${1:-}" == "rev-parse" && "${2:-}" == "--git-dir" ]]; then
  if [[ -n "${STCK_TEST_GIT_DIR:-}" ]]; then
    echo "${STCK_TEST_GIT_DIR}"
  else
    echo ".git"
  fi
  exit 0
fi

if [[ "${1:-}" == "rebase" && "${2:-}" == "--onto" ]]; then
  if [[ -n "${STCK_TEST_LOG:-}" ]]; then
    echo "$*" >> "${STCK_TEST_LOG}"
  fi
  if [[ -n "${STCK_TEST_REBASE_FAIL_ONCE_FILE:-}" && ! -f "${STCK_TEST_REBASE_FAIL_ONCE_FILE}" ]]; then
    mkdir -p "$(dirname "${STCK_TEST_REBASE_FAIL_ONCE_FILE}")"
    touch "${STCK_TEST_REBASE_FAIL_ONCE_FILE}"
    exit 1
  fi
  if [[ "${STCK_TEST_REBASE_FAIL:-0}" == "1" ]]; then
    exit 1
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

if [[ "${1:-}" == "pr" && "${2:-}" == "list" ]]; then
  if [[ "${STCK_TEST_PR_LIST_FAIL:-0}" == "1" ]]; then
    echo "failed to list pull requests" >&2
    exit 1
  fi

  if [[ "${STCK_TEST_NON_LINEAR:-0}" == "1" ]]; then
    echo '[{"number":100,"headRefName":"feature-base","baseRefName":"main","state":"MERGED","mergedAt":"2026-01-01T00:00:00Z"},{"number":101,"headRefName":"feature-branch","baseRefName":"feature-base","state":"OPEN","mergedAt":null},{"number":102,"headRefName":"feature-child-a","baseRefName":"feature-branch","state":"OPEN","mergedAt":null},{"number":103,"headRefName":"feature-child-b","baseRefName":"feature-branch","state":"OPEN","mergedAt":null}]'
    exit 0
  fi

  if [[ "${STCK_TEST_MISSING_CURRENT_PR:-0}" == "1" ]]; then
    echo '[{"number":100,"headRefName":"feature-base","baseRefName":"main","state":"MERGED","mergedAt":"2026-01-01T00:00:00Z"}]'
    exit 0
  fi

  echo '[{"number":100,"headRefName":"feature-base","baseRefName":"main","state":"MERGED","mergedAt":"2026-01-01T00:00:00Z"},{"number":101,"headRefName":"feature-branch","baseRefName":"feature-base","state":"OPEN","mergedAt":null},{"number":102,"headRefName":"feature-child","baseRefName":"feature-branch","state":"OPEN","mergedAt":null}]'
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
    let git_dir = temp.path().join("git-dir");
    fs::create_dir_all(&git_dir).expect("git dir should be created");

    let mut cmd = stck_cmd();
    cmd.env("PATH", full_path);
    cmd.env("STCK_TEST_GIT_DIR", git_dir.as_os_str());
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
    for command in ["new", "push"] {
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
fn sync_executes_rebase_plan_and_prints_success_message() {
    let (_temp, mut cmd) = stck_cmd_with_stubbed_tools();
    let log_path = std::env::temp_dir().join("stck-sync-rebase.log");
    let _ = fs::remove_file(&log_path);
    cmd.env("STCK_TEST_LOG", log_path.as_os_str());
    cmd.arg("sync");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains(
            "$ git rebase --onto main 1111111111111111111111111111111111111111 feature-branch",
        ))
        .stdout(predicate::str::contains(
            "$ git rebase --onto feature-branch 2222222222222222222222222222222222222222 feature-child",
        ))
        .stdout(predicate::str::contains(
            "Sync succeeded locally. Run `stck push` to update remotes + PR bases.",
        ));

    let log = fs::read_to_string(&log_path).expect("rebase log should exist");
    assert!(
        log.contains("rebase --onto main 1111111111111111111111111111111111111111 feature-branch")
    );
    assert!(log.contains(
        "rebase --onto feature-branch 2222222222222222222222222222222222222222 feature-child"
    ));
}

#[test]
fn sync_surfaces_rebase_failure_with_guidance() {
    let (_temp, mut cmd) = stck_cmd_with_stubbed_tools();
    cmd.env("STCK_TEST_REBASE_FAIL", "1");
    cmd.arg("sync");

    cmd.assert().code(1).stderr(predicate::str::contains(
        "error: rebase failed for branch feature-branch; resolve conflicts, run `git rebase --continue` or `git rebase --abort`, then rerun `stck sync`",
    ));
}

#[test]
fn sync_continue_requires_existing_state() {
    let (_temp, mut cmd) = stck_cmd_with_stubbed_tools();
    cmd.args(["sync", "--continue"]);

    cmd.assert().code(1).stderr(predicate::str::contains(
        "error: no sync state found; run `stck sync` first",
    ));
}

#[test]
fn sync_continue_resumes_after_previous_failure() {
    let (temp, mut first) = stck_cmd_with_stubbed_tools();
    let log_path = temp.path().join("rebase.log");
    let fail_once_path = temp.path().join("fail-once.marker");

    first.env("STCK_TEST_LOG", log_path.as_os_str());
    first.env(
        "STCK_TEST_REBASE_FAIL_ONCE_FILE",
        fail_once_path.as_os_str(),
    );
    first.arg("sync");
    first.assert().code(1).stderr(predicate::str::contains(
        "error: rebase failed for branch feature-branch; resolve conflicts, run `git rebase --continue` or `git rebase --abort`, then rerun `stck sync`",
    ));

    let state_path = temp
        .path()
        .join("git-dir")
        .join("stck")
        .join("last-plan.json");
    assert!(
        state_path.exists(),
        "sync state should persist after failure"
    );

    let mut resume = stck_cmd();
    let path = std::env::var("PATH").expect("PATH should be set");
    let full_path = format!("{}:{}", temp.path().join("bin").display(), path);
    resume.env("PATH", full_path);
    resume.env("STCK_TEST_GIT_DIR", temp.path().join("git-dir").as_os_str());
    resume.env("STCK_TEST_LOG", log_path.as_os_str());
    resume.args(["sync", "--continue"]);

    resume
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "$ git rebase --onto feature-branch 2222222222222222222222222222222222222222 feature-child",
        ))
        .stdout(predicate::str::contains(
            "Sync succeeded locally. Run `stck push` to update remotes + PR bases.",
        ));

    let log = fs::read_to_string(&log_path).expect("rebase log should exist");
    let first_step = "rebase --onto main 1111111111111111111111111111111111111111 feature-branch";
    let second_step =
        "rebase --onto feature-branch 2222222222222222222222222222222222222222 feature-child";
    assert_eq!(log.matches(first_step).count(), 1);
    assert_eq!(log.matches(second_step).count(), 1);
    assert!(
        !state_path.exists(),
        "sync state should be removed after success"
    );
}

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
            "feature-base PR #100 [MERGED] base=main head=feature-base flags=none",
        ))
        .stdout(predicate::str::contains(
            "feature-branch PR #101 [OPEN] base=feature-base head=feature-branch flags=needs_sync",
        ))
        .stdout(predicate::str::contains(
            "feature-child PR #102 [OPEN] base=feature-branch head=feature-child flags=none",
        ))
        .stdout(predicate::str::contains(
            "Summary: needs_sync=1 needs_push=0 base_mismatch=0",
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
            "feature-child PR #102 [OPEN] base=feature-branch head=feature-child flags=needs_push",
        ))
        .stdout(predicate::str::contains(
            "Summary: needs_sync=1 needs_push=1 base_mismatch=0",
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
