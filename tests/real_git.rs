mod harness;

use harness::RealGitRepo;
use predicates::prelude::*;

#[test]
fn new_creates_and_publishes_a_branch_with_real_git() {
    let repo = RealGitRepo::new();
    let mut cmd = repo.stck_cmd();
    cmd.args(["new", "feature-new"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("$ git checkout -b feature-new"))
        .stdout(predicate::str::contains("$ git push -u origin feature-new"))
        .stdout(predicate::str::contains(
            "No branch-only commits in feature-new yet.",
        ));

    assert_eq!(repo.current_branch(), "feature-new");
    assert_eq!(
        repo.local_sha("refs/heads/feature-new"),
        repo.remote_sha("feature-new")
    );
    assert!(
        !repo.gh_log().contains("pr create"),
        "new should wait for a branch-only commit before creating a PR"
    );
}

#[test]
fn submit_pushes_a_real_branch_before_creating_its_pr() {
    let repo = RealGitRepo::new();
    repo.create_branch("feature-submit");
    repo.commit_file("feature.txt", "submitted\n", "Add submitted feature");

    let mut cmd = repo.stck_cmd();
    cmd.args(["submit", "--base", "main"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains(
            "$ git push -u origin feature-submit",
        ))
        .stdout(predicate::str::contains(
            "$ gh pr create --base main --head feature-submit",
        ))
        .stdout(predicate::str::contains(
            "Created PR for feature-submit targeting main.",
        ));

    assert_eq!(
        repo.local_sha("refs/heads/feature-submit"),
        repo.remote_sha("feature-submit")
    );
    let gh_log = repo.gh_log();
    assert!(gh_log.contains("pr view feature-submit --json number"));
    assert!(gh_log.contains("pr create --base main --head feature-submit"));
    assert!(gh_log.contains(
        "This pull request is part of a stack.\n\n- **Position:** Root\n- **Base:** `main`"
    ));
}

#[test]
fn submit_discovers_a_remote_parent_without_its_local_branch() {
    let repo = RealGitRepo::new();
    repo.create_branch("feature-base");
    repo.commit_file("base.txt", "base\n", "Add base feature");
    repo.push("feature-base");

    repo.create_branch("feature-child");
    repo.commit_file("child.txt", "child\n", "Add child feature");
    repo.delete_local_branch("feature-base");
    repo.write_open_pr_head_response(
        "feature-base",
        r#"[{"headRefName":"feature-base","isCrossRepository":false}]"#,
    );

    let mut cmd = repo.stck_cmd();
    cmd.arg("submit");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains(
            "No --base provided. Detected stack parent: feature-base.",
        ))
        .stdout(predicate::str::contains(
            "$ gh pr create --base feature-base --head feature-child",
        ));

    let gh_log = repo.gh_log();
    assert!(gh_log.contains(
        "pr list --head feature-base --state open --limit 1 --json headRefName,isCrossRepository"
    ));
    assert!(
        !gh_log.contains("pr list --state open --limit 100"),
        "parent discovery must not depend on a bounded repository-wide PR scan"
    );
    assert!(gh_log.contains("pr create --base feature-base --head feature-child"));
    assert!(gh_log.contains(
        "This pull request is part of a stack.\n\n- **Position:** Child\n- **Base:** `feature-base`"
    ));
}

#[test]
fn status_and_push_handle_a_missing_remote_branch() {
    let repo = RealGitRepo::new();
    repo.create_branch("feature-missing-remote");
    repo.commit_file("feature.txt", "feature\n", "Add local feature");
    repo.write_pr_response(
        "feature-missing-remote",
        r#"{"number":103,"headRefName":"feature-missing-remote","baseRefName":"main","state":"OPEN"}"#,
    );
    repo.write_children_response("feature-missing-remote", "[]");

    let mut status = repo.stck_cmd();
    status.arg("status");
    status
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "* feature-missing-remote PR #103 OPEN base=main [needs_push]",
        ))
        .stdout(predicate::str::contains(
            "Summary: 0 needs_sync, 1 needs_push, 0 base_mismatch",
        ));

    let mut push = repo.stck_cmd();
    push.arg("push");
    push.assert()
        .success()
        .stdout(predicate::str::contains(
            "$ git push --force-with-lease origin feature-missing-remote",
        ))
        .stdout(predicate::str::contains(
            "Push succeeded. Pushed 1 branch(es) and applied 0 PR base update(s) in this run.",
        ));

    assert_eq!(
        repo.local_sha("refs/heads/feature-missing-remote"),
        repo.remote_sha("feature-missing-remote")
    );
}

#[test]
fn sync_rebases_a_real_linear_stack_after_main_advances() {
    let repo = RealGitRepo::new();

    repo.create_branch("feature-base");
    repo.commit_file("base.txt", "base\n", "Add base feature");
    repo.push("feature-base");
    let old_base_sha = repo.remote_sha("feature-base");

    repo.create_branch("feature-child");
    repo.commit_file("child.txt", "child\n", "Add child feature");
    repo.push("feature-child");
    let old_child_sha = repo.remote_sha("feature-child");

    repo.checkout("main");
    repo.commit_file("main.txt", "main advanced\n", "Advance main");
    repo.push("main");
    repo.checkout("feature-base");

    repo.write_pr_response(
        "feature-base",
        r#"{"number":101,"headRefName":"feature-base","baseRefName":"main","state":"OPEN"}"#,
    );
    repo.write_pr_response(
        "feature-child",
        r#"{"number":102,"headRefName":"feature-child","baseRefName":"feature-base","state":"OPEN"}"#,
    );
    repo.write_children_response(
        "feature-base",
        r#"[{"number":102,"headRefName":"feature-child","baseRefName":"feature-base","state":"OPEN"}]"#,
    );
    repo.write_children_response("feature-child", "[]");

    let mut cmd = repo.stck_cmd();
    cmd.arg("sync");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains(
            "$ git rebase --onto refs/remotes/origin/main",
        ))
        .stdout(predicate::str::contains(
            "$ git rebase --onto refs/heads/feature-base",
        ))
        .stdout(predicate::str::contains(
            "Sync succeeded locally. Run `stck push` to update remotes + PR bases.",
        ));

    assert_eq!(repo.current_branch(), "feature-base");
    assert!(repo.is_ancestor("refs/remotes/origin/main", "refs/heads/feature-base"));
    assert!(repo.is_ancestor("refs/heads/feature-base", "refs/heads/feature-child"));
    assert_ne!(repo.local_sha("refs/heads/feature-base"), old_base_sha);
    assert_ne!(repo.local_sha("refs/heads/feature-child"), old_child_sha);
    assert_eq!(repo.remote_sha("feature-base"), old_base_sha);
    assert_eq!(repo.remote_sha("feature-child"), old_child_sha);
}

#[test]
fn push_does_not_reuse_a_real_cached_plan_for_another_stack() {
    let repo = RealGitRepo::new();

    repo.create_branch("stack-a-base");
    repo.commit_file("stack-a-base.txt", "base a\n", "Add stack A base");
    repo.push("stack-a-base");
    repo.create_branch("stack-a-child");
    repo.commit_file("stack-a-child.txt", "child a\n", "Add stack A child");
    repo.push("stack-a-child");

    repo.checkout("main");
    repo.commit_file("main-ahead.txt", "main ahead\n", "Advance main");
    repo.push("main");
    repo.checkout("stack-a-base");

    repo.write_pr_response(
        "stack-a-base",
        r#"{"number":201,"headRefName":"stack-a-base","baseRefName":"main","state":"OPEN"}"#,
    );
    repo.write_pr_response(
        "stack-a-child",
        r#"{"number":202,"headRefName":"stack-a-child","baseRefName":"stack-a-base","state":"OPEN"}"#,
    );
    repo.write_children_response(
        "stack-a-base",
        r#"[{"number":202,"headRefName":"stack-a-child","baseRefName":"stack-a-base","state":"OPEN"}]"#,
    );
    repo.write_children_response("stack-a-child", "[]");

    let mut sync = repo.stck_cmd();
    sync.arg("sync");
    sync.assert().success();

    repo.checkout("main");
    repo.create_branch("stack-b-base");
    repo.commit_file("stack-b-base.txt", "base b\n", "Add stack B base");
    repo.push("stack-b-base");
    repo.create_branch("stack-b-child");
    repo.commit_file("stack-b-child.txt", "child b\n", "Add stack B child");
    repo.push("stack-b-child");

    repo.write_pr_response(
        "stack-b-base",
        r#"{"number":301,"headRefName":"stack-b-base","baseRefName":"main","state":"OPEN"}"#,
    );
    repo.write_pr_response(
        "stack-b-child",
        r#"{"number":302,"headRefName":"stack-b-child","baseRefName":"stack-b-base","state":"OPEN"}"#,
    );
    repo.write_children_response("stack-b-child", "[]");

    let mut push = repo.stck_cmd();
    push.arg("push");
    push.assert()
        .success()
        .stdout(predicate::str::contains(
            "Push succeeded. Pushed 0 branch(es) and applied 0 PR base update(s) in this run.",
        ))
        .stdout(predicate::str::contains("$ gh pr edit").not());

    let gh_log = repo.gh_log();
    assert!(
        !gh_log.contains("pr edit stack-a-base") && !gh_log.contains("pr edit stack-a-child"),
        "push for stack B must not apply stack A's cached retarget plan"
    );
}
