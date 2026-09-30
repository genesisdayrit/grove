mod common;

use common::*;

#[test]
fn repo_add_creates_the_folder_and_link_without_a_worktree() {
    let fx = Fixture::unregistered("main");
    let assert = fx
        .grove()
        .args(["repo", "add"])
        .assert()
        .success()
        .stdout("");

    assert!(stderr_of(&assert).contains("registered myrepo"));
    assert_eq!(
        fx.repo_dir().join("@repo").canonicalize().unwrap(),
        fx.clone
    );
    assert!(fx.root.join(".metadata_never_index").is_file());
    let entries: Vec<_> = std::fs::read_dir(fx.repo_dir()).unwrap().collect();
    assert_eq!(entries.len(), 1, "only @repo: {entries:?}");
    assert!(fx.git(&fx.clone, &["worktree", "list"]).lines().count() == 1);
}

#[test]
fn repo_add_twice_is_harmless() {
    let fx = Fixture::new();
    fx.grove()
        .args(["repo", "add"])
        .assert()
        .success()
        .stderr(predicates::str::contains("already registered"));
}

#[test]
fn repo_add_takes_a_path_and_resolves_worktrees_to_their_clone() {
    let fx = Fixture::new();
    let other = fx.another_clone("oss", "other");
    fx.grove_in(&fx.home)
        .args(["repo", "add", other.to_str().unwrap()])
        .assert()
        .success();
    assert_eq!(fx.root.join("other/@repo").canonicalize().unwrap(), other);

    let wt = fx.new_worktree("a");
    fx.grove_in(&fx.home)
        .args(["repo", "add", wt.to_str().unwrap()])
        .assert()
        .success()
        .stderr(predicates::str::contains("myrepo already registered"));
}

#[test]
fn repo_add_name_normalizes_and_refuses_a_second_name_for_the_same_clone() {
    let fx = Fixture::unregistered("main");
    fx.grove()
        .args(["repo", "add", "--name", "My App"])
        .assert()
        .success();
    assert!(fx.root.join("my-app/@repo").exists());

    fx.grove()
        .args(["repo", "add", "--name", "other"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("grove repo mv other"));
}

#[test]
fn repo_add_outside_git_or_over_a_stray_folder_fails() {
    let fx = Fixture::unregistered("main");
    fx.grove_in(&fx.home)
        .args(["repo", "add"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("not inside a git clone"));

    std::fs::create_dir_all(fx.repo_dir()).unwrap();
    fx.grove()
        .args(["repo", "add"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("isn't a grove repo folder"));
}

#[test]
fn unregistered_repos_are_gated() {
    let fx = Fixture::unregistered("main");
    for args in [
        vec!["worktree", "add", "a"],
        vec!["env", "edit"],
        vec!["env", "setup"],
        vec!["cd", "a"],
    ] {
        fx.grove()
            .args(&args)
            .write_stdin("#!/bin/sh\n")
            .assert()
            .failure()
            .stdout("")
            .stderr(predicates::str::contains(
                "myrepo isn't set up with grove yet. Run: grove repo add",
            ));
    }
    assert!(!fx.root.exists());
    assert!(!fx.git(&fx.clone, &["branch", "--list", "a"]).contains('a'));
}

#[test]
fn repo_flag_works_from_anywhere() {
    let fx = Fixture::new();
    let assert = fx
        .grove_in(&fx.home)
        .args(["worktree", "add", "far", "--repo", "myrepo"])
        .assert()
        .success();
    let wt = fx.repo_dir().join("far");
    assert_eq!(stdout_of(&assert).trim(), wt.display().to_string());
    fx.grove_in(&fx.home)
        .args(["-r", "myrepo", "cd", "far"])
        .assert()
        .success()
        .stdout(format!("{}\n", wt.display()));
}

#[test]
fn unknown_repo_lists_the_registered_ones() {
    let fx = Fixture::new();
    fx.grove_in(&fx.home)
        .args(["cd", "x", "--repo", "nope"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("registered: myrepo"));
}

#[test]
fn env_setup_rejects_repo_flag_and_all_is_only_for_listing() {
    let fx = Fixture::new();
    fx.grove()
        .args(["env", "setup", "--repo", "myrepo"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("drop --repo"));
    fx.grove()
        .args(["cd", "x", "--all"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("--all only applies"));
}

#[test]
fn repo_ls_shows_each_repo_and_flags_missing_clones() {
    let fx = Fixture::new();
    fx.new_worktree("a");
    fx.new_worktree("b");
    let other = fx.another_clone("oss", "other");
    fx.grove_in(&other).args(["repo", "add"]).assert().success();
    let gone = fx.another_clone("tmp", "gone");
    fx.grove_in(&gone).args(["repo", "add"]).assert().success();
    std::fs::remove_dir_all(&gone).unwrap();

    let out = stdout_of(&fx.grove().args(["repo", "ls"]).assert().success());
    let header: Vec<&str> = out.lines().next().unwrap().split_whitespace().collect();
    assert_eq!(
        header,
        ["NAME", "CLONE", "WORKTREES", "SETUP", "LAST", "ACTIVE"]
    );
    let line = |name: &str| {
        out.lines()
            .find(|l| l.split_whitespace().any(|w| w == name))
            .unwrap_or_else(|| panic!("{name} missing: {out}"))
            .to_string()
    };
    assert!(line("myrepo").starts_with('*'), "{out}");
    assert!(line("myrepo").contains(" 2 "), "{out}");
    assert!(line("other").contains(" 0 "), "{out}");
    assert!(line("gone").contains("(missing)"), "{out}");

    let names = stdout_of(
        &fx.grove_in(&fx.home)
            .args(["repo", "ls", "--names"])
            .assert()
            .success(),
    );
    assert_eq!(names, "gone\nmyrepo\nother\n");
}

#[test]
fn bare_repo_prints_help() {
    let fx = Fixture::new();
    fx.grove()
        .arg("repo")
        .assert()
        .failure()
        .stderr(predicates::str::contains("add"));
}

#[test]
fn repo_mv_moves_worktrees_and_keeps_git_happy() {
    let fx = Fixture::new();
    let a = fx.new_worktree("a");
    std::fs::create_dir_all(fx.repo_dir().join("@env")).unwrap();
    std::fs::write(fx.repo_dir().join("@env/setup"), "#!/bin/sh\n").unwrap();

    fx.grove()
        .args(["repo", "mv", "Renamed"])
        .assert()
        .success()
        .stdout("");

    let moved = fx.root.join("renamed/a");
    assert!(!a.exists());
    assert!(moved.join("README.md").is_file());
    assert!(fx.root.join("renamed/@env/setup").is_file());
    assert_eq!(fx.git(&moved, &["branch", "--show-current"]), "a");
    assert!(
        fx.git(&fx.clone, &["worktree", "list"])
            .contains(&moved.display().to_string())
    );
    fx.grove()
        .args(["cd", "a"])
        .assert()
        .success()
        .stdout(format!("{}\n", moved.display()));
}

#[test]
fn repo_mv_from_inside_prints_where_you_now_are() {
    let fx = Fixture::new();
    let a = fx.new_worktree("a");
    let assert = fx
        .grove_in(&a)
        .args(["repo", "mv", "renamed"])
        .assert()
        .success();
    assert_eq!(
        stdout_of(&assert).trim(),
        fx.root.join("renamed/a").display().to_string()
    );
}

#[test]
fn repo_mv_refuses_taken_names_and_locked_worktrees() {
    let fx = Fixture::new();
    let a = fx.new_worktree("a");
    let other = fx.another_clone("oss", "other");
    fx.grove_in(&other).args(["repo", "add"]).assert().success();

    fx.grove()
        .args(["repo", "mv", "other"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("already exists"));

    fx.git(&fx.clone, &["worktree", "lock", a.to_str().unwrap()]);
    fx.grove()
        .args(["repo", "mv", "fresh"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("locked"));
    assert!(a.is_dir());
    assert!(!fx.root.join("fresh").exists());
}
