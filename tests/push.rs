mod harness;

use harness::{log_path, stck_cmd_for_temp, stck_cmd_with_stubbed_tools};
use predicates::prelude::*;
use std::fs;

#[test]
fn push_executes_pushes_before_retargets_and_prints_summary() {
    let (temp, mut cmd) = stck_cmd_with_stubbed_tools();
    let log_path = log_path(&temp, "stck-push.log");
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
fn shows_detached_head_remediation() {
    let (_temp, mut cmd) = stck_cmd_with_stubbed_tools();
    cmd.env("STCK_TEST_DETACHED_HEAD", "1");
    cmd.arg("push");

    cmd.assert().code(1).stderr(predicate::str::contains(
        "error: not on a branch (detached HEAD); checkout a branch and retry",
    ));
}

#[test]
fn push_stops_before_retarget_when_a_push_fails() {
    let (temp, mut cmd) = stck_cmd_with_stubbed_tools();
    let log_path = log_path(&temp, "stck-push-fail.log");
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
    let log_path = log_path(&temp, "push-resume.log");
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

    let mut resume = stck_cmd_for_temp(&temp);
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
    let log_path = log_path(&temp, "push-cached-plan.log");

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

    let mut push = stck_cmd_for_temp(&temp);
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
    let log_path = log_path(&temp, "push-cached-plan-noop-retargets.log");

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

    let mut push = stck_cmd_for_temp(&temp);
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
fn push_resume_clears_stale_state_when_remaining_retargets_are_already_satisfied() {
    let (temp, mut push) = stck_cmd_with_stubbed_tools();
    let log_path = log_path(&temp, "push-resume-stale-state.log");

    let stck_dir = temp.path().join("git-dir").join("stck");
    fs::create_dir_all(&stck_dir).expect("stck state dir should exist");
    let state_path = stck_dir.join("last-plan.json");
    fs::write(
        &state_path,
        r#"{
  "kind": "push",
  "push_branches": ["feature-branch", "feature-child"],
  "completed_pushes": 2,
  "retargets": [
    {"branch": "feature-branch", "new_base_ref": "main"},
    {"branch": "feature-child", "new_base_ref": "feature-branch"}
  ],
  "completed_retargets": 0
}"#,
    )
    .expect("push state should be written");

    let cached_plan_path = stck_dir.join("last-sync-plan.json");
    fs::write(
        &cached_plan_path,
        r#"{
  "default_branch": "main",
  "retargets": [
    {"branch": "feature-branch", "new_base_ref": "main"},
    {"branch": "feature-child", "new_base_ref": "feature-branch"}
  ]
}"#,
    )
    .expect("cached sync plan should be written");

    push.env("STCK_TEST_LOG", log_path.as_os_str());
    push.env("STCK_TEST_FEATURE_BRANCH_BASE", "main");
    push.arg("push");

    push.assert()
        .success()
        .stdout(predicate::str::contains(
            "Push succeeded. Pushed 0 branch(es) and applied 0 PR base update(s) in this run.",
        ))
        .stdout(predicate::str::contains("$ gh pr edit").not());

    if log_path.exists() {
        let log = fs::read_to_string(&log_path).expect("push log should be readable");
        assert!(
            !log.contains("pr edit"),
            "resume should skip retarget calls when saved retargets are already satisfied"
        );
    }
    assert!(
        !state_path.exists(),
        "push state should be cleared after a no-op resume succeeds"
    );
    assert!(
        !cached_plan_path.exists(),
        "cached sync plan should be cleared after a no-op resume succeeds"
    );
}

#[test]
fn push_skips_branches_without_local_changes() {
    let (temp, mut cmd) = stck_cmd_with_stubbed_tools();
    let log_path = log_path(&temp, "stck-push-no-divergence.log");
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
fn sync_then_push_after_squash_merge_produces_correct_retargets() {
    let (temp, mut sync) = stck_cmd_with_stubbed_tools();
    let log_path = log_path(&temp, "sync-push-squash.log");

    sync.env("STCK_TEST_LOG", log_path.as_os_str());
    sync.env(
        "STCK_TEST_NEEDS_PUSH_BRANCHES",
        "feature-branch,feature-child",
    );
    sync.arg("sync");
    sync.assert().success();

    let mut push = stck_cmd_for_temp(&temp);
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

    let mut push = stck_cmd_for_temp(&temp);
    push.arg("push");

    push.assert().code(1).stderr(predicate::str::contains(
        "error: sync operation state is in progress; run `stck sync --continue` before running push",
    ));
}
