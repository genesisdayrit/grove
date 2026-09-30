mod common;

use common::*;
use std::time::{Duration, SystemTime};

fn rows(out: &str) -> Vec<Vec<String>> {
    out.lines()
        .skip(1)
        .map(|l| {
            l.split("  ")
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(String::from)
                .collect()
        })
        .collect()
}

#[test]
fn ls_prints_a_table_of_this_repos_grove_worktrees_only() {
    let fx = Fixture::new();
    fx.new_worktree("alpha");
    fx.new_worktree("feat/auth");
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

    let out = stdout_of(&fx.grove().arg("ls").assert().success());

    let header: Vec<&str> = out.lines().next().unwrap().split_whitespace().collect();
    assert_eq!(
        header,
        ["NAME", "BRANCH", "CREATED", "LAST", "ACTIVE", "STATUS"]
    );
    let mut names: Vec<(String, String)> = rows(&out)
        .into_iter()
        .map(|r| (r[0].clone(), r[1].clone()))
        .collect();
    names.sort();
    assert_eq!(
        names,
        [
            ("alpha".into(), "alpha".into()),
            ("feat-auth".into(), "feat/auth".into())
        ]
    );
    assert!(!out.contains("foreign") && !out.contains("@repo"), "{out}");
}

#[test]
fn ls_marks_the_current_worktree() {
    let fx = Fixture::new();
    let a = fx.new_worktree("a");
    fx.new_worktree("b");
    let out = stdout_of(&fx.grove_in(&a.join(".")).arg("ls").assert().success());
    let marked: Vec<&str> = out.lines().filter(|l| l.starts_with('*')).collect();
    assert_eq!(marked.len(), 1, "{out}");
    assert!(marked[0].contains(" a "), "{out}");
}

#[test]
fn ls_status_counts_modified_staged_and_untracked_files() {
    let fx = Fixture::new();
    let dirty = fx.new_worktree("dirty");
    fx.new_worktree("tidy");
    std::fs::write(dirty.join("README.md"), "changed\n").unwrap();
    std::fs::write(dirty.join("new.txt"), "new\n").unwrap();
    std::fs::write(dirty.join("staged.txt"), "s\n").unwrap();
    fx.git(&dirty, &["add", "staged.txt"]);

    let out = stdout_of(&fx.grove().arg("ls").assert().success());
    let status = |name: &str| {
        rows(&out)
            .into_iter()
            .find(|r| r[0] == name)
            .unwrap()
            .last()
            .unwrap()
            .clone()
    };
    assert_eq!(status("dirty"), "3 changed");
    assert_eq!(status("tidy"), "clean");
}

#[test]
fn ls_sorts_by_last_active_using_commits_and_changed_file_mtimes() {
    let fx = Fixture::new();
    let a = fx.new_worktree("a");
    let b = fx.new_worktree("b");
    fx.new_worktree("c");

    // `a` gets a recent commit.
    std::fs::write(a.join("work.txt"), "w").unwrap();
    fx.git(&a, &["add", "."]);
    fx.git(&a, &["commit", "-m", "work"]);
    let names = |fx: &Fixture| -> Vec<String> {
        rows(&stdout_of(&fx.grove().arg("ls").assert().success()))
            .into_iter()
            .map(|r| r[0].clone())
            .collect()
    };
    assert_eq!(names(&fx)[0], "a");

    // `b` gets an uncommitted edit that is newer still.
    let f = b.join("scratch.txt");
    std::fs::write(&f, "s").unwrap();
    std::fs::File::options()
        .write(true)
        .open(&f)
        .unwrap()
        .set_modified(SystemTime::now() + Duration::from_secs(3600))
        .unwrap();
    assert_eq!(&names(&fx)[..2], ["b", "a"]);
}

#[test]
fn ls_paths_prints_one_path_per_line() {
    let fx = Fixture::new();
    let a = fx.new_worktree("a");
    let b = fx.new_worktree("b");
    let out = stdout_of(&fx.grove().args(["ls", "--paths"]).assert().success());
    let mut lines: Vec<&str> = out.lines().collect();
    lines.sort();
    assert_eq!(lines, [a.to_str().unwrap(), b.to_str().unwrap()]);
}

#[test]
fn ls_with_no_worktrees_prints_nothing_to_stdout() {
    let fx = Fixture::new();
    fx.grove()
        .args(["ls", "--paths"])
        .assert()
        .success()
        .stdout("");
}

#[test]
fn ls_outside_a_repo_fails() {
    let fx = Fixture::new();
    fx.grove_in(&fx.home).arg("ls").assert().failure();
}
