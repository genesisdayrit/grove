mod common;

use common::*;
use std::process::Command;

#[test]
fn init_prints_a_shell_function_for_zsh_and_bash() {
    let fx = Fixture::new();
    for shell in ["zsh", "bash"] {
        let out = stdout_of(
            &fx.grove_in(&fx.home)
                .args(["init", shell])
                .assert()
                .success(),
        );
        assert!(out.contains("grove()"), "{shell}: {out}");
        assert!(out.contains("cd "), "{shell}: {out}");
    }
}

#[test]
fn init_rejects_unsupported_shells() {
    let fx = Fixture::new();
    fx.grove_in(&fx.home)
        .args(["init", "fish"])
        .assert()
        .failure()
        .stdout("");
}

#[test]
fn without_shell_integration_navigation_prints_path_plus_hint() {
    let fx = Fixture::new();
    fx.new_worktree("a");
    fx.grove()
        .args(["cd", "a"])
        .assert()
        .success()
        .stderr(predicates::str::contains("grove init"));
}

#[test]
fn picker_without_a_terminal_fails_cleanly() {
    let fx = Fixture::new();
    fx.new_worktree("a");
    fx.grove()
        .assert()
        .failure()
        .stdout("")
        .stderr(predicates::str::contains("grove cd"));
}

#[test]
fn picker_with_no_worktrees_says_how_to_make_one() {
    let fx = Fixture::new();
    fx.grove()
        .assert()
        .failure()
        .stdout("")
        .stderr(predicates::str::contains("grove new"));
}

/// Run a script in a real shell that has `eval "$(grove init <shell>)"` loaded.
fn in_shell(fx: &Fixture, shell: &str, script: &str) -> std::process::Output {
    let bin = assert_cmd::cargo::cargo_bin("grove");
    let path = format!(
        "{}:{}",
        bin.parent().unwrap().display(),
        std::env::var("PATH").unwrap()
    );
    Command::new(shell)
        .arg("-c")
        .arg(format!(
            "eval \"$(grove init {shell})\"\ncd '{}'\n{script}",
            fx.clone.display()
        ))
        .env_clear()
        .env("PATH", path)
        .env("HOME", &fx.home)
        .env("GIT_CONFIG_GLOBAL", fx.home.join(".gitconfig"))
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GROVE_ROOT", &fx.root)
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap()
}

fn shell_available(shell: &str) -> bool {
    Command::new(shell)
        .arg("-c")
        .arg("true")
        .output()
        .is_ok_and(|o| o.status.success())
}

#[test]
fn wrapper_cds_into_new_and_cd_targets_and_passes_ls_through() {
    for shell in ["zsh", "bash"] {
        if !shell_available(shell) {
            eprintln!("skipping {shell}: not installed");
            continue;
        }
        let fx = Fixture::new();
        let out = in_shell(
            &fx,
            shell,
            "grove new one && pwd && grove new two && grove cd one && pwd && grove ls --paths | wc -l",
        );
        let stdout = String::from_utf8_lossy(&out.stdout);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success(), "{shell}: {stderr}");
        let lines: Vec<&str> = stdout.lines().map(str::trim).collect();
        let one = fx.repo_dir().join("one").display().to_string();
        assert_eq!(
            lines,
            [one.as_str(), one.as_str(), "2"],
            "{shell} stderr: {stderr}"
        );
        assert!(!stderr.contains("hint:"), "{shell}: {stderr}");
    }
}

#[test]
fn wrapper_stays_put_and_propagates_failure() {
    for shell in ["zsh", "bash"] {
        if !shell_available(shell) {
            continue;
        }
        let fx = Fixture::new();
        let out = in_shell(&fx, shell, "grove cd missing; echo \"status=$?\"; pwd");
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(stdout.contains("status=1"), "{shell}: {stdout}");
        assert!(
            stdout.contains(&fx.clone.display().to_string()),
            "{shell}: {stdout}"
        );
    }
}
