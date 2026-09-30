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
        .args(["new", "b"])
        .assert()
        .success();
}

#[test]
fn second_clone_with_same_basename_is_refused_naming_the_owner() {
    let fx = Fixture::new();
    fx.new_worktree("a");
    let other = fx.base().join("elsewhere").join("myrepo");
    std::fs::create_dir_all(other.parent().unwrap()).unwrap();
    fx.git(
        &fx.base(),
        &[
            "clone",
            fx.origin.to_str().unwrap(),
            other.to_str().unwrap(),
        ],
    );

    fx.grove_in(&other)
        .args(["new", "b"])
        .assert()
        .failure()
        .stderr(predicates::str::contains(fx.clone.display().to_string()));
    assert!(!fx.repo_dir().join("b").exists());
}

#[test]
fn works_from_inside_a_grove_worktree() {
    let fx = Fixture::new();
    let a = fx.new_worktree("a");
    let assert = fx.grove_in(&a).args(["new", "b"]).assert().success();
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
    fx.grove_in(&sub).args(["new", "b"]).assert().success();
    assert!(fx.repo_dir().join("b").is_dir());
}

#[test]
fn works_from_the_repo_folder_under_the_root() {
    let fx = Fixture::new();
    fx.new_worktree("a");
    fx.grove_in(&fx.repo_dir())
        .args(["new", "b"])
        .assert()
        .success();
    assert!(fx.repo_dir().join("b").is_dir());
}

#[test]
fn outside_any_repo_is_an_error() {
    let fx = Fixture::new();
    fx.grove_in(&fx.home)
        .args(["new", "x"])
        .assert()
        .failure()
        .stdout("")
        .stderr(predicates::str::contains("not in a git repo"));
}

#[test]
fn root_defaults_to_dot_grove_in_home() {
    let fx = Fixture::new();
    fx.grove()
        .env_remove("GROVE_ROOT")
        .args(["new", "a"])
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
        .args(["new", "a"])
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
    fx.grove()
        .env_remove("GROVE_ROOT")
        .args(["new", "a"])
        .assert()
        .success();
    assert!(!fx.home.join(".config/grove").exists());
}
