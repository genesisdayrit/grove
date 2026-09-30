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
        .stderr(predicates::str::contains("grove worktree add"));
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
fn wrapper_cds_into_worktree_add_and_cd_targets_and_passes_ls_through() {
    for shell in ["zsh", "bash"] {
        if !shell_available(shell) {
            eprintln!("skipping {shell}: not installed");
            continue;
        }
        let fx = Fixture::new();
        let out = in_shell(
            &fx,
            shell,
            "grove worktree add one && pwd && grove wt add two && pwd && grove cd one && pwd && grove ls --paths | wc -l",
        );
        let stdout = String::from_utf8_lossy(&out.stdout);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success(), "{shell}: {stderr}");
        let lines: Vec<&str> = stdout.lines().map(str::trim).collect();
        let one = fx.repo_dir().join("one").display().to_string();
        let two = fx.repo_dir().join("two").display().to_string();
        assert_eq!(
            lines,
            [one.as_str(), two.as_str(), one.as_str(), "2"],
            "{shell} stderr: {stderr}"
        );
        assert!(!stderr.contains("hint:"), "{shell}: {stderr}");
    }
}

#[test]
fn wrapper_follows_flags_before_the_command_and_repo_mv() {
    for shell in ["zsh", "bash"] {
        if !shell_available(shell) {
            continue;
        }
        let fx = Fixture::new();
        fx.new_worktree("a");
        let out = in_shell(
            &fx,
            shell,
            &format!(
                "cd '{}' && grove -r myrepo cd a && pwd && mkdir sub && cd sub && grove repo mv renamed && pwd && grove repo ls --names",
                fx.home.display()
            ),
        );
        let stdout = String::from_utf8_lossy(&out.stdout);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success(), "{shell}: {stderr}");
        let lines: Vec<&str> = stdout.lines().map(str::trim).collect();
        let before = fx.repo_dir().join("a").display().to_string();
        let after = fx.root.join("renamed/a/sub").display().to_string();
        assert_eq!(
            lines,
            [before.as_str(), after.as_str(), "renamed"],
            "{shell} stderr: {stderr}"
        );
    }
}

#[test]
fn wrapper_leaves_no_temp_files_behind() {
    for shell in ["zsh", "bash"] {
        if !shell_available(shell) {
            continue;
        }
        let fx = Fixture::new();
        let tmp = fx.base().join("tmp");
        std::fs::create_dir(&tmp).unwrap();
        let out = in_shell(
            &fx,
            shell,
            &format!(
                "export TMPDIR='{}'; grove worktree add a; grove ls >/dev/null; grove cd nope; ls -A \"$TMPDIR\" | wc -l",
                tmp.display()
            ),
        );
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert_eq!(
            stdout.trim(),
            "0",
            "{shell}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

#[test]
fn cd_file_receives_the_path_instead_of_stdout() {
    let fx = Fixture::new();
    let a = fx.new_worktree("a");
    let file = fx.base().join("cd-target");
    std::fs::write(&file, "").unwrap();
    fx.grove()
        .env("GROVE_CD_FILE", &file)
        .env("GROVE_SHELL", "zsh")
        .args(["cd", "a"])
        .assert()
        .success()
        .stdout("");
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        a.display().to_string()
    );
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

#[test]
fn hint_names_the_users_shell() {
    let fx = Fixture::new();
    fx.new_worktree("a");
    fx.grove()
        .env("SHELL", "/bin/bash")
        .args(["cd", "a"])
        .assert()
        .success()
        .stderr(predicates::str::contains("grove init bash"));
}
