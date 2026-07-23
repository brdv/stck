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
