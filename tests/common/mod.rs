#![allow(dead_code)]

use assert_cmd::Command;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
use tempfile::TempDir;

/// A throwaway world: a bare "origin", a clone of it, a fake HOME and a grove root.
pub struct Fixture {
    pub tmp: TempDir,
    pub origin: PathBuf,
    pub clone: PathBuf,
    pub root: PathBuf,
    pub home: PathBuf,
}

impl Fixture {
    /// Origin whose default branch is `main`, cloned to `<tmp>/repos/myrepo`
    /// and registered with grove.
    pub fn new() -> Self {
        Self::with_default_branch("main")
    }

    pub fn with_default_branch(branch: &str) -> Self {
        let fx = Self::unregistered(branch);
        fx.grove().args(["repo", "add"]).assert().success();
        fx
    }

    /// Like `new`, but the clone isn't registered with grove.
    pub fn unregistered(branch: &str) -> Self {
        let tmp = TempDir::new().unwrap();
        // Canonicalize so macOS /var -> /private/var doesn't bite path comparisons.
        let base = tmp.path().canonicalize().unwrap();
        let home = base.join("home");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::write(
            home.join(".gitconfig"),
            "[user]\n\tname = Test\n\temail = test@example.com\n[commit]\n\tgpgsign = false\n",
        )
        .unwrap();

        let origin = base.join("origin.git");
        let seed = base.join("seed");
        let fx = Fixture {
            tmp,
            origin: origin.clone(),
            clone: base.join("repos").join("myrepo"),
            root: base.join("groveroot"),
            home,
        };

        fx.git(
            &base,
            &["init", "--bare", "-b", branch, origin.to_str().unwrap()],
        );
        fx.git(&base, &["init", "-b", branch, seed.to_str().unwrap()]);
        std::fs::write(seed.join("README.md"), "hello\n").unwrap();
        fx.git(&seed, &["add", "."]);
        fx.git(&seed, &["commit", "-m", "initial"]);
        fx.git(
            &seed,
            &["remote", "add", "origin", origin.to_str().unwrap()],
        );
        fx.git(&seed, &["push", "origin", branch]);
        std::fs::create_dir_all(fx.clone.parent().unwrap()).unwrap();
        fx.git(
            &base,
            &[
                "clone",
                origin.to_str().unwrap(),
                fx.clone.to_str().unwrap(),
            ],
        );
        fx
    }

    pub fn base(&self) -> PathBuf {
        self.tmp.path().canonicalize().unwrap()
    }

    pub fn git_env(&self, cmd: &mut StdCommand) {
        cmd.env("HOME", &self.home)
            .env("GIT_CONFIG_GLOBAL", self.home.join(".gitconfig"))
            .env("GIT_CONFIG_NOSYSTEM", "1");
    }

    /// Run git in `dir`, panic on failure, return trimmed stdout.
    pub fn git(&self, dir: &Path, args: &[&str]) -> String {
        let mut cmd = StdCommand::new("git");
        cmd.current_dir(dir).args(args);
        self.git_env(&mut cmd);
        let out = cmd.output().unwrap();
        assert!(
            out.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8(out.stdout).unwrap().trim().to_string()
    }

    /// Push a new commit to origin's `branch` from a scratch clone; returns its sha.
    pub fn advance_origin(&self, branch: &str) -> String {
        let scratch = self.base().join(format!("scratch-{}", rand_suffix()));
        self.git(
            &self.base(),
            &[
                "clone",
                "-b",
                branch,
                self.origin.to_str().unwrap(),
                scratch.to_str().unwrap(),
            ],
        );
        std::fs::write(scratch.join("upstream.txt"), rand_suffix()).unwrap();
        self.git(&scratch, &["add", "."]);
        self.git(&scratch, &["commit", "-m", "upstream work"]);
        self.git(&scratch, &["push", "origin", branch]);
        self.git(&scratch, &["rev-parse", "HEAD"])
    }

    /// `grove` with an isolated environment, run from `dir`.
    pub fn grove_in(&self, dir: &Path) -> Command {
        let mut cmd = Command::cargo_bin("grove").unwrap();
        cmd.current_dir(dir)
            .env_clear()
            .env("PATH", std::env::var("PATH").unwrap())
            .env("HOME", &self.home)
            .env("GIT_CONFIG_GLOBAL", self.home.join(".gitconfig"))
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("XDG_CONFIG_HOME", self.home.join(".config"))
            .env("GROVE_ROOT", &self.root)
            .write_stdin("");
        cmd
    }

    /// `grove` run from the original clone.
    pub fn grove(&self) -> Command {
        self.grove_in(&self.clone)
    }

    /// Create a worktree, asserting success; returns its path.
    pub fn new_worktree(&self, name: &str) -> PathBuf {
        let out = self
            .grove()
            .args(["worktree", "add", name])
            .assert()
            .success();
        PathBuf::from(
            String::from_utf8(out.get_output().stdout.clone())
                .unwrap()
                .trim(),
        )
    }

    /// Clone origin again to `<tmp>/<parent>/<basename>`; returns the path.
    pub fn another_clone(&self, parent: &str, basename: &str) -> PathBuf {
        let path = self.base().join(parent).join(basename);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        self.git(
            &self.base(),
            &[
                "clone",
                self.origin.to_str().unwrap(),
                path.to_str().unwrap(),
            ],
        );
        path
    }

    pub fn repo_dir(&self) -> PathBuf {
        self.root.join("myrepo")
    }
}

fn rand_suffix() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    format!(
        "{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    )
}

pub fn stdout_of(assert: &assert_cmd::assert::Assert) -> String {
    String::from_utf8(assert.get_output().stdout.clone()).unwrap()
}

pub fn stderr_of(assert: &assert_cmd::assert::Assert) -> String {
    String::from_utf8(assert.get_output().stderr.clone()).unwrap()
}
