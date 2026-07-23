#![allow(dead_code)]

use assert_cmd::cargo::cargo_bin_cmd;
use assert_cmd::Command;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command as StdCommand, Output};
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

if [[ "${1:-}" == "for-each-ref" && "${2:-}" == "--format=%(refname:strip=3)" && "${3:-}" == "refs/remotes/origin" ]]; then
  if [[ "${STCK_TEST_LIST_ORIGIN_BRANCHES_FAIL:-0}" == "1" ]]; then
    echo "failed to enumerate remote refs" >&2
    exit 1
  fi
  if [[ -n "${STCK_TEST_ORIGIN_BRANCHES:-}" ]]; then
    printf '%s\n' "${STCK_TEST_ORIGIN_BRANCHES}"
  else
    printf '%s\n' main feature-base feature-branch feature-child
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
      main)
        if [[ -n "${STCK_TEST_LOCAL_MAIN_SHA:-}" ]]; then
          echo "${STCK_TEST_LOCAL_MAIN_SHA}"
        else
          echo "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        fi
        ;;
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
      main)
        if [[ -n "${STCK_TEST_REMOTE_MAIN_SHA:-}" ]]; then
          echo "${STCK_TEST_REMOTE_MAIN_SHA}"
        else
          echo "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        fi
        ;;
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
  if [[ "${STCK_TEST_REF_LOOKUP_FAIL:-}" == "${ref}" ]]; then
    exit 128
  fi
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
    refs/heads/main)
      if [[ -n "${STCK_TEST_LOCAL_MAIN_SHA:-}" ]]; then
        echo "${STCK_TEST_LOCAL_MAIN_SHA}"
      else
        echo "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
      fi
      ;;
    refs/remotes/origin/main)
      if [[ -n "${STCK_TEST_REMOTE_MAIN_SHA:-}" ]]; then
        echo "${STCK_TEST_REMOTE_MAIN_SHA}"
      else
        echo "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
      fi
      ;;
    *) echo "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" ;;
  esac
  exit 0
fi

if [[ "${1:-}" == "merge-base" && "${2:-}" == "--is-ancestor" ]]; then
  ancestor="${3:-}"
  descendant="${4:-}"
  if [[ "${STCK_TEST_STALE_LOCAL_PARENT:-}" != "" && "${ancestor}" == "refs/heads/${STCK_TEST_STALE_LOCAL_PARENT}" ]]; then
    exit 1
  fi
  if [[ "${STCK_TEST_MISSING_LOCAL_BRANCH_REF:-}" != "" && ( "${ancestor}" == "refs/heads/${STCK_TEST_MISSING_LOCAL_BRANCH_REF}" || "${descendant}" == "refs/heads/${STCK_TEST_MISSING_LOCAL_BRANCH_REF}" ) ]]; then
    exit 128
  fi
  if [[ "${STCK_TEST_MISSING_REMOTE_BRANCH:-}" != "" && ( "${ancestor}" == "refs/remotes/origin/${STCK_TEST_MISSING_REMOTE_BRANCH}" || "${descendant}" == "refs/remotes/origin/${STCK_TEST_MISSING_REMOTE_BRANCH}" ) ]]; then
    exit 128
  fi
  if [[ "${STCK_TEST_DEFAULT_ADVANCED:-0}" == "1" && "${ancestor}" == "refs/remotes/origin/main" && "${descendant}" == "refs/heads/feature-base" ]]; then
    exit 1
  fi
  ancestor_branch="${ancestor#refs/heads/}"
  ancestor_branch="${ancestor_branch#refs/remotes/origin/}"
  descendant_branch="${descendant#refs/heads/}"
  descendant_branch="${descendant_branch#refs/remotes/origin/}"

  if [[ ",${STCK_TEST_ANCESTRY_ERROR_PAIRS:-}," == *",${ancestor_branch}:${descendant_branch},"* ]]; then
    exit 128
  fi

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

if [[ "${1:-}" == "push" && "${2:-}" == "origin" ]]; then
  branch="${3:-}"
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
  printf 'example/stck\tmain\n'
  exit 0
fi

if [[ "${1:-}" == "pr" && "${2:-}" == "list" ]]; then
  if [[ -n "${STCK_TEST_LOG:-}" ]]; then
    echo "$*" >> "${STCK_TEST_LOG}"
  fi
  if [[ "${STCK_TEST_PR_VIEW_ERROR:-0}" == "1" ]]; then
    echo "network unavailable" >&2
    exit 1
  fi
  if [[ "${STCK_TEST_PR_LIST_FAIL:-0}" == "1" ]]; then
    echo "failed to list pull requests" >&2
    exit 1
  fi

  pr_list_state="all"
  pr_list_base=""
  pr_list_head=""
  for ((i=1; i<=$#; i++)); do
    if [[ "${!i}" == "--state" ]]; then
      next=$((i+1))
      pr_list_state="${!next}"
    fi
    if [[ "${!i}" == "--base" ]]; then
      next=$((i+1))
      pr_list_base="${!next}"
    fi
    if [[ "${!i}" == "--head" ]]; then
      next=$((i+1))
      pr_list_head="${!next}"
    fi
  done

  if [[ -n "${STCK_TEST_PR_LIST_FAIL_HEAD:-}" && "${pr_list_head}" == "${STCK_TEST_PR_LIST_FAIL_HEAD}" ]]; then
    echo "failed to list pull requests" >&2
    exit 1
  fi

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

  if [[ -n "${pr_list_head}" ]]; then
    if [[ "${pr_list_state}" == "open" ]]; then
      if [[ "${STCK_TEST_HAS_CURRENT_PR:-0}" == "1" && "${pr_list_head}" == "feature-branch" ]]; then
        echo '[{"headRefName":"feature-branch","isCrossRepository":false}]'
      elif [[ -n "${STCK_TEST_OPEN_PRS_JSON:-}" ]]; then
        echo "${STCK_TEST_OPEN_PRS_JSON}"
      else
        echo '[]'
      fi
      exit 0
    fi

    if [[ "${STCK_TEST_MISSING_CURRENT_PR:-0}" == "1" && "${pr_list_head}" == "feature-branch" ]]; then
      echo '[]'
      exit 0
    fi

    if [[ -n "${STCK_TEST_FEATURE_BRANCH_BASE:-}" && "${pr_list_head}" == "feature-branch" ]]; then
      echo "[{\"number\":101,\"headRefName\":\"feature-branch\",\"baseRefName\":\"${STCK_TEST_FEATURE_BRANCH_BASE}\",\"state\":\"OPEN\",\"isCrossRepository\":false}]"
      exit 0
    fi

    if [[ -n "${STCK_TEST_FEATURE_CHILD_BASE:-}" && "${pr_list_head}" == "feature-child" ]]; then
      echo "[{\"number\":102,\"headRefName\":\"feature-child\",\"baseRefName\":\"${STCK_TEST_FEATURE_CHILD_BASE}\",\"state\":\"OPEN\",\"isCrossRepository\":false}]"
      exit 0
    fi

    if [[ "${STCK_TEST_NON_LINEAR:-0}" == "1" ]]; then
      case "${pr_list_head}" in
        feature-base) echo '[{"number":100,"headRefName":"feature-base","baseRefName":"main","state":"MERGED","isCrossRepository":false}]' ;;
        feature-branch) echo '[{"number":101,"headRefName":"feature-branch","baseRefName":"feature-base","state":"OPEN","isCrossRepository":false}]' ;;
        *) echo '[]' ;;
      esac
      exit 0
    fi

    if [[ "${STCK_TEST_SYNC_NOOP:-0}" == "1" ]]; then
      case "${pr_list_head}" in
        feature-base) echo '[{"number":100,"headRefName":"feature-base","baseRefName":"main","state":"OPEN","isCrossRepository":false}]' ;;
        feature-branch) echo '[{"number":101,"headRefName":"feature-branch","baseRefName":"feature-base","state":"OPEN","isCrossRepository":false}]' ;;
        feature-child) echo '[{"number":102,"headRefName":"feature-child","baseRefName":"feature-branch","state":"OPEN","isCrossRepository":false}]' ;;
        *) echo '[]' ;;
      esac
      exit 0
    fi

    case "${pr_list_head}" in
      feature-base) echo '[{"number":100,"headRefName":"feature-base","baseRefName":"main","state":"MERGED","isCrossRepository":false}]' ;;
      feature-branch) echo '[{"number":101,"headRefName":"feature-branch","baseRefName":"feature-base","state":"OPEN","isCrossRepository":false}]' ;;
      feature-child) echo '[{"number":102,"headRefName":"feature-child","baseRefName":"feature-branch","state":"OPEN","isCrossRepository":false}]' ;;
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
  body=""
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --base) base="${2:-}"; shift 2 ;;
      --head) head="${2:-}"; shift 2 ;;
      --title) shift 2 ;;
      --body) body="${2:-}"; shift 2 ;;
      *) shift ;;
    esac
  done
  if [[ -n "${STCK_TEST_LOG:-}" ]]; then
    printf 'pr create --base %s --head %s\npr body --head %s\n%s\n' \
      "${base}" "${head}" "${head}" "${body}" >> "${STCK_TEST_LOG}"
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

/// A temporary repository that runs the real `git` binary against a local
/// bare `origin`, while keeping all GitHub interactions offline through a
/// configurable `gh` stub.
pub struct RealGitRepo {
    _temp: TempDir,
    worktree: PathBuf,
    remote: PathBuf,
    bin_dir: PathBuf,
    gh_responses: PathBuf,
    gh_log: PathBuf,
    global_git_config: PathBuf,
}

impl RealGitRepo {
    pub fn new() -> Self {
        let temp = TempDir::new().expect("real-git tempdir should be created");
        let worktree = temp.path().join("worktree");
        let remote = temp.path().join("origin.git");
        let bin_dir = temp.path().join("bin");
        let gh_responses = temp.path().join("gh-responses");
        let gh_log = temp.path().join("gh.log");
        let global_git_config = temp.path().join("global.gitconfig");

        fs::create_dir_all(&bin_dir).expect("real-git bin dir should be created");
        fs::create_dir_all(&gh_responses).expect("real-git gh response dir should be created");
        fs::write(&gh_log, "").expect("real-git gh log should be initialized");
        fs::write(&global_git_config, "")
            .expect("isolated global git config should be initialized");

        write_stub(
            &bin_dir.join("gh"),
            r#"#!/usr/bin/env bash
set -euo pipefail

printf '%s\n' "$*" >> "${STCK_REAL_GH_LOG}"

if [[ "${1:-}" == "--version" ]]; then
  echo "gh version 2.0.0"
  exit 0
fi

if [[ "${1:-}" == "auth" && "${2:-}" == "status" ]]; then
  exit 0
fi

if [[ "${1:-}" == "repo" && "${2:-}" == "view" ]]; then
  printf 'example/stck\tmain\n'
  exit 0
fi

if [[ "${1:-}" == "pr" && "${2:-}" == "view" ]]; then
  branch="${3:-}"
  safe_branch="${branch//\//__}"
  response="${STCK_REAL_GH_RESPONSES}/pr-view-${safe_branch}.json"
  if [[ -f "${response}" ]]; then
    cat "${response}"
    exit 0
  fi
  echo "no pull requests found for branch ${branch}" >&2
  exit 1
fi

if [[ "${1:-}" == "pr" && "${2:-}" == "list" ]]; then
  base=""
  head=""
  state="all"
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --base) base="${2:-}"; shift 2 ;;
      --head) head="${2:-}"; shift 2 ;;
      --state) state="${2:-}"; shift 2 ;;
      *) shift ;;
    esac
  done

  if [[ -n "${head}" ]]; then
    safe_head="${head//\//__}"
    response="${STCK_REAL_GH_RESPONSES}/pr-list-head-${state}-${safe_head}.json"
  elif [[ -n "${base}" ]]; then
    safe_base="${base//\//__}"
    response="${STCK_REAL_GH_RESPONSES}/pr-list-base-${safe_base}.json"
  else
    response="${STCK_REAL_GH_RESPONSES}/pr-list-open.json"
  fi

  if [[ -f "${response}" ]]; then
    cat "${response}"
  else
    echo "[]"
  fi
  exit 0
fi

if [[ "${1:-}" == "pr" && ( "${2:-}" == "create" || "${2:-}" == "edit" ) ]]; then
  exit 0
fi

echo "unsupported gh invocation: $*" >&2
exit 1
"#,
        );

        let remote_arg = remote.to_string_lossy().into_owned();
        let worktree_arg = worktree.to_string_lossy().into_owned();
        assert_git_success(
            temp.path(),
            &global_git_config,
            &["init", "--bare", &remote_arg],
        );
        assert_git_success(
            temp.path(),
            &global_git_config,
            &[
                "--git-dir",
                &remote_arg,
                "symbolic-ref",
                "HEAD",
                "refs/heads/main",
            ],
        );
        assert_git_success(temp.path(), &global_git_config, &["init", &worktree_arg]);

        let repo = Self {
            _temp: temp,
            worktree,
            remote,
            bin_dir,
            gh_responses,
            gh_log,
            global_git_config,
        };

        let hooks_dir = repo.worktree.join(".git").join("test-hooks");
        fs::create_dir_all(&hooks_dir).expect("isolated hooks dir should be created");
        let hooks_arg = hooks_dir.to_string_lossy().into_owned();
        let remote_arg = repo.remote.to_string_lossy().into_owned();

        repo.git_success(&["symbolic-ref", "HEAD", "refs/heads/main"]);
        repo.git_success(&["config", "user.name", "stck test"]);
        repo.git_success(&["config", "user.email", "stck-test@example.com"]);
        repo.git_success(&["config", "commit.gpgsign", "false"]);
        repo.git_success(&["config", "core.hooksPath", &hooks_arg]);
        repo.git_success(&["remote", "add", "origin", &remote_arg]);

        fs::write(repo.worktree.join("README.md"), "initial\n")
            .expect("initial repository file should be written");
        repo.git_success(&["add", "README.md"]);
        repo.git_success(&["commit", "-m", "Initial commit"]);
        repo.git_success(&["push", "-u", "origin", "main"]);

        repo
    }

    pub fn stck_cmd(&self) -> Command {
        let mut paths = vec![self.bin_dir.clone()];
        paths.extend(std::env::split_paths(
            &std::env::var_os("PATH").expect("PATH should be set"),
        ));
        let path = std::env::join_paths(paths).expect("test PATH should be valid");

        let mut cmd = stck_cmd();
        cmd.current_dir(&self.worktree);
        cmd.env("PATH", path);
        cmd.env("GIT_CONFIG_GLOBAL", &self.global_git_config);
        cmd.env("GIT_CONFIG_NOSYSTEM", "1");
        cmd.env("STCK_REAL_GH_LOG", &self.gh_log);
        cmd.env("STCK_REAL_GH_RESPONSES", &self.gh_responses);
        cmd.env_remove("GIT_DIR");
        cmd.env_remove("GIT_WORK_TREE");
        cmd
    }

    pub fn create_branch(&self, branch: &str) {
        self.git_success(&["checkout", "-b", branch]);
    }

    pub fn checkout(&self, branch: &str) {
        self.git_success(&["checkout", branch]);
    }

    pub fn delete_local_branch(&self, branch: &str) {
        self.git_success(&["branch", "-d", branch]);
    }

    pub fn commit_file(&self, relative_path: &str, contents: &str, message: &str) {
        let path = self.worktree.join(relative_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("commit file parent should be created");
        }
        fs::write(&path, contents).expect("commit file should be written");
        self.git_success(&["add", "--", relative_path]);
        self.git_success(&["commit", "-m", message]);
    }

    pub fn push(&self, branch: &str) {
        self.git_success(&["push", "-u", "origin", branch]);
    }

    pub fn local_sha(&self, reference: &str) -> String {
        self.git_stdout(&["rev-parse", reference])
    }

    pub fn remote_sha(&self, branch: &str) -> String {
        let remote_arg = self.remote.to_string_lossy().into_owned();
        let output = assert_git_success(
            self._temp.path(),
            &self.global_git_config,
            &[
                "--git-dir",
                &remote_arg,
                "rev-parse",
                &format!("refs/heads/{branch}"),
            ],
        );
        trimmed_stdout(output.stdout)
    }

    pub fn current_branch(&self) -> String {
        self.git_stdout(&["branch", "--show-current"])
    }

    pub fn is_ancestor(&self, ancestor: &str, descendant: &str) -> bool {
        let output = self.git_output(&["merge-base", "--is-ancestor", ancestor, descendant]);
        match output.status.code() {
            Some(0) => true,
            Some(1) => false,
            _ => panic!(
                "git merge-base failed\nstdout: {}\nstderr: {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ),
        }
    }

    pub fn resolve_rebase_conflict(&self, relative_path: &str, contents: &str) {
        fs::write(self.worktree.join(relative_path), contents)
            .expect("resolved conflict contents should be written");
        self.git_success(&["add", "--", relative_path]);
        self.git_success(&["-c", "core.editor=true", "rebase", "--continue"]);
    }

    pub fn install_failing_pre_rebase_hook(&self) {
        write_stub(
            &self
                .worktree
                .join(".git")
                .join("test-hooks")
                .join("pre-rebase"),
            "#!/usr/bin/env bash\nexit 1\n",
        );
    }

    pub fn remove_pre_rebase_hook(&self) {
        let hook = self
            .worktree
            .join(".git")
            .join("test-hooks")
            .join("pre-rebase");
        fs::remove_file(hook).expect("pre-rebase hook should be removed");
    }

    pub fn sync_state_exists(&self) -> bool {
        self.worktree
            .join(".git")
            .join("stck")
            .join("last-plan.json")
            .exists()
    }

    pub fn write_pr_response(&self, branch: &str, json: &str) {
        let branch = branch.replace('/', "__");
        fs::write(
            self.gh_responses.join(format!("pr-view-{branch}.json")),
            json,
        )
        .expect("gh PR response should be written");
        fs::write(
            self.gh_responses
                .join(format!("pr-list-head-all-{branch}.json")),
            format!("[{json}]"),
        )
        .expect("gh all-state head response should be written");
        if json.contains(r#""state":"OPEN""#) {
            fs::write(
                self.gh_responses
                    .join(format!("pr-list-head-open-{branch}.json")),
                format!("[{json}]"),
            )
            .expect("gh open-state head response should be written");
        }
    }

    pub fn write_children_response(&self, base: &str, json: &str) {
        let base = base.replace('/', "__");
        fs::write(
            self.gh_responses.join(format!("pr-list-base-{base}.json")),
            json,
        )
        .expect("gh children response should be written");
    }

    pub fn write_open_pr_head_response(&self, head: &str, json: &str) {
        let head = head.replace('/', "__");
        fs::write(
            self.gh_responses
                .join(format!("pr-list-head-open-{head}.json")),
            json,
        )
        .expect("gh head response should be written");
    }

    pub fn write_open_prs_response(&self, json: &str) {
        fs::write(self.gh_responses.join("pr-list-open.json"), json)
            .expect("gh open PR response should be written");
    }

    pub fn gh_log(&self) -> String {
        fs::read_to_string(&self.gh_log).expect("gh log should be readable")
    }

    fn git_stdout(&self, args: &[&str]) -> String {
        trimmed_stdout(self.git_success(args).stdout)
    }

    fn git_success(&self, args: &[&str]) -> Output {
        let output = self.git_output(args);
        if !output.status.success() {
            panic!(
                "git {} failed\nstdout: {}\nstderr: {}",
                args.join(" "),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
        output
    }

    fn git_output(&self, args: &[&str]) -> Output {
        run_git(&self.worktree, &self.global_git_config, args)
    }
}

fn assert_git_success(cwd: &Path, global_git_config: &Path, args: &[&str]) -> Output {
    let output = run_git(cwd, global_git_config, args);
    if !output.status.success() {
        panic!(
            "git {} failed\nstdout: {}\nstderr: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    output
}

fn run_git(cwd: &Path, global_git_config: &Path, args: &[&str]) -> Output {
    StdCommand::new("git")
        .current_dir(cwd)
        .env("GIT_CONFIG_GLOBAL", global_git_config)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .args(args)
        .output()
        .expect("git command should run")
}

fn trimmed_stdout(stdout: Vec<u8>) -> String {
    String::from_utf8(stdout)
        .expect("git stdout should be UTF-8")
        .trim()
        .to_string()
}
