mod common;

use common::*;

#[test]
fn new_creates_branch_and_worktree_and_prints_only_the_path() {
    let fx = Fixture::new();
    let assert = fx.grove().args(["new", "my-feature"]).assert().success();

    let expected = fx.repo_dir().join("my-feature");
    assert_eq!(stdout_of(&assert), format!("{}\n", expected.display()));
    assert!(expected.join("README.md").exists());
    assert_eq!(
        fx.git(&expected, &["branch", "--show-current"]),
        "my-feature"
    );
    let err = stderr_of(&assert);
    assert!(err.contains("branch: my-feature"), "stderr: {err}");
    assert!(err.contains("folder: my-feature"), "stderr: {err}");
}

#[test]
fn new_branches_from_freshly_fetched_origin_without_touching_local_main() {
    let fx = Fixture::new();
    let local_main_before = fx.git(&fx.clone, &["rev-parse", "main"]);
    let upstream = fx.advance_origin("main");

    let wt = fx.new_worktree("fresh");

    assert_eq!(fx.git(&wt, &["rev-parse", "HEAD"]), upstream);
    assert_eq!(fx.git(&fx.clone, &["rev-parse", "main"]), local_main_before);
    assert_eq!(fx.git(&fx.clone, &["branch", "--show-current"]), "main");
}

#[test]
fn new_detects_default_branch_from_origin_head() {
    let fx = Fixture::with_default_branch("trunk");
    let upstream = fx.advance_origin("trunk");

    let wt = fx.new_worktree("x");

    assert_eq!(fx.git(&wt, &["rev-parse", "HEAD"]), upstream);
}

#[test]
fn new_branch_does_not_track_the_default_branch() {
    let fx = Fixture::new();
    let wt = fx.new_worktree("solo");
    let upstream = common_git_ok(&fx, &wt, &["rev-parse", "--abbrev-ref", "@{upstream}"]);
    assert!(!upstream, "new branch should have no upstream");
}

fn common_git_ok(fx: &Fixture, dir: &std::path::Path, args: &[&str]) -> bool {
    std::process::Command::new("git")
        .current_dir(dir)
        .args(args)
        .env("HOME", &fx.home)
        .output()
        .unwrap()
        .status
        .success()
}

#[test]
fn new_when_offline_warns_and_falls_back_to_local_default_branch() {
    let fx = Fixture::new();
    fx.git(
        &fx.clone,
        &["remote", "set-url", "origin", "/nonexistent/origin.git"],
    );
    std::fs::write(fx.clone.join("local.txt"), "local only").unwrap();
    fx.git(&fx.clone, &["add", "."]);
    fx.git(&fx.clone, &["commit", "-m", "local work"]);
    let local_main = fx.git(&fx.clone, &["rev-parse", "main"]);

    let assert = fx.grove().args(["new", "offline"]).assert().success();

    let wt = fx.repo_dir().join("offline");
    assert_eq!(fx.git(&wt, &["rev-parse", "HEAD"]), local_main);
    assert!(
        stderr_of(&assert).contains("warning"),
        "{}",
        stderr_of(&assert)
    );
}

#[test]
fn new_without_any_remote_uses_local_default_branch() {
    let fx = Fixture::new();
    fx.git(&fx.clone, &["remote", "remove", "origin"]);
    let local_main = fx.git(&fx.clone, &["rev-parse", "main"]);

    let assert = fx.grove().args(["new", "lonely"]).assert().success();

    assert_eq!(
        fx.git(&fx.repo_dir().join("lonely"), &["rev-parse", "HEAD"]),
        local_main
    );
    assert!(stderr_of(&assert).contains("warning"));
}
