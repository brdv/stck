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
  if [[ -n "${STCK_TEST_CURRENT_BRANCH:-}" ]]; then
    echo "${STCK_TEST_CURRENT_BRANCH}"
  else
    echo "feature-branch"
  fi
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
    if [[ "${STCK_TEST_MISSING_LOCAL_BRANCH_REF:-}" == "${branch}" ]]; then
      exit 1
    fi
    case "${branch}" in
      feature-base) echo "1111111111111111111111111111111111111111" ;;
      feature-branch)
        if [[ -n "${STCK_TEST_FEATURE_BRANCH_HEAD:-}" ]]; then
          echo "${STCK_TEST_FEATURE_BRANCH_HEAD}"
        else
          echo "2222222222222222222222222222222222222222"
        fi
        ;;
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
    if [[ ",${STCK_TEST_NEEDS_PUSH_BRANCHES:-}," == *",${branch},"* ]]; then
      echo "ffffffffffffffffffffffffffffffffffffffff"
      exit 0
    fi
    case "${branch}" in
      feature-base)
        if [[ -n "${STCK_TEST_REMOTE_FEATURE_BASE_SHA:-}" ]]; then
          echo "${STCK_TEST_REMOTE_FEATURE_BASE_SHA}"
        else
          echo "1111111111111111111111111111111111111111"
        fi
        ;;
      feature-branch) echo "2222222222222222222222222222222222222222" ;;
      feature-child) echo "3333333333333333333333333333333333333333" ;;
      *) echo "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" ;;
    esac
    exit 0
  fi
fi

if [[ "${1:-}" == "rev-parse" && "${2:-}" == "--abbrev-ref" && "${3:-}" == "--symbolic-full-name" ]]; then
  upstream_ref="${4:-}"
  if [[ "${upstream_ref}" == "feature-branch@{upstream}" && "${STCK_TEST_HAS_UPSTREAM:-0}" == "1" ]]; then
    echo "origin/feature-branch"
    exit 0
  fi
  exit 1
fi

if [[ "${1:-}" == "rev-parse" && "${2:-}" == "--git-dir" ]]; then
  if [[ -n "${STCK_TEST_GIT_DIR:-}" ]]; then
    echo "${STCK_TEST_GIT_DIR}"
  else
    echo ".git"
  fi
  exit 0
fi

if [[ "${1:-}" == "show-ref" && "${2:-}" == "--verify" && "${3:-}" == "--quiet" ]]; then
  ref="${4:-}"
  if [[ "${ref}" == refs/heads/* ]]; then
    branch="${ref#refs/heads/}"
    if [[ "${STCK_TEST_LOCAL_BRANCH_EXISTS:-}" == "${branch}" ]]; then
      exit 0
    fi
    # For resolve_base_ref: known branches exist unless explicitly missing
    if [[ "${STCK_TEST_MISSING_LOCAL_BRANCH_REF:-}" == "${branch}" ]]; then
      exit 1
    fi
    case "${branch}" in
      feature-base|feature-branch|feature-child|main) exit 0 ;;
    esac
    exit 1
  fi
  if [[ "${ref}" == refs/remotes/origin/* ]]; then
    branch="${ref#refs/remotes/origin/}"
    if [[ "${STCK_TEST_REMOTE_BRANCH_EXISTS:-}" == "${branch}" ]]; then
      exit 0
    fi
    if [[ "${STCK_TEST_MISSING_REMOTE_BRANCH:-}" == "${branch}" ]]; then
      exit 1
    fi
    case "${branch}" in
      feature-base|feature-branch|feature-child|main) exit 0 ;;
    esac
    exit 1
  fi
  exit 1
fi

if [[ "${1:-}" == "rev-list" && "${2:-}" == "--count" ]]; then
  range="${3:-}"
  if [[ "${range}" == *"..refs/heads/feature-next" || "${range}" == *"..refs/heads/feature-x" ]]; then
    if [[ "${STCK_TEST_NEW_BRANCH_HAS_COMMITS:-0}" == "1" ]]; then
      echo "1"
    else
      echo "0"
    fi
    exit 0
  fi
  echo "1"
  exit 0
fi

if [[ "${1:-}" == "merge-base" && "${2:-}" != "--is-ancestor" ]]; then
  ref_a="${2:-}"
  ref_b="${3:-}"
  # Return the SHA of ref_a as a simple merge-base approximation.
  # For test purposes this simulates finding the fork point.
  if [[ "${STCK_TEST_MERGE_BASE_FAIL:-0}" == "1" ]]; then
    exit 1
  fi
  # Resolve the ref to its known SHA using the same logic as rev-parse
  case "${ref_a}" in
    refs/heads/feature-base) echo "1111111111111111111111111111111111111111" ;;
    refs/remotes/origin/feature-base)
      if [[ -n "${STCK_TEST_REMOTE_FEATURE_BASE_SHA:-}" ]]; then
        echo "${STCK_TEST_REMOTE_FEATURE_BASE_SHA}"
      else
        echo "1111111111111111111111111111111111111111"
      fi
      ;;
    refs/heads/feature-branch)
      if [[ -n "${STCK_TEST_FEATURE_BRANCH_HEAD:-}" ]]; then
        echo "${STCK_TEST_FEATURE_BRANCH_HEAD}"
      else
        echo "2222222222222222222222222222222222222222"
      fi
      ;;
    refs/remotes/origin/feature-branch)
      echo "2222222222222222222222222222222222222222"
      ;;
    refs/heads/main|refs/remotes/origin/main)
      echo "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
      ;;
    *) echo "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" ;;
  esac
  exit 0
fi

if [[ "${1:-}" == "merge-base" && "${2:-}" == "--is-ancestor" ]]; then
  ancestor="${3:-}"
  descendant="${4:-}"
  if [[ "${STCK_TEST_DEFAULT_ADVANCED:-0}" == "1" && "${ancestor}" == "refs/remotes/origin/main" && "${descendant}" == "refs/heads/feature-base" ]]; then
    exit 1
  fi
  # Known parent-child relationships in the test stack:
  #   main -> feature-base -> feature-branch -> feature-child
  # Only return true for known ancestor relationships
  ancestor_branch="${ancestor#refs/heads/}"
  ancestor_branch="${ancestor_branch#refs/remotes/origin/}"
  descendant_branch="${descendant#refs/heads/}"
  descendant_branch="${descendant_branch#refs/remotes/origin/}"

  # Configurable ancestor pairs: STCK_TEST_ANCESTOR_PAIRS="branch_a:branch_b,..."
  if [[ -n "${STCK_TEST_ANCESTOR_PAIRS:-}" ]]; then
    IFS=',' read -ra pairs <<< "${STCK_TEST_ANCESTOR_PAIRS}"
    for pair in "${pairs[@]}"; do
      IFS=':' read -r pa pd <<< "${pair}"
      if [[ "${ancestor_branch}" == "${pa}" && "${descendant_branch}" == "${pd}" ]]; then
        exit 0
      fi
    done
  fi

  case "${ancestor_branch}:${descendant_branch}" in
    main:feature-base|main:feature-branch|main:feature-child) exit 0 ;;
    feature-base:feature-branch|feature-base:feature-child) exit 0 ;;
    feature-branch:feature-child) exit 0 ;;
    *) exit 1 ;;
  esac
fi

if [[ "${1:-}" == "rebase" && "${2:-}" == "--onto" ]]; then
  if [[ -n "${STCK_TEST_LOG:-}" ]]; then
    echo "$*" >> "${STCK_TEST_LOG}"
  fi
  if [[ "${STCK_TEST_REBASE_FAIL_STDERR:-0}" == "1" ]]; then
    echo "CONFLICT (content): Merge conflict in src/main.rs" >&2
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

if [[ "${1:-}" == "push" && "${2:-}" == "--force-with-lease" && "${3:-}" == "origin" ]]; then
  branch="${4:-}"
  if [[ -n "${STCK_TEST_LOG:-}" ]]; then
    echo "$*" >> "${STCK_TEST_LOG}"
  fi
  if [[ "${STCK_TEST_PUSH_FAIL_BRANCH:-}" == "${branch}" ]]; then
    exit 1
  fi
  exit 0
fi

if [[ "${1:-}" == "push" && "${2:-}" == "-u" && "${3:-}" == "origin" ]]; then
  branch="${4:-}"
  if [[ -n "${STCK_TEST_LOG:-}" ]]; then
    echo "$*" >> "${STCK_TEST_LOG}"
  fi
  if [[ "${STCK_TEST_PUSH_U_FAIL_BRANCH:-}" == "${branch}" ]]; then
    exit 1
  fi
  exit 0
fi

if [[ "${1:-}" == "checkout" && "${2:-}" == "-b" ]]; then
  branch="${3:-}"
  if [[ -n "${STCK_TEST_LOG:-}" ]]; then
    echo "$*" >> "${STCK_TEST_LOG}"
  fi
  if [[ "${STCK_TEST_CHECKOUT_FAIL_BRANCH:-}" == "${branch}" ]]; then
    exit 1
  fi
  exit 0
fi

if [[ "${1:-}" == "checkout" ]]; then
  branch="${2:-}"
  if [[ -n "${STCK_TEST_LOG:-}" ]]; then
    echo "$*" >> "${STCK_TEST_LOG}"
  fi
  if [[ "${STCK_TEST_CHECKOUT_FAIL_BRANCH:-}" == "${branch}" ]]; then
    exit 1
  fi
  exit 0
fi

if [[ "${1:-}" == "check-ref-format" ]]; then
  shift
  while [[ "${1:-}" == --* ]]; do shift; done
  name="${1:-}"
  if [[ "${name}" == *" "* || "${name}" == *".."* || "${name}" == *"~"* || "${name}" == *"^"* || "${name}" == *":"* || "${name}" == *"\\"* ]]; then
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

  # Detect --state and --base flags
  pr_list_state="all"
  pr_list_base=""
  for ((i=1; i<=$#; i++)); do
    if [[ "${!i}" == "--state" ]]; then
      next=$((i+1))
      pr_list_state="${!next}"
    fi
    if [[ "${!i}" == "--base" ]]; then
      next=$((i+1))
      pr_list_base="${!next}"
    fi
  done

  # Targeted child discovery: gh pr list --base <branch>
  if [[ -n "${pr_list_base}" ]]; then
    if [[ "${STCK_TEST_NON_LINEAR:-0}" == "1" && "${pr_list_base}" == "feature-branch" ]]; then
      echo '[{"number":102,"headRefName":"feature-child-a","baseRefName":"feature-branch","state":"OPEN"},{"number":103,"headRefName":"feature-child-b","baseRefName":"feature-branch","state":"OPEN"}]'
      exit 0
    fi
    if [[ -n "${STCK_TEST_FEATURE_CHILD_BASE:-}" && "${pr_list_base}" == "feature-branch" ]]; then
      echo "[{\"number\":102,\"headRefName\":\"feature-child\",\"baseRefName\":\"${STCK_TEST_FEATURE_CHILD_BASE}\",\"state\":\"OPEN\"}]"
      exit 0
    fi
    if [[ "${STCK_TEST_SYNC_NOOP:-0}" == "1" ]]; then
      case "${pr_list_base}" in
        main) echo '[{"number":100,"headRefName":"feature-base","baseRefName":"main","state":"OPEN"}]' ;;
        feature-base) echo '[{"number":101,"headRefName":"feature-branch","baseRefName":"feature-base","state":"OPEN"}]' ;;
        feature-branch) echo '[{"number":102,"headRefName":"feature-child","baseRefName":"feature-branch","state":"OPEN"}]' ;;
        *) echo '[]' ;;
      esac
      exit 0
    fi
    # Default children for the default stack
    case "${pr_list_base}" in
      feature-base) echo '[{"number":101,"headRefName":"feature-branch","baseRefName":"feature-base","state":"OPEN"}]' ;;
      feature-branch) echo '[{"number":102,"headRefName":"feature-child","baseRefName":"feature-branch","state":"OPEN"}]' ;;
      *) echo '[]' ;;
    esac
    exit 0
  fi

  # Bulk list (used by list_open_prs for parent discovery)
  if [[ -n "${STCK_TEST_OPEN_PRS_JSON:-}" && "${pr_list_state}" == "open" ]]; then
    echo "${STCK_TEST_OPEN_PRS_JSON}"
    exit 0
  fi

  # Default bulk list for list_open_prs
  echo '[]'
  exit 0
fi

if [[ "${1:-}" == "pr" && "${2:-}" == "view" ]]; then
  branch="${3:-}"
  all_args="$*"

  if [[ "${STCK_TEST_PR_VIEW_ERROR:-0}" == "1" ]]; then
    echo "network unavailable" >&2
    exit 1
  fi

  # Stack discovery path (full field set including headRefName)
  if [[ "${all_args}" == *"headRefName"* ]]; then
    if [[ "${STCK_TEST_MISSING_CURRENT_PR:-0}" == "1" && "${branch}" == "feature-branch" ]]; then
      echo "no pull requests found for branch ${branch}" >&2
      exit 1
    fi

    if [[ -n "${STCK_TEST_FEATURE_BRANCH_BASE:-}" && "${branch}" == "feature-branch" ]]; then
      echo "{\"number\":101,\"headRefName\":\"feature-branch\",\"baseRefName\":\"${STCK_TEST_FEATURE_BRANCH_BASE}\",\"state\":\"OPEN\"}"
      exit 0
    fi

    if [[ -n "${STCK_TEST_FEATURE_CHILD_BASE:-}" && "${branch}" == "feature-child" ]]; then
      echo "{\"number\":102,\"headRefName\":\"feature-child\",\"baseRefName\":\"${STCK_TEST_FEATURE_CHILD_BASE}\",\"state\":\"OPEN\"}"
      exit 0
    fi

    if [[ "${STCK_TEST_NON_LINEAR:-0}" == "1" ]]; then
      case "${branch}" in
        feature-base) echo '{"number":100,"headRefName":"feature-base","baseRefName":"main","state":"MERGED"}' ;;
        feature-branch) echo '{"number":101,"headRefName":"feature-branch","baseRefName":"feature-base","state":"OPEN"}' ;;
        *) echo "no pull requests found for branch ${branch}" >&2; exit 1 ;;
      esac
      exit 0
    fi

    if [[ "${STCK_TEST_SYNC_NOOP:-0}" == "1" ]]; then
      case "${branch}" in
        feature-base) echo '{"number":100,"headRefName":"feature-base","baseRefName":"main","state":"OPEN"}' ;;
        feature-branch) echo '{"number":101,"headRefName":"feature-branch","baseRefName":"feature-base","state":"OPEN"}' ;;
        feature-child) echo '{"number":102,"headRefName":"feature-child","baseRefName":"feature-branch","state":"OPEN"}' ;;
        *) echo "no pull requests found for branch ${branch}" >&2; exit 1 ;;
      esac
      exit 0
    fi

    # Default: feature-base(merged) -> feature-branch(open) -> feature-child(open)
    case "${branch}" in
      feature-base) echo '{"number":100,"headRefName":"feature-base","baseRefName":"main","state":"MERGED"}' ;;
      feature-branch) echo '{"number":101,"headRefName":"feature-branch","baseRefName":"feature-base","state":"OPEN"}' ;;
      feature-child) echo '{"number":102,"headRefName":"feature-child","baseRefName":"feature-branch","state":"OPEN"}' ;;
      *) echo "no pull requests found for branch ${branch}" >&2; exit 1 ;;
    esac
    exit 0
  fi

  # Legacy: pr_exists_for_head check (--json number)
  if [[ "${STCK_TEST_HAS_CURRENT_PR:-0}" == "1" && "${branch}" == "feature-branch" ]]; then
    echo '{"number":101}'
    exit 0
  fi
  echo "no pull requests found for branch" >&2
  exit 1
fi

if [[ "${1:-}" == "pr" && "${2:-}" == "edit" ]]; then
  branch="${3:-}"
  if [[ -n "${STCK_TEST_LOG:-}" ]]; then
    echo "$*" >> "${STCK_TEST_LOG}"
  fi
  if [[ -n "${STCK_TEST_RETARGET_FAIL_ONCE_FILE:-}" && "${STCK_TEST_RETARGET_FAIL_ONCE_BRANCH:-}" == "${branch}" && ! -f "${STCK_TEST_RETARGET_FAIL_ONCE_FILE}" ]]; then
    mkdir -p "$(dirname "${STCK_TEST_RETARGET_FAIL_ONCE_FILE}")"
    touch "${STCK_TEST_RETARGET_FAIL_ONCE_FILE}"
    exit 1
  fi
  if [[ "${STCK_TEST_RETARGET_FAIL_BRANCH:-}" == "${branch}" ]]; then
    exit 1
  fi
  exit 0
fi

if [[ "${1:-}" == "pr" && "${2:-}" == "create" ]]; then
  base=""
  head=""
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --base) base="${2:-}"; shift 2 ;;
      --head) head="${2:-}"; shift 2 ;;
      --title) shift 2 ;;
      --body) shift 2 ;;
      *) shift ;;
    esac
  done
  if [[ -n "${STCK_TEST_LOG:-}" ]]; then
    echo "pr create --base ${base} --head ${head}" >> "${STCK_TEST_LOG}"
  fi
  if [[ "${STCK_TEST_PR_CREATE_FAIL_HEAD:-}" == "${head}" ]]; then
    exit 1
  fi
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
    let (_temp, mut cmd) = stck_cmd_with_stubbed_tools();
    let log_path = std::env::temp_dir().join("stck-new-bootstrap.log");
    let _ = fs::remove_file(&log_path);
    cmd.env("STCK_TEST_LOG", log_path.as_os_str());
    cmd.env("STCK_TEST_NEW_BRANCH_HAS_COMMITS", "1");
    cmd.arg("new");
    cmd.arg("feature-next");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains(
            "$ git push -u origin feature-branch",
        ))
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
    let (_temp, mut cmd) = stck_cmd_with_stubbed_tools();
    let log_path = std::env::temp_dir().join("stck-new-skip-bootstrap.log");
    let _ = fs::remove_file(&log_path);
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
    // When on feature-branch (child of feature-base which has an OPEN PR),
    // `stck new` should auto-create feature-branch's bootstrap PR targeting feature-base
    let (_temp, mut cmd) = stck_cmd_with_stubbed_tools();
    let log_path = std::env::temp_dir().join("stck-new-stacked-parent.log");
    let _ = fs::remove_file(&log_path);
    cmd.env("STCK_TEST_LOG", log_path.as_os_str());
    cmd.env("STCK_TEST_NEW_BRANCH_HAS_COMMITS", "1");
    // Provide open PRs where feature-base has an open PR that is ancestor of feature-branch
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
fn submit_creates_pr_with_base_override() {
    let (_temp, mut cmd) = stck_cmd_with_stubbed_tools();
    let log_path = std::env::temp_dir().join("stck-submit-base-override.log");
    let _ = fs::remove_file(&log_path);
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
    let (_temp, mut cmd) = stck_cmd_with_stubbed_tools();
    let log_path = std::env::temp_dir().join("stck-submit-default-base.log");
    let _ = fs::remove_file(&log_path);
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
    // When no open parent PR exists, submit should default to the default branch
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
    // --base should take precedence over parent discovery
    let (_temp, mut cmd) = stck_cmd_with_stubbed_tools();
    let log_path = std::env::temp_dir().join("stck-submit-explicit-override.log");
    let _ = fs::remove_file(&log_path);
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
    let (_temp, mut cmd) = stck_cmd_with_stubbed_tools();
    let log_path = std::env::temp_dir().join("stck-submit-stacked.log");
    let _ = fs::remove_file(&log_path);
    cmd.env("STCK_TEST_LOG", log_path.as_os_str());
    // feature-branch is on top of feature-base (which has an OPEN PR)
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

#[test]
fn new_from_default_branch_skips_default_branch_bootstrap() {
    let (_temp, mut cmd) = stck_cmd_with_stubbed_tools();
    let log_path = std::env::temp_dir().join("stck-new-from-default.log");
    let _ = fs::remove_file(&log_path);
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
    let (_temp, mut cmd) = stck_cmd_with_stubbed_tools();
    let log_path = std::env::temp_dir().join("stck-new-local-exists.log");
    let _ = fs::remove_file(&log_path);
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
    let (_temp, mut cmd) = stck_cmd_with_stubbed_tools();
    let log_path = std::env::temp_dir().join("stck-new-remote-exists.log");
    let _ = fs::remove_file(&log_path);
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
fn new_fails_when_pr_presence_check_errors() {
    let (_temp, mut cmd) = stck_cmd_with_stubbed_tools();
    let log_path = std::env::temp_dir().join("stck-new-pr-view-error.log");
    let _ = fs::remove_file(&log_path);
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

#[test]
fn push_executes_pushes_before_retargets_and_prints_summary() {
    let (_temp, mut cmd) = stck_cmd_with_stubbed_tools();
    let log_path = std::env::temp_dir().join("stck-push.log");
    let _ = fs::remove_file(&log_path);
    cmd.env("STCK_TEST_LOG", log_path.as_os_str());
    cmd.env(
        "STCK_TEST_NEEDS_PUSH_BRANCHES",
        "feature-branch,feature-child",
    );
    cmd.arg("push");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains(
            "$ git push --force-with-lease origin feature-branch",
        ))
        .stdout(predicate::str::contains(
            "$ git push --force-with-lease origin feature-child",
        ))
        .stdout(predicate::str::contains(
            "$ gh pr edit feature-branch --base main",
        ))
        .stdout(predicate::str::contains("$ gh pr edit feature-child --base feature-branch").not())
        .stdout(predicate::str::contains(
            "Push succeeded. Pushed 2 branch(es) and applied 1 PR base update(s) in this run.",
        ));

    let log = fs::read_to_string(&log_path).expect("push log should exist");
    let push_idx = log
        .find("push --force-with-lease origin feature-child")
        .expect("second push command missing");
    let retarget_idx = log
        .find("pr edit feature-branch --base main")
        .expect("first retarget command missing");
    assert!(
        push_idx < retarget_idx,
        "retarget should start only after pushes complete"
    );
}

#[test]
fn push_shows_fetch_failure_remediation() {
    let (_temp, mut cmd) = stck_cmd_with_stubbed_tools();
    cmd.env("STCK_TEST_FETCH_FAIL", "1");
    cmd.arg("push");

    cmd.assert().code(1).stderr(predicate::str::contains(
        "error: failed to fetch from `origin`; check remote connectivity and permissions",
    ));
}

#[test]
fn push_stops_before_retarget_when_a_push_fails() {
    let (_temp, mut cmd) = stck_cmd_with_stubbed_tools();
    let log_path = std::env::temp_dir().join("stck-push-fail.log");
    let _ = fs::remove_file(&log_path);
    cmd.env("STCK_TEST_LOG", log_path.as_os_str());
    cmd.env(
        "STCK_TEST_NEEDS_PUSH_BRANCHES",
        "feature-branch,feature-child",
    );
    cmd.env("STCK_TEST_PUSH_FAIL_BRANCH", "feature-child");
    cmd.arg("push");

    cmd.assert().code(1).stderr(predicate::str::contains(
        "error: push failed for branch feature-child; fix the push error and rerun `stck push`",
    ));

    let log = fs::read_to_string(&log_path).expect("push log should exist");
    assert!(
        !log.contains("pr edit"),
        "retarget should not run when a push fails"
    );
}

#[test]
fn push_resumes_after_partial_retarget_failure() {
    let (temp, mut first) = stck_cmd_with_stubbed_tools();
    let log_path = temp.path().join("push-resume.log");
    let marker_path = temp.path().join("retarget-fail-once.marker");
    first.env("STCK_TEST_LOG", log_path.as_os_str());
    first.env(
        "STCK_TEST_NEEDS_PUSH_BRANCHES",
        "feature-branch,feature-child",
    );
    first.env("STCK_TEST_FEATURE_CHILD_BASE", "main");
    first.env("STCK_TEST_RETARGET_FAIL_ONCE_FILE", marker_path.as_os_str());
    first.env("STCK_TEST_RETARGET_FAIL_ONCE_BRANCH", "feature-child");
    first.arg("push");

    first.assert().code(1).stderr(predicate::str::contains(
        "error: failed to retarget PR base for branch feature-child to feature-branch; fix the GitHub error and rerun `stck push`",
    ));

    let state_path = temp
        .path()
        .join("git-dir")
        .join("stck")
        .join("last-plan.json");
    assert!(
        state_path.exists(),
        "push state should persist after partial failure"
    );

    let mut resume = stck_cmd();
    let path = std::env::var("PATH").expect("PATH should be set");
    let full_path = format!("{}:{}", temp.path().join("bin").display(), path);
    resume.env("PATH", full_path);
    resume.env("STCK_TEST_GIT_DIR", temp.path().join("git-dir").as_os_str());
    resume.env("STCK_TEST_LOG", log_path.as_os_str());
    resume.env(
        "STCK_TEST_NEEDS_PUSH_BRANCHES",
        "feature-branch,feature-child",
    );
    resume.env("STCK_TEST_FEATURE_CHILD_BASE", "main");
    resume.arg("push");

    resume
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "$ gh pr edit feature-child --base feature-branch",
        ))
        .stdout(predicate::str::contains(
            "Push succeeded. Pushed 0 branch(es) and applied 1 PR base update(s) in this run.",
        ));

    let log = fs::read_to_string(&log_path).expect("push log should exist");
    let push_a = "push --force-with-lease origin feature-branch";
    let push_b = "push --force-with-lease origin feature-child";
    let retarget_a = "pr edit feature-branch --base main";
    let retarget_b = "pr edit feature-child --base feature-branch";
    assert_eq!(log.matches(push_a).count(), 1);
    assert_eq!(log.matches(push_b).count(), 1);
    assert_eq!(log.matches(retarget_a).count(), 1);
    assert_eq!(log.matches(retarget_b).count(), 2);
    assert!(
        !state_path.exists(),
        "push state should be removed after successful retry"
    );
}

#[test]
fn push_uses_cached_sync_plan_retargets_when_available() {
    let (temp, mut sync) = stck_cmd_with_stubbed_tools();
    let log_path = temp.path().join("push-cached-plan.log");
    let _ = fs::remove_file(&log_path);

    sync.env("STCK_TEST_LOG", log_path.as_os_str());
    sync.env(
        "STCK_TEST_NEEDS_PUSH_BRANCHES",
        "feature-branch,feature-child",
    );
    sync.arg("sync");
    sync.assert().success();

    let cached_plan_path = temp
        .path()
        .join("git-dir")
        .join("stck")
        .join("last-sync-plan.json");
    assert!(
        cached_plan_path.exists(),
        "sync should persist cached sync plan"
    );

    let mut push = stck_cmd();
    let path = std::env::var("PATH").expect("PATH should be set");
    let full_path = format!("{}:{}", temp.path().join("bin").display(), path);
    push.env("PATH", full_path);
    push.env("STCK_TEST_GIT_DIR", temp.path().join("git-dir").as_os_str());
    push.env("STCK_TEST_LOG", log_path.as_os_str());
    push.env(
        "STCK_TEST_NEEDS_PUSH_BRANCHES",
        "feature-branch,feature-child",
    );
    push.env("STCK_TEST_SYNC_NOOP", "1");
    push.arg("push");

    push.assert()
        .success()
        .stdout(predicate::str::contains(
            "$ gh pr edit feature-branch --base main",
        ))
        .stdout(predicate::str::contains("$ gh pr edit feature-child --base feature-branch").not())
        .stdout(predicate::str::contains(
            "Push succeeded. Pushed 2 branch(es) and applied 1 PR base update(s) in this run.",
        ));

    assert!(
        !cached_plan_path.exists(),
        "push should clear cached sync plan after success"
    );
}

#[test]
fn push_skips_cached_sync_plan_retargets_that_are_already_satisfied() {
    let (temp, mut sync) = stck_cmd_with_stubbed_tools();
    let log_path = temp.path().join("push-cached-plan-noop-retargets.log");
    let _ = fs::remove_file(&log_path);

    sync.env("STCK_TEST_LOG", log_path.as_os_str());
    sync.env(
        "STCK_TEST_NEEDS_PUSH_BRANCHES",
        "feature-branch,feature-child",
    );
    sync.arg("sync");
    sync.assert().success();

    let cached_plan_path = temp
        .path()
        .join("git-dir")
        .join("stck")
        .join("last-sync-plan.json");
    assert!(
        cached_plan_path.exists(),
        "sync should persist cached sync plan"
    );

    let mut push = stck_cmd();
    let path = std::env::var("PATH").expect("PATH should be set");
    let full_path = format!("{}:{}", temp.path().join("bin").display(), path);
    push.env("PATH", full_path);
    push.env("STCK_TEST_GIT_DIR", temp.path().join("git-dir").as_os_str());
    push.env("STCK_TEST_LOG", log_path.as_os_str());
    push.env(
        "STCK_TEST_NEEDS_PUSH_BRANCHES",
        "feature-branch,feature-child",
    );
    push.env("STCK_TEST_FEATURE_BRANCH_BASE", "main");
    push.arg("push");

    push.assert()
        .success()
        .stdout(predicate::str::contains(
            "Push succeeded. Pushed 2 branch(es) and applied 0 PR base update(s) in this run.",
        ))
        .stdout(predicate::str::contains("$ gh pr edit").not());

    let log = fs::read_to_string(&log_path).expect("push log should exist");
    assert!(
        !log.contains("pr edit"),
        "push should skip retarget calls when cached plan bases are already satisfied"
    );
    assert!(
        !cached_plan_path.exists(),
        "push should clear cached sync plan after success"
    );
}

#[test]
fn push_skips_branches_without_local_changes() {
    let (_temp, mut cmd) = stck_cmd_with_stubbed_tools();
    let log_path = std::env::temp_dir().join("stck-push-no-divergence.log");
    let _ = fs::remove_file(&log_path);
    cmd.env("STCK_TEST_LOG", log_path.as_os_str());
    cmd.arg("push");

    cmd.assert().success().stdout(predicate::str::contains(
        "Push succeeded. Pushed 0 branch(es) and applied 1 PR base update(s) in this run.",
    ));

    let log = fs::read_to_string(&log_path).expect("push log should exist");
    assert!(
        !log.contains("push --force-with-lease"),
        "push should skip branches without divergence"
    );
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
            "$ git checkout feature-branch",
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
    assert!(log.contains("checkout feature-branch"));
}

#[test]
fn sync_uses_remote_old_base_when_local_old_base_is_missing() {
    let (_temp, mut cmd) = stck_cmd_with_stubbed_tools();
    cmd.env("STCK_TEST_MISSING_LOCAL_BRANCH_REF", "feature-base");
    cmd.env(
        "STCK_TEST_REMOTE_FEATURE_BASE_SHA",
        "9999999999999999999999999999999999999999",
    );
    cmd.arg("sync");

    cmd.assert().success().stdout(predicate::str::contains(
        "$ git rebase --onto main 9999999999999999999999999999999999999999 feature-branch",
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
fn sync_includes_rebase_stderr_on_failure() {
    let (_temp, mut cmd) = stck_cmd_with_stubbed_tools();
    cmd.env("STCK_TEST_REBASE_FAIL", "1");
    cmd.env("STCK_TEST_REBASE_FAIL_STDERR", "1");
    cmd.arg("sync");

    cmd.assert().code(1).stderr(predicate::str::contains(
        "CONFLICT (content): Merge conflict in src/main.rs",
    ));
}

#[test]
fn sync_reports_noop_when_stack_is_already_up_to_date() {
    let (_temp, mut cmd) = stck_cmd_with_stubbed_tools();
    cmd.env("STCK_TEST_SYNC_NOOP", "1");
    cmd.arg("sync");

    cmd.assert().success().stdout(predicate::str::contains(
        "Stack is already up to date. No sync needed.",
    ));
}

#[test]
fn sync_from_mid_stack_rebases_current_and_descendants() {
    // Current branch is feature-branch (mid-stack). Sync should rebase both
    // feature-branch (child of merged feature-base) and feature-child (descendant).
    let (_temp, mut cmd) = stck_cmd_with_stubbed_tools();
    let log_path = std::env::temp_dir().join("stck-sync-mid-stack.log");
    let _ = fs::remove_file(&log_path);
    cmd.env("STCK_TEST_LOG", log_path.as_os_str());
    cmd.arg("sync");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("feature-branch"))
        .stdout(predicate::str::contains("feature-child"))
        .stdout(predicate::str::contains(
            "Sync succeeded locally. Run `stck push` to update remotes + PR bases.",
        ));

    let log = fs::read_to_string(&log_path).expect("rebase log should exist");
    // Both branches should be rebased
    assert!(
        log.contains("feature-branch"),
        "mid-stack branch should be rebased"
    );
    assert!(
        log.contains("feature-child"),
        "descendant of mid-stack branch should also be rebased"
    );
    // feature-branch rebase should come before feature-child
    let branch_idx = log
        .find("feature-branch")
        .expect("feature-branch should appear in log");
    let child_idx = log
        .find("feature-child")
        .expect("feature-child should appear in log");
    assert!(
        branch_idx < child_idx,
        "mid-stack branch should be rebased before its descendant"
    );
}

#[test]
fn sync_rebases_when_default_branch_has_advanced() {
    let (_temp, mut cmd) = stck_cmd_with_stubbed_tools();
    let log_path = std::env::temp_dir().join("stck-sync-default-advanced.log");
    let _ = fs::remove_file(&log_path);
    cmd.env("STCK_TEST_LOG", log_path.as_os_str());
    cmd.env("STCK_TEST_SYNC_NOOP", "1");
    cmd.env("STCK_TEST_DEFAULT_ADVANCED", "1");
    cmd.arg("sync");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains(
            "$ git rebase --onto main aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa feature-base",
        ))
        .stdout(predicate::str::contains(
            "$ git rebase --onto feature-base 1111111111111111111111111111111111111111 feature-branch",
        ))
        .stdout(predicate::str::contains(
            "$ git rebase --onto feature-branch 2222222222222222222222222222222222222222 feature-child",
        ));
}

#[test]
fn sync_continue_requires_existing_state() {
    let (_temp, mut cmd) = stck_cmd_with_stubbed_tools();
    cmd.args(["sync", "--continue"]);

    cmd.assert().code(1).stderr(predicate::str::contains(
        "error: no sync state found; run `stck sync` to compute a new plan",
    ));
}

#[test]
fn sync_fails_early_when_rebase_is_already_in_progress() {
    let (temp, mut cmd) = stck_cmd_with_stubbed_tools();
    fs::create_dir_all(temp.path().join("git-dir").join("rebase-merge"))
        .expect("rebase-merge dir should be created");
    cmd.arg("sync");

    cmd.assert().code(1).stderr(predicate::str::contains(
        "error: rebase is already in progress; run `git rebase --continue` or `git rebase --abort` before starting a new `stck sync`",
    ));
}

#[test]
fn sync_rejects_continue_and_reset_together() {
    let (_temp, mut cmd) = stck_cmd_with_stubbed_tools();
    cmd.args(["sync", "--continue", "--reset"]);

    cmd.assert().code(2).stderr(predicate::str::contains(
        "the argument '--continue' cannot be used with '--reset'",
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
    resume.env(
        "STCK_TEST_FEATURE_BRANCH_HEAD",
        "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
    );
    resume.args(["sync", "--continue"]);

    resume
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "$ git rebase --onto feature-branch bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb feature-child",
        ))
        .stdout(predicate::str::contains(
            "Sync succeeded locally. Run `stck push` to update remotes + PR bases.",
        ));

    let log = fs::read_to_string(&log_path).expect("rebase log should exist");
    let first_step = "rebase --onto main 1111111111111111111111111111111111111111 feature-branch";
    let second_step =
        "rebase --onto feature-branch bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb feature-child";
    assert_eq!(log.matches(first_step).count(), 1);
    assert_eq!(log.matches(second_step).count(), 1);
    assert!(
        !state_path.exists(),
        "sync state should be removed after success"
    );
}

#[test]
fn sync_reset_recomputes_from_scratch_after_failure() {
    let (temp, mut first) = stck_cmd_with_stubbed_tools();
    let log_path = temp.path().join("rebase-reset.log");
    let fail_once_path = temp.path().join("fail-once-reset.marker");

    first.env("STCK_TEST_LOG", log_path.as_os_str());
    first.env(
        "STCK_TEST_REBASE_FAIL_ONCE_FILE",
        fail_once_path.as_os_str(),
    );
    first.arg("sync");
    first.assert().code(1).stderr(predicate::str::contains(
        "error: rebase failed for branch feature-branch;",
    ));

    let mut reset = stck_cmd();
    let path = std::env::var("PATH").expect("PATH should be set");
    let full_path = format!("{}:{}", temp.path().join("bin").display(), path);
    reset.env("PATH", full_path);
    reset.env("STCK_TEST_GIT_DIR", temp.path().join("git-dir").as_os_str());
    reset.env("STCK_TEST_LOG", log_path.as_os_str());
    reset.args(["sync", "--reset"]);

    reset
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Cleared previous sync state. Recomputing from scratch.",
        ))
        .stdout(predicate::str::contains(
            "$ git rebase --onto main 1111111111111111111111111111111111111111 feature-branch",
        ))
        .stdout(predicate::str::contains(
            "$ git rebase --onto feature-branch 2222222222222222222222222222222222222222 feature-child",
        ));

    let log = fs::read_to_string(&log_path).expect("rebase log should exist");
    let first_step = "rebase --onto main 1111111111111111111111111111111111111111 feature-branch";
    let second_step =
        "rebase --onto feature-branch 2222222222222222222222222222222222222222 feature-child";
    assert_eq!(
        log.matches(first_step).count(),
        2,
        "reset should rerun first step from scratch"
    );
    assert_eq!(
        log.matches(second_step).count(),
        1,
        "second step should run once on reset recompute"
    );
}

#[test]
fn sync_continue_fails_when_rebase_was_aborted() {
    let (temp, mut first) = stck_cmd_with_stubbed_tools();
    let fail_once_path = temp.path().join("fail-once.marker");

    first.env(
        "STCK_TEST_REBASE_FAIL_ONCE_FILE",
        fail_once_path.as_os_str(),
    );
    first.arg("sync");
    first.assert().code(1).stderr(predicate::str::contains(
        "error: rebase failed for branch feature-branch; resolve conflicts, run `git rebase --continue` or `git rebase --abort`, then rerun `stck sync`",
    ));

    let mut resume = stck_cmd();
    let path = std::env::var("PATH").expect("PATH should be set");
    let full_path = format!("{}:{}", temp.path().join("bin").display(), path);
    resume.env("PATH", full_path);
    resume.env("STCK_TEST_GIT_DIR", temp.path().join("git-dir").as_os_str());
    resume.args(["sync", "--continue"]);

    resume.assert().code(1).stderr(predicate::str::contains(
        "error: no completed rebase detected for feature-branch; resolve with `git rebase --continue` (or rerun `stck sync` to retry the step)",
    ));
}

#[test]
fn sync_plain_retry_requires_continue_or_reset_after_failure() {
    let (temp, mut first) = stck_cmd_with_stubbed_tools();
    let fail_once_path = temp.path().join("fail-once-plain-retry.marker");

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

    let mut retry = stck_cmd();
    let path = std::env::var("PATH").expect("PATH should be set");
    let full_path = format!("{}:{}", temp.path().join("bin").display(), path);
    retry.env("PATH", full_path);
    retry.env("STCK_TEST_GIT_DIR", temp.path().join("git-dir").as_os_str());
    retry.arg("sync");

    retry.assert().code(1).stderr(predicate::str::contains(
        "error: sync stopped at failed step for feature-branch; run `stck sync --continue` after completing the rebase, or `stck sync --reset` to discard saved state and recompute",
    ));

    assert!(
        state_path.exists(),
        "sync state should remain available for continue or reset"
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

#[test]
fn sync_after_squash_merge_uses_merge_base_for_old_base() {
    // Default PR list has feature-base as MERGED. Sync should use merge-base
    // to find the fork point rather than just resolving the ref directly.
    let (_temp, mut cmd) = stck_cmd_with_stubbed_tools();
    let log_path = std::env::temp_dir().join("stck-sync-squash-merge.log");
    let _ = fs::remove_file(&log_path);
    cmd.env("STCK_TEST_LOG", log_path.as_os_str());
    cmd.arg("sync");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("$ git rebase --onto main"))
        .stdout(predicate::str::contains("feature-branch"))
        .stdout(predicate::str::contains(
            "Sync succeeded locally. Run `stck push` to update remotes + PR bases.",
        ));
}

#[test]
fn sync_falls_back_to_remote_ref_when_merge_base_fails() {
    let (_temp, mut cmd) = stck_cmd_with_stubbed_tools();
    cmd.env("STCK_TEST_MISSING_LOCAL_BRANCH_REF", "feature-base");
    cmd.env("STCK_TEST_MERGE_BASE_FAIL", "1");
    cmd.env(
        "STCK_TEST_REMOTE_FEATURE_BASE_SHA",
        "9999999999999999999999999999999999999999",
    );
    cmd.arg("sync");

    // Should fall back to rev_parse on the remote ref
    cmd.assert().success().stdout(predicate::str::contains(
        "$ git rebase --onto main 9999999999999999999999999999999999999999 feature-branch",
    ));
}

#[test]
fn sync_continue_uses_merge_base_for_remaining_steps() {
    // After --continue, remaining steps should use merge-base for old_base resolution
    let (temp, mut first) = stck_cmd_with_stubbed_tools();
    let log_path = temp.path().join("rebase-continue-mergebase.log");
    let fail_once_path = temp.path().join("fail-once-mergebase.marker");

    first.env("STCK_TEST_LOG", log_path.as_os_str());
    first.env(
        "STCK_TEST_REBASE_FAIL_ONCE_FILE",
        fail_once_path.as_os_str(),
    );
    first.arg("sync");
    first.assert().code(1);

    let mut resume = stck_cmd();
    let path = std::env::var("PATH").expect("PATH should be set");
    let full_path = format!("{}:{}", temp.path().join("bin").display(), path);
    resume.env("PATH", full_path);
    resume.env("STCK_TEST_GIT_DIR", temp.path().join("git-dir").as_os_str());
    resume.env("STCK_TEST_LOG", log_path.as_os_str());
    resume.env(
        "STCK_TEST_FEATURE_BRANCH_HEAD",
        "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
    );
    resume.args(["sync", "--continue"]);

    resume
        .assert()
        .success()
        .stdout(predicate::str::contains("feature-child"))
        .stdout(predicate::str::contains(
            "Sync succeeded locally. Run `stck push` to update remotes + PR bases.",
        ));

    let log = fs::read_to_string(&log_path).expect("rebase log should exist");
    // The child step should use merge-base result for old_base (feature-branch head after resolve)
    assert!(log.contains("rebase --onto feature-branch"));
    assert!(log.contains("feature-child"));
}

#[test]
fn sync_reset_with_merge_base_recomputes_correctly() {
    // After --reset, a fresh plan is computed using merge-base for old_base
    let (temp, mut first) = stck_cmd_with_stubbed_tools();
    let log_path = temp.path().join("rebase-reset-mergebase.log");
    let fail_once_path = temp.path().join("fail-once-reset-mergebase.marker");

    first.env("STCK_TEST_LOG", log_path.as_os_str());
    first.env(
        "STCK_TEST_REBASE_FAIL_ONCE_FILE",
        fail_once_path.as_os_str(),
    );
    first.arg("sync");
    first.assert().code(1);

    let mut reset = stck_cmd();
    let path = std::env::var("PATH").expect("PATH should be set");
    let full_path = format!("{}:{}", temp.path().join("bin").display(), path);
    reset.env("PATH", full_path);
    reset.env("STCK_TEST_GIT_DIR", temp.path().join("git-dir").as_os_str());
    reset.env("STCK_TEST_LOG", log_path.as_os_str());
    reset.args(["sync", "--reset"]);

    reset
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Cleared previous sync state. Recomputing from scratch.",
        ))
        .stdout(predicate::str::contains("feature-branch"))
        .stdout(predicate::str::contains("feature-child"));
}

#[test]
fn sync_then_push_after_squash_merge_produces_correct_retargets() {
    // End-to-end: sync followed by push should retarget merged PR's child to default branch
    let (temp, mut sync) = stck_cmd_with_stubbed_tools();
    let log_path = temp.path().join("sync-push-squash.log");
    let _ = fs::remove_file(&log_path);

    sync.env("STCK_TEST_LOG", log_path.as_os_str());
    sync.env(
        "STCK_TEST_NEEDS_PUSH_BRANCHES",
        "feature-branch,feature-child",
    );
    sync.arg("sync");
    sync.assert().success();

    let mut push = stck_cmd();
    let path = std::env::var("PATH").expect("PATH should be set");
    let full_path = format!("{}:{}", temp.path().join("bin").display(), path);
    push.env("PATH", full_path);
    push.env("STCK_TEST_GIT_DIR", temp.path().join("git-dir").as_os_str());
    push.env("STCK_TEST_LOG", log_path.as_os_str());
    push.env(
        "STCK_TEST_NEEDS_PUSH_BRANCHES",
        "feature-branch,feature-child",
    );
    push.arg("push");

    push.assert()
        .success()
        .stdout(predicate::str::contains(
            "$ gh pr edit feature-branch --base main",
        ))
        .stdout(predicate::str::contains("$ gh pr edit feature-child --base feature-branch").not())
        .stdout(predicate::str::contains(
            "Push succeeded. Pushed 2 branch(es) and applied 1 PR base update(s) in this run.",
        ));
}

#[test]
fn push_blocked_while_sync_state_exists() {
    // Trigger a sync failure to leave sync state on disk, then run push.
    let (temp, mut sync) = stck_cmd_with_stubbed_tools();
    sync.env("STCK_TEST_REBASE_FAIL", "1");
    sync.arg("sync");
    sync.assert().code(1);

    let state_path = temp
        .path()
        .join("git-dir")
        .join("stck")
        .join("last-plan.json");
    assert!(
        state_path.exists(),
        "sync state should persist after failure"
    );

    let mut push = stck_cmd();
    let path = std::env::var("PATH").expect("PATH should be set");
    let full_path = format!("{}:{}", temp.path().join("bin").display(), path);
    push.env("PATH", full_path);
    push.env("STCK_TEST_GIT_DIR", temp.path().join("git-dir").as_os_str());
    push.arg("push");

    push.assert().code(1).stderr(predicate::str::contains(
        "error: sync operation state is in progress; run `stck sync --continue` before running push",
    ));
}

#[test]
fn sync_blocked_while_push_state_exists() {
    // Trigger a push failure to leave push state on disk, then run sync.
    let (temp, mut push) = stck_cmd_with_stubbed_tools();
    push.env(
        "STCK_TEST_NEEDS_PUSH_BRANCHES",
        "feature-branch,feature-child",
    );
    push.env("STCK_TEST_PUSH_FAIL_BRANCH", "feature-child");
    push.arg("push");
    push.assert().code(1);

    let state_path = temp
        .path()
        .join("git-dir")
        .join("stck")
        .join("last-plan.json");
    assert!(
        state_path.exists(),
        "push state should persist after failure"
    );

    let mut sync = stck_cmd();
    let path = std::env::var("PATH").expect("PATH should be set");
    let full_path = format!("{}:{}", temp.path().join("bin").display(), path);
    sync.env("PATH", full_path);
    sync.env("STCK_TEST_GIT_DIR", temp.path().join("git-dir").as_os_str());
    sync.arg("sync");

    sync.assert().code(1).stderr(predicate::str::contains(
        "error: push operation state is in progress; run `stck push` before starting a new sync",
    ));
}
