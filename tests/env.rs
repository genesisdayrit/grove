mod common;

use common::*;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

/// Install `body` as this repo's executable `@env/setup`; returns its path.
fn write_setup(fx: &Fixture, body: &str) -> PathBuf {
    let dir = fx.repo_dir().join("@env");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("setup");
    std::fs::write(&path, format!("#!/bin/sh\n{body}")).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

/// `grove new <name> --env`, asserting success; returns the worktree path.
fn new_with_env(fx: &Fixture, name: &str) -> PathBuf {
    let out = fx.grove().args(["new", name, "--env"]).assert().success();
    PathBuf::from(stdout_of(&out).trim())
}

#[test]
fn new_with_env_runs_setup_in_the_worktree_with_grove_variables() {
    let fx = Fixture::new();
    write_setup(
        &fx,
        r#"{
  echo "cwd=$(pwd -P)"
  echo "repo=$GROVE_REPO_PATH"
  echo "worktree=$GROVE_WORKTREE_PATH"
  echo "name=$GROVE_WORKTREE_NAME"
  echo "branch=$GROVE_BRANCH"
} > ran.txt
"#,
    );

    let wt = new_with_env(&fx, "feat/auth");

    let ran = std::fs::read_to_string(wt.join("ran.txt")).unwrap();
    assert_eq!(
        ran,
        format!(
            "cwd={wt}\nrepo={clone}\nworktree={wt}\nname=feat-auth\nbranch=feat/auth\n",
            wt = wt.display(),
            clone = fx.clone.display()
        )
    );
}

#[test]
fn setup_output_goes_to_stderr_and_it_cannot_read_stdin() {
    let fx = Fixture::new();
    write_setup(
        &fx,
        "echo out-from-setup\necho err-from-setup >&2\ncat > stdin.txt\n",
    );

    let assert = fx
        .grove()
        .args(["new", "a", "--env"])
        .write_stdin("typed by the user\n")
        .assert()
        .success();

    let wt = fx.repo_dir().join("a");
    assert_eq!(stdout_of(&assert), format!("{}\n", wt.display()));
    let err = stderr_of(&assert);
    assert!(err.contains("out-from-setup"), "{err}");
    assert!(err.contains("err-from-setup"), "{err}");
    assert_eq!(std::fs::read_to_string(wt.join("stdin.txt")).unwrap(), "");
}

#[test]
fn failed_setup_keeps_the_worktree_warns_and_still_prints_the_path() {
    let fx = Fixture::new();
    write_setup(&fx, "exit 3\n");

    let assert = fx.grove().args(["new", "a", "--env"]).assert().success();

    let wt = fx.repo_dir().join("a");
    assert!(wt.join("README.md").is_file());
    assert_eq!(stdout_of(&assert), format!("{}\n", wt.display()));
    let err = stderr_of(&assert);
    assert!(err.contains("setup failed (exit 3)"), "{err}");
    assert!(err.contains("grove env setup"), "{err}");
}

#[test]
fn new_without_env_flag_skips_setup() {
    let fx = Fixture::new();
    write_setup(&fx, "touch ran.txt\n");

    fx.grove().args(["new", "a"]).assert().success();

    let wt = fx.repo_dir().join("a");
    assert!(wt.join("README.md").is_file());
    assert!(!wt.join("ran.txt").exists());
}

#[test]
fn env_setup_reruns_setup_for_the_current_worktree() {
    let fx = Fixture::new();
    let wt = fx.new_worktree("feat/auth");
    write_setup(
        &fx,
        "echo \"$(pwd -P) $GROVE_WORKTREE_NAME $GROVE_BRANCH\" > ran.txt\n",
    );
    let sub = wt.join("sub");
    std::fs::create_dir(&sub).unwrap();

    fx.grove_in(&sub)
        .args(["env", "setup"])
        .assert()
        .success()
        .stdout("");

    assert_eq!(
        std::fs::read_to_string(wt.join("ran.txt")).unwrap(),
        format!("{} feat-auth feat/auth\n", wt.display())
    );
}

#[test]
fn env_setup_fails_when_the_script_fails() {
    let fx = Fixture::new();
    let wt = fx.new_worktree("a");
    write_setup(&fx, "exit 4\n");

    fx.grove_in(&wt)
        .args(["env", "setup"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("setup failed (exit 4)"));
}

#[test]
fn env_setup_refuses_to_run_in_the_original_clone() {
    let fx = Fixture::new();
    write_setup(&fx, "touch ran.txt\n");

    fx.grove()
        .args(["env", "setup"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("grove worktree"));
    assert!(!fx.clone.join("ran.txt").exists());
}

#[test]
fn env_edit_with_piped_input_installs_an_executable_setup() {
    let fx = Fixture::new();
    let body = "#!/bin/sh\ntouch ran.txt\n";

    fx.grove()
        .args(["env", "edit"])
        .write_stdin(body)
        .assert()
        .success()
        .stdout("");

    let script = fx.repo_dir().join("@env/setup");
    assert_eq!(std::fs::read_to_string(&script).unwrap(), body);
    let mode = std::fs::metadata(&script).unwrap().permissions().mode();
    assert_eq!(mode & 0o111, 0o111, "{mode:o}");
    let wt = new_with_env(&fx, "a");
    assert!(wt.join("ran.txt").exists());
}

#[test]
fn env_edit_with_empty_piped_input_leaves_the_script_alone() {
    let fx = Fixture::new();
    let script = write_setup(&fx, "touch ran.txt\n");
    let before = std::fs::read_to_string(&script).unwrap();

    fx.grove()
        .args(["env", "edit"])
        .write_stdin("")
        .assert()
        .failure()
        .stderr(predicates::str::contains("nothing on stdin"));

    assert_eq!(std::fs::read_to_string(&script).unwrap(), before);
}

#[test]
fn env_status_shows_the_script_path_and_whether_it_exists() {
    let fx = Fixture::new();
    let script = fx.repo_dir().join("@env/setup");

    let missing = fx.grove().arg("env").assert().success();
    let out = stdout_of(&missing);
    assert!(out.contains(&script.display().to_string()), "{out}");
    assert!(out.contains("not set up"), "{out}");
    assert!(out.contains("grove env edit"), "{out}");

    write_setup(&fx, "true\n");
    let present = fx.grove().arg("env").assert().success();
    let out = stdout_of(&present);
    assert!(out.contains(&script.display().to_string()), "{out}");
    assert!(out.contains("grove new <name> --env"), "{out}");
}

#[test]
fn env_status_flags_a_script_that_is_not_executable() {
    let fx = Fixture::new();
    let script = write_setup(&fx, "true\n");
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o644)).unwrap();

    let out = stdout_of(&fx.grove().arg("env").assert().success());
    assert!(out.contains("not executable"), "{out}");
}

#[test]
fn new_with_env_but_no_script_fails_before_creating_anything() {
    let fx = Fixture::new();

    fx.grove()
        .args(["new", "a", "--env"])
        .assert()
        .failure()
        .stdout("")
        .stderr(predicates::str::contains("grove env edit"));

    assert!(!fx.repo_dir().join("a").exists());
    assert!(!fx.git(&fx.clone, &["branch", "--list", "a"]).contains('a'));
}
