mod common;

use common::*;

#[test]
fn root_gets_spotlight_marker_and_repo_link() {
    let fx = Fixture::new();
    fx.new_worktree("a");

    assert!(fx.root.join(".metadata_never_index").is_file());
    let link = fx.repo_dir().join("@repo");
    assert!(
        std::fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(link.canonicalize().unwrap(), fx.clone);
    // Siblings and the original clone are reachable from a worktree.
    assert!(fx.repo_dir().join("a").join("../@repo/README.md").exists());
}

#[test]
fn repo_link_is_created_once_and_reused() {
    let fx = Fixture::new();
    fx.new_worktree("a");
    fx.new_worktree("b");
    assert_eq!(
        fx.repo_dir().join("@repo").canonicalize().unwrap(),
        fx.clone
    );
}

#[test]
fn repo_link_through_symlinked_path_is_accepted() {
    // Same clone reached via a symlinked path (like macOS /tmp -> /private/tmp).
    let fx = Fixture::new();
    fx.new_worktree("a");
    let alias = fx.base().join("alias");
    std::os::unix::fs::symlink(fx.clone.parent().unwrap(), &alias).unwrap();
    fx.grove_in(&alias.join("myrepo"))
        .args(["worktree", "add", "b"])
        .assert()
        .success();
}

#[test]
fn second_clone_with_same_basename_needs_its_own_name() {
    let fx = Fixture::new();
    fx.new_worktree("a");
    let other = fx.another_clone("elsewhere", "myrepo");

    let assert = fx.grove_in(&other).args(["repo", "add"]).assert().failure();
    let err = stderr_of(&assert);
    assert!(err.contains(&fx.clone.display().to_string()), "{err}");
    assert!(err.contains("--name elsewhere-myrepo"), "{err}");

    fx.grove_in(&other)
        .args(["repo", "add", "--name", "elsewhere-myrepo"])
        .assert()
        .success();
    let wt = stdout_of(
        &fx.grove_in(&other)
            .args(["worktree", "add", "b"])
            .assert()
            .success(),
    );
    assert_eq!(
        wt.trim(),
        fx.root.join("elsewhere-myrepo/b").display().to_string()
    );
    assert!(!fx.repo_dir().join("b").exists());
}

#[test]
fn works_from_inside_a_grove_worktree() {
    let fx = Fixture::new();
    let a = fx.new_worktree("a");
    let assert = fx
        .grove_in(&a)
        .args(["worktree", "add", "b"])
        .assert()
        .success();
    assert_eq!(
        stdout_of(&assert).trim(),
        fx.repo_dir().join("b").display().to_string()
    );
}

#[test]
fn works_from_a_subdirectory_of_the_clone() {
    let fx = Fixture::new();
    let sub = fx.clone.join("deep/er");
    std::fs::create_dir_all(&sub).unwrap();
    fx.grove_in(&sub)
        .args(["worktree", "add", "b"])
        .assert()
        .success();
    assert!(fx.repo_dir().join("b").is_dir());
}

#[test]
fn works_from_the_repo_folder_under_the_root() {
    let fx = Fixture::new();
    fx.new_worktree("a");
    fx.grove_in(&fx.repo_dir())
        .args(["worktree", "add", "b"])
        .assert()
        .success();
    assert!(fx.repo_dir().join("b").is_dir());
}

#[test]
fn outside_any_repo_is_an_error() {
    let fx = Fixture::new();
    fx.grove_in(&fx.home)
        .args(["worktree", "add", "x"])
        .assert()
        .failure()
        .stdout("")
        .stderr(predicates::str::contains("not in a registered repo"));
}

#[test]
fn root_defaults_to_dot_grove_in_home() {
    let fx = Fixture::new();
    fx.grove()
        .env_remove("GROVE_ROOT")
        .args(["repo", "add"])
        .assert()
        .success();
    fx.grove()
        .env_remove("GROVE_ROOT")
        .args(["worktree", "add", "a"])
        .assert()
        .success();
    assert!(fx.home.join(".grove/myrepo/a").is_dir());
}

#[test]
fn root_comes_from_xdg_config_when_env_is_unset() {
    let fx = Fixture::new();
    let cfg = fx.home.join(".config/grove");
    std::fs::create_dir_all(&cfg).unwrap();
    std::fs::write(cfg.join("config.toml"), "root = \"~/trees\"\n").unwrap();

    fx.grove()
        .env_remove("GROVE_ROOT")
        .args(["repo", "add"])
        .assert()
        .success();
    fx.grove()
        .env_remove("GROVE_ROOT")
        .args(["worktree", "add", "a"])
        .assert()
        .success();
    assert!(fx.home.join("trees/myrepo/a").is_dir());
}

#[test]
fn grove_root_env_beats_config() {
    let fx = Fixture::new();
    let cfg = fx.home.join(".config/grove");
    std::fs::create_dir_all(&cfg).unwrap();
    std::fs::write(cfg.join("config.toml"), "root = \"~/trees\"\n").unwrap();

    fx.new_worktree("a");
    assert!(fx.repo_dir().join("a").is_dir());
    assert!(!fx.home.join("trees").exists());
}

#[test]
fn config_file_is_never_created() {
    let fx = Fixture::new();
    for args in [vec!["repo", "add"], vec!["worktree", "add", "a"]] {
        fx.grove()
            .env_remove("GROVE_ROOT")
            .args(args)
            .assert()
            .success();
    }
    assert!(!fx.home.join(".config/grove").exists());
}

#[test]
fn relative_grove_root_is_resolved_against_the_working_directory() {
    let fx = Fixture::new();
    fx.grove()
        .env("GROVE_ROOT", "../trees")
        .args(["repo", "add"])
        .assert()
        .success();
    let assert = fx
        .grove()
        .env("GROVE_ROOT", "../trees")
        .args(["worktree", "add", "a"])
        .assert()
        .success();
    let expected = fx.clone.parent().unwrap().join("trees/myrepo/a");
    assert_eq!(stdout_of(&assert).trim(), expected.display().to_string());
    fx.grove_in(&expected)
        .env("GROVE_ROOT", "../../../trees")
        .args(["cd", "a"])
        .assert()
        .success()
        .stdout(format!("{}\n", expected.display()));
}
