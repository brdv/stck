mod harness;

use harness::{log_path, stck_cmd_for_temp, stck_cmd_with_stubbed_tools};
use predicates::prelude::*;
use std::fs;

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
fn sync_executes_rebase_plan_and_prints_success_message() {
    let (temp, mut cmd) = stck_cmd_with_stubbed_tools();
    let log_path = log_path(&temp, "stck-sync-rebase.log");
    cmd.env("STCK_TEST_LOG", log_path.as_os_str());
    cmd.arg("sync");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains(
            "$ git rebase --onto refs/remotes/origin/main 1111111111111111111111111111111111111111 feature-branch",
        ))
        .stdout(predicate::str::contains(
            "$ git rebase --onto refs/heads/feature-branch 2222222222222222222222222222222222222222 feature-child",
        ))
        .stdout(predicate::str::contains("$ git checkout feature-branch"))
        .stdout(predicate::str::contains(
            "Sync succeeded locally. Run `stck push` to update remotes + PR bases.",
        ));

    let log = fs::read_to_string(&log_path).expect("rebase log should exist");
    assert!(
        log.contains("rebase --onto refs/remotes/origin/main 1111111111111111111111111111111111111111 feature-branch")
    );
    assert!(log.contains(
        "rebase --onto refs/heads/feature-branch 2222222222222222222222222222222222222222 feature-child"
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
        "$ git rebase --onto refs/remotes/origin/main 9999999999999999999999999999999999999999 feature-branch",
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
    let (temp, mut cmd) = stck_cmd_with_stubbed_tools();
    let log_path = log_path(&temp, "stck-sync-mid-stack.log");
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
    assert!(
        log.contains("feature-branch"),
        "mid-stack branch should be rebased"
    );
    assert!(
        log.contains("feature-child"),
        "descendant of mid-stack branch should also be rebased"
    );
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
    let (temp, mut cmd) = stck_cmd_with_stubbed_tools();
    let log_path = log_path(&temp, "stck-sync-default-advanced.log");
    cmd.env("STCK_TEST_LOG", log_path.as_os_str());
    cmd.env("STCK_TEST_SYNC_NOOP", "1");
    cmd.env("STCK_TEST_DEFAULT_ADVANCED", "1");
    cmd.arg("sync");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains(
            "$ git rebase --onto refs/remotes/origin/main aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa feature-base",
        ))
        .stdout(predicate::str::contains(
            "$ git rebase --onto refs/heads/feature-base 1111111111111111111111111111111111111111 feature-branch",
        ))
        .stdout(predicate::str::contains(
            "$ git rebase --onto refs/heads/feature-branch 2222222222222222222222222222222222222222 feature-child",
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
    let log_path = log_path(&temp, "rebase.log");
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

    let mut resume = stck_cmd_for_temp(&temp);
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
            "$ git rebase --onto refs/remotes/origin/feature-branch 2222222222222222222222222222222222222222 feature-child",
        ))
        .stdout(predicate::str::contains(
            "Sync succeeded locally. Run `stck push` to update remotes + PR bases.",
        ));

    let log = fs::read_to_string(&log_path).expect("rebase log should exist");
    let first_step = "rebase --onto refs/remotes/origin/main 1111111111111111111111111111111111111111 feature-branch";
    let second_step =
        "rebase --onto refs/remotes/origin/feature-branch 2222222222222222222222222222222222222222 feature-child";
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
    let log_path = log_path(&temp, "rebase-reset.log");
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

    let mut reset = stck_cmd_for_temp(&temp);
    reset.env("STCK_TEST_LOG", log_path.as_os_str());
    reset.args(["sync", "--reset"]);

    reset
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Cleared previous sync state. Recomputing from scratch.",
        ))
        .stdout(predicate::str::contains(
            "$ git rebase --onto refs/remotes/origin/main 1111111111111111111111111111111111111111 feature-branch",
        ))
        .stdout(predicate::str::contains(
            "$ git rebase --onto refs/heads/feature-branch 2222222222222222222222222222222222222222 feature-child",
        ));

    let log = fs::read_to_string(&log_path).expect("rebase log should exist");
    let first_step = "rebase --onto refs/remotes/origin/main 1111111111111111111111111111111111111111 feature-branch";
    let second_step =
        "rebase --onto refs/heads/feature-branch 2222222222222222222222222222222222222222 feature-child";
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

    let mut resume = stck_cmd_for_temp(&temp);
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

    let mut retry = stck_cmd_for_temp(&temp);
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
fn sync_after_squash_merge_uses_merge_base_for_old_base() {
    let (temp, mut cmd) = stck_cmd_with_stubbed_tools();
    let log_path = log_path(&temp, "stck-sync-squash-merge.log");
    cmd.env("STCK_TEST_LOG", log_path.as_os_str());
    cmd.arg("sync");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains(
            "$ git rebase --onto refs/remotes/origin/main",
        ))
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

    cmd.assert().success().stdout(predicate::str::contains(
        "$ git rebase --onto refs/remotes/origin/main 9999999999999999999999999999999999999999 feature-branch",
    ));
}

#[test]
fn sync_continue_uses_merge_base_for_remaining_steps() {
    let (temp, mut first) = stck_cmd_with_stubbed_tools();
    let log_path = log_path(&temp, "rebase-continue-mergebase.log");
    let fail_once_path = temp.path().join("fail-once-mergebase.marker");

    first.env("STCK_TEST_LOG", log_path.as_os_str());
    first.env(
        "STCK_TEST_REBASE_FAIL_ONCE_FILE",
        fail_once_path.as_os_str(),
    );
    first.arg("sync");
    first.assert().code(1);

    let mut resume = stck_cmd_for_temp(&temp);
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
    assert!(log.contains("rebase --onto refs/remotes/origin/feature-branch"));
    assert!(log.contains("feature-child"));
}

#[test]
fn sync_reset_with_merge_base_recomputes_correctly() {
    let (temp, mut first) = stck_cmd_with_stubbed_tools();
    let log_path = log_path(&temp, "rebase-reset-mergebase.log");
    let fail_once_path = temp.path().join("fail-once-reset-mergebase.marker");

    first.env("STCK_TEST_LOG", log_path.as_os_str());
    first.env(
        "STCK_TEST_REBASE_FAIL_ONCE_FILE",
        fail_once_path.as_os_str(),
    );
    first.arg("sync");
    first.assert().code(1);

    let mut reset = stck_cmd_for_temp(&temp);
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
fn sync_uses_remote_main_when_local_main_is_stale() {
    // After a parent PR merges on GitHub, origin/main advances but local main
    // stays stale (user hasn't pulled). Sync must use the fetched remote ref
    // for both the merge-base calculation and the --onto target.
    //
    // Scenario:
    //   - feature-base PR merged into main on GitHub
    //   - GitHub auto-retargets feature-branch base to main
    //   - origin/main at bbbb (advanced, includes merged PR)
    //   - local main at aaaa (stale, hasn't been pulled)
    //
    // Expected: rebase uses origin/main (bbbb) for both --onto and old-base
    let (temp, mut cmd) = stck_cmd_with_stubbed_tools();
    let log_path = log_path(&temp, "stck-sync-stale-main.log");
    cmd.env("STCK_TEST_LOG", log_path.as_os_str());
    // feature-branch PR base is already retargeted to main (by GitHub auto-retarget)
    cmd.env("STCK_TEST_FEATURE_BRANCH_BASE", "main");
    // Local main is stale (hasn't been pulled after the merge)
    cmd.env(
        "STCK_TEST_LOCAL_MAIN_SHA",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    );
    // origin/main has advanced (includes the merged PR)
    cmd.env(
        "STCK_TEST_REMOTE_MAIN_SHA",
        "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
    );
    // origin/main is NOT an ancestor of feature-branch (main advanced past fork point)
    cmd.env("STCK_TEST_NOT_ANCESTOR_PAIRS", "main:feature-branch");
    cmd.arg("sync");

    cmd.assert().success();

    let log = fs::read_to_string(&log_path).expect("sync log should exist");

    // After the fix, the rebase should use the remote merge-base (bbbb) and
    // the remote onto ref (refs/remotes/origin/main), not the stale local
    // main (aaaa / refs/heads/main).
    assert!(
        log.contains("rebase --onto refs/remotes/origin/main bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb feature-branch"),
        "sync should use remote merge-base and remote onto ref. Log: {log}"
    );
    assert!(
        !log.contains(
            "rebase --onto refs/remotes/origin/main aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        ),
        "sync should NOT use stale local main as old-base. Log: {log}"
    );
}

#[test]
fn sync_chained_rebase_uses_local_ref_for_previously_rebased_branch() {
    // After a bottom PR merges, sync cascades through the chain:
    //   Step 1: rebase feature-branch onto main (from feature-base)
    //   Step 2: rebase feature-child onto feature-branch
    //
    // Step 1's --onto target should be refs/remotes/origin/main (remote is
    // up-to-date after fetch; local main may be stale).
    //
    // Step 2's --onto target should be refs/heads/feature-branch (local ref
    // was just updated by the rebase in step 1; remote hasn't been pushed yet
    // and is stale).
    //
    // Without the fix, step 2 uses refs/remotes/origin/feature-branch which
    // still points to the pre-rebase commit, making the rebase a no-op.
    let (temp, mut cmd) = stck_cmd_with_stubbed_tools();
    let log_path = log_path(&temp, "stck-sync-chained.log");
    cmd.env("STCK_TEST_LOG", log_path.as_os_str());
    cmd.arg("sync");

    cmd.assert().success();

    let log = fs::read_to_string(&log_path).expect("sync log should exist");

    // Step 1: onto target must be the remote ref (handles stale local main)
    assert!(
        log.contains("rebase --onto refs/remotes/origin/main"),
        "step 1 should use remote ref for default branch. Log:\n{log}"
    );

    // Step 2: onto target must be the LOCAL ref (parent was just rebased)
    assert!(
        log.contains("rebase --onto refs/heads/feature-branch"),
        "step 2 should use local ref for branch rebased in prior step. Log:\n{log}"
    );
    assert!(
        !log.contains("rebase --onto refs/remotes/origin/feature-branch"),
        "step 2 should NOT use stale remote ref for branch rebased in prior step. Log:\n{log}"
    );
}

#[test]
fn sync_blocked_while_push_state_exists() {
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

    let mut sync = stck_cmd_for_temp(&temp);
    sync.arg("sync");

    sync.assert().code(1).stderr(predicate::str::contains(
        "error: push operation state is in progress; run `stck push` before starting a new sync",
    ));
}
