mod common;

use assert_cmd::Command;
use common::*;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

const CURRENT: &str = env!("CARGO_PKG_VERSION");

/// `bin` with an isolated HOME/config (so no real install receipt is found) and a
/// faked "latest release" instead of a GitHub call.
fn self_update(tmp: &TempDir, bin: &Path, latest: &str) -> Command {
    let home = tmp.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    let mut cmd = Command::new(bin);
    cmd.env_clear()
        .env("PATH", std::env::var("PATH").unwrap())
        .env("HOME", &home)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .env("GROVE_SELF_UPDATE_LATEST", latest)
        .args(["self", "update"]);
    cmd
}

fn built_grove() -> PathBuf {
    assert_cmd::cargo::cargo_bin("grove")
}

/// A copy of grove laid out the way Homebrew installs it.
fn brew_grove(tmp: &TempDir) -> PathBuf {
    let bin = tmp
        .path()
        .join("homebrew/Cellar/grove")
        .join(CURRENT)
        .join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    let dest = bin.join("grove");
    std::fs::copy(built_grove(), &dest).unwrap();
    dest
}

#[test]
fn check_reports_up_to_date_when_latest_is_this_version() {
    let tmp = TempDir::new().unwrap();
    let out = self_update(&tmp, &built_grove(), CURRENT)
        .arg("--check")
        .assert()
        .success();
    assert_eq!(
        stdout_of(&out).trim(),
        format!("grove {CURRENT} is up to date")
    );
}

#[test]
fn check_treats_an_older_latest_release_as_up_to_date() {
    let tmp = TempDir::new().unwrap();
    let out = self_update(&tmp, &built_grove(), "0.0.1")
        .arg("--check")
        .assert()
        .success();
    assert!(stdout_of(&out).contains("is up to date"));
}

#[test]
fn check_reports_a_newer_release_and_exits_zero() {
    let tmp = TempDir::new().unwrap();
    let out = self_update(&tmp, &built_grove(), "9.9.9")
        .arg("--check")
        .assert()
        .success();
    assert_eq!(
        stdout_of(&out).trim(),
        format!("grove {CURRENT} → 9.9.9 available")
    );
    assert_eq!(stderr_of(&out), "");
}

#[test]
fn update_when_up_to_date_does_nothing_and_succeeds() {
    let tmp = TempDir::new().unwrap();
    let out = self_update(&tmp, &built_grove(), CURRENT)
        .assert()
        .success();
    assert!(stdout_of(&out).contains("is up to date"));
}

#[test]
fn update_of_a_cargo_or_local_build_prints_the_cargo_install_command() {
    let tmp = TempDir::new().unwrap();
    let out = self_update(&tmp, &built_grove(), "9.9.9")
        .assert()
        .failure();
    assert!(stdout_of(&out).contains("9.9.9 available"));
    assert!(stderr_of(&out).contains(
        "cargo install --git https://github.com/genesisdayrit/grove --tag v9.9.9 --force"
    ));
}

#[test]
fn update_of_a_homebrew_install_points_at_brew_upgrade() {
    let tmp = TempDir::new().unwrap();
    let out = self_update(&tmp, &brew_grove(&tmp), "9.9.9")
        .assert()
        .failure();
    assert!(stderr_of(&out).contains("run: brew upgrade grove"));
    assert!(!stderr_of(&out).contains("cargo install"));
}

#[test]
fn check_works_the_same_for_a_homebrew_install() {
    let tmp = TempDir::new().unwrap();
    let out = self_update(&tmp, &brew_grove(&tmp), "9.9.9")
        .arg("--check")
        .assert()
        .success();
    assert!(stdout_of(&out).contains("9.9.9 available"));
}

#[test]
fn self_update_passes_through_the_shell_wrapper() {
    let tmp = TempDir::new().unwrap();
    let init = Command::new(built_grove())
        .args(["init", "zsh"])
        .output()
        .unwrap();
    let script = String::from_utf8(init.stdout).unwrap();
    let bin_dir = built_grove().parent().unwrap().to_path_buf();
    let out = std::process::Command::new("zsh")
        .args(["-f", "-c"])
        .arg(format!(
            "{script}\ncd {tmp}\ngrove self update --check; printf 'pwd=%s\\n' \"$PWD\"",
            tmp = tmp.path().display()
        ))
        .env_clear()
        .env(
            "PATH",
            format!("{}:{}", bin_dir.display(), std::env::var("PATH").unwrap()),
        )
        .env("HOME", tmp.path())
        .env("XDG_CONFIG_HOME", tmp.path().join(".config"))
        .env("GROVE_SELF_UPDATE_LATEST", "9.9.9")
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("9.9.9 available"), "{stdout}");
    assert!(
        stdout.contains(&format!("pwd={}", tmp.path().display())),
        "{stdout}"
    );
}
