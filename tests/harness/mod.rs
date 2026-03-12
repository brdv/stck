#![allow(dead_code)]

use assert_cmd::cargo::cargo_bin_cmd;
use assert_cmd::Command;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

pub fn stck_cmd() -> Command {
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
  if [[ "${STCK_TEST_MERGE_BASE_FAIL:-0}" == "1" ]]; then
    exit 1
  fi
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
  ancestor_branch="${ancestor#refs/heads/}"
  ancestor_branch="${ancestor_branch#refs/remotes/origin/}"
  descendant_branch="${descendant#refs/heads/}"
  descendant_branch="${descendant_branch#refs/remotes/origin/}"

  if [[ -n "${STCK_TEST_NOT_ANCESTOR_PAIRS:-}" ]]; then
    IFS=',' read -ra pairs <<< "${STCK_TEST_NOT_ANCESTOR_PAIRS}"
    for pair in "${pairs[@]}"; do
      IFS=':' read -r pa pd <<< "${pair}"
      if [[ "${ancestor_branch}" == "${pa}" && "${descendant_branch}" == "${pd}" ]]; then
        exit 1
      fi
    done
  fi

  if [[ -n "${STCK_TEST_ANCESTOR_PAIRS:-}" ]]; then
    IFS=',' read -ra pairs <<< "${STCK_TEST_ANCESTOR_PAIRS}"
    for pair in "${pairs[@]}"; do
      IFS=':' read -r pa pd <<< "${pair}"
      if [[ "${ancestor_branch}" == "${pa}" && "${descendant_branch}" == "${pd}" ]]; then
        exit 0
      fi
    done
  fi

  if [[ "${ancestor_branch}" == "${descendant_branch}" ]]; then
    exit 0
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
    case "${pr_list_base}" in
      feature-base) echo '[{"number":101,"headRefName":"feature-branch","baseRefName":"feature-base","state":"OPEN"}]' ;;
      feature-branch) echo '[{"number":102,"headRefName":"feature-child","baseRefName":"feature-branch","state":"OPEN"}]' ;;
      *) echo '[]' ;;
    esac
    exit 0
  fi

  if [[ -n "${STCK_TEST_OPEN_PRS_JSON:-}" && "${pr_list_state}" == "open" ]]; then
    echo "${STCK_TEST_OPEN_PRS_JSON}"
    exit 0
  fi

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

    case "${branch}" in
      feature-base) echo '{"number":100,"headRefName":"feature-base","baseRefName":"main","state":"MERGED"}' ;;
      feature-branch) echo '{"number":101,"headRefName":"feature-branch","baseRefName":"feature-base","state":"OPEN"}' ;;
      feature-child) echo '{"number":102,"headRefName":"feature-child","baseRefName":"feature-branch","state":"OPEN"}' ;;
      *) echo "no pull requests found for branch ${branch}" >&2; exit 1 ;;
    esac
    exit 0
  fi

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

pub fn stck_cmd_for_temp(temp: &TempDir) -> Command {
    let path = std::env::var("PATH").expect("PATH should be set");
    let full_path = format!("{}:{}", temp.path().join("bin").display(), path);
    let git_dir = temp.path().join("git-dir");
    fs::create_dir_all(&git_dir).expect("git dir should be created");

    let mut cmd = stck_cmd();
    cmd.env("PATH", full_path);
    cmd.env("STCK_TEST_GIT_DIR", git_dir.as_os_str());
    cmd
}

pub fn stck_cmd_with_stubbed_tools() -> (TempDir, Command) {
    let temp = setup_stubbed_tools();
    let cmd = stck_cmd_for_temp(&temp);
    (temp, cmd)
}

pub fn log_path(temp: &TempDir, name: &str) -> PathBuf {
    temp.path().join(name)
}
