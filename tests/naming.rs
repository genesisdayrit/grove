mod common;

use common::*;

#[test]
fn name_is_lowercased_and_invalid_chars_become_single_dashes() {
    let fx = Fixture::new();
    let assert = fx
        .grove()
        .args(["new", "  Fix Login!!  Redirect "])
        .assert()
        .success();

    let wt = fx.repo_dir().join("fix-login-redirect");
    assert_eq!(stdout_of(&assert).trim(), wt.display().to_string());
    assert_eq!(
        fx.git(&wt, &["branch", "--show-current"]),
        "fix-login-redirect"
    );
}

#[test]
fn slash_is_kept_in_branch_but_flattened_in_folder() {
    let fx = Fixture::new();
    let assert = fx.grove().args(["new", "Feat/Auth"]).assert().success();

    let wt = fx.repo_dir().join("feat-auth");
    assert!(wt.is_dir());
    assert_eq!(fx.git(&wt, &["branch", "--show-current"]), "feat/auth");
    let err = stderr_of(&assert);
    assert!(
        err.contains("branch: feat/auth") && err.contains("folder: feat-auth"),
        "{err}"
    );
}

#[test]
fn names_starting_with_at_are_rejected() {
    let fx = Fixture::new();
    fx.grove()
        .args(["new", "@repo"])
        .assert()
        .failure()
        .stdout("")
        .stderr(predicates::str::contains("reserved"));
}

#[test]
fn names_that_normalize_to_nothing_are_rejected() {
    let fx = Fixture::new();
    fx.grove()
        .args(["new", "!!!"])
        .assert()
        .failure()
        .stdout("");
}

#[test]
fn existing_folder_fails_with_its_path() {
    let fx = Fixture::new();
    let wt = fx.new_worktree("dup");
    fx.grove()
        .args(["new", "dup"])
        .assert()
        .failure()
        .stdout("")
        .stderr(predicates::str::contains(wt.display().to_string()));
}

#[test]
fn flattened_folder_clash_fails() {
    let fx = Fixture::new();
    fx.new_worktree("feat/auth");
    fx.grove()
        .args(["new", "feat-auth"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("feat-auth"));
}

#[test]
fn existing_local_branch_fails() {
    let fx = Fixture::new();
    fx.git(&fx.clone, &["branch", "taken"]);
    fx.grove()
        .args(["new", "taken"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("branch `taken` already exists"));
    assert!(!fx.repo_dir().join("taken").exists());
}

#[test]
fn branch_existing_only_on_origin_fails() {
    let fx = Fixture::new();
    let seedless = fx.base().join("pusher");
    fx.git(
        &fx.base(),
        &[
            "clone",
            fx.origin.to_str().unwrap(),
            seedless.to_str().unwrap(),
        ],
    );
    fx.git(
        &seedless,
        &["push", "origin", "HEAD:refs/heads/remote-only"],
    );

    fx.grove()
        .args(["new", "remote-only"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("origin/remote-only"));
    assert!(!fx.repo_dir().join("remote-only").exists());
}

#[test]
fn git_ref_hierarchy_conflict_surfaces_gits_error() {
    let fx = Fixture::new();
    fx.git(&fx.clone, &["branch", "feat"]);
    fx.grove()
        .args(["new", "feat/auth"])
        .assert()
        .failure()
        .stdout("")
        .stderr(predicates::str::contains("refs/heads/feat"));
    assert!(!fx.repo_dir().join("feat-auth").exists());
}
