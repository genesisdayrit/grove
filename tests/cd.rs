mod common;

use common::*;

#[test]
fn cd_prints_the_worktree_path() {
    let fx = Fixture::new();
    let wt = fx.new_worktree("alpha");
    fx.grove()
        .args(["cd", "alpha"])
        .assert()
        .success()
        .stdout(format!("{}\n", wt.display()));
}

#[test]
fn cd_accepts_the_branch_spelling_of_a_flattened_folder() {
    let fx = Fixture::new();
    let wt = fx.new_worktree("feat/auth");
    fx.grove()
        .args(["cd", "feat/auth"])
        .assert()
        .success()
        .stdout(format!("{}\n", wt.display()));
}

#[test]
fn cd_works_from_a_sibling_worktree() {
    let fx = Fixture::new();
    let a = fx.new_worktree("a");
    let b = fx.new_worktree("b");
    fx.grove_in(&a)
        .args(["cd", "b"])
        .assert()
        .success()
        .stdout(format!("{}\n", b.display()));
}

#[test]
fn cd_without_exact_match_fails_and_lists_close_matches() {
    let fx = Fixture::new();
    fx.new_worktree("login-fix");
    fx.new_worktree("unrelated");
    let assert = fx
        .grove()
        .args(["cd", "login"])
        .assert()
        .failure()
        .stdout("");
    let err = stderr_of(&assert);
    assert!(err.contains("login-fix"), "{err}");
    assert!(!err.contains("unrelated"), "{err}");
}

#[test]
fn cd_ignores_worktrees_not_created_by_grove() {
    let fx = Fixture::new();
    fx.new_worktree("mine");
    let foreign = fx.base().join("foreign");
    fx.git(
        &fx.clone,
        &[
            "worktree",
            "add",
            "-b",
            "foreign",
            foreign.to_str().unwrap(),
        ],
    );
    fx.grove()
        .args(["cd", "foreign"])
        .assert()
        .failure()
        .stdout("");
}

#[test]
fn cd_never_targets_grove_managed_at_entries() {
    let fx = Fixture::new();
    fx.new_worktree("mine");
    fx.grove()
        .args(["cd", "@repo"])
        .assert()
        .failure()
        .stdout("");
}
