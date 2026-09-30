//! `grove self update [--check]`: find out whether a newer release exists and install it.
//!
//! Only copies installed by the release's shell installer can replace themselves.
//! Homebrew and `cargo install` copies are told which command to run instead, so
//! grove never overwrites a binary another package manager owns.

use anyhow::{Context, Result, bail};
use axoupdater::{AxoUpdater, ReleaseSource, ReleaseSourceType, Version};
use std::path::Path;

const OWNER: &str = "genesisdayrit";
const REPO: &str = "grove";
const APP: &str = "grove";

/// Test hook: pretend GitHub reported this as the latest release.
const LATEST_OVERRIDE: &str = "GROVE_SELF_UPDATE_LATEST";

enum Install {
    /// Installed by the release's shell installer; axoupdater can replace it.
    Installer(Box<AxoUpdater>),
    Homebrew,
    /// `cargo install`, a local build, or anything else we can't update.
    Other,
}

pub fn run(check: bool) -> Result<()> {
    let current = Version::parse(env!("CARGO_PKG_VERSION"))?;
    let latest = latest_version()?;
    if latest <= current {
        println!("grove {current} is up to date");
        return Ok(());
    }
    println!("grove {current} → {latest} available");
    if check {
        return Ok(());
    }

    let exe = std::env::current_exe()?;
    let exe = exe.canonicalize().unwrap_or(exe);
    match detect_install(&exe) {
        Install::Installer(mut updater) => {
            let result = updater
                .run_sync()
                .context("update failed")?
                .context("update failed: installer reported nothing to do")?;
            println!("updated grove to {}", result.new_version);
            Ok(())
        }
        Install::Homebrew => bail!("grove was installed with Homebrew; run: brew upgrade grove"),
        Install::Other => bail!(
            "this grove wasn't installed by the grove installer; to update, run:\n  \
             cargo install --git https://github.com/{OWNER}/{REPO} --tag v{latest} --force"
        ),
    }
}

fn latest_version() -> Result<Version> {
    if let Ok(v) = std::env::var(LATEST_OVERRIDE) {
        return Ok(Version::parse(&v)?);
    }
    let mut updater = AxoUpdater::new_for(APP);
    updater.set_release_source(ReleaseSource {
        release_type: ReleaseSourceType::GitHub,
        owner: OWNER.into(),
        name: REPO.into(),
        app_name: APP.into(),
    });
    authorize(&mut updater);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let latest = runtime
        .block_on(updater.query_new_version())
        .context("couldn't check GitHub for the latest grove release")?
        .context("no grove releases found on GitHub")?;
    Ok(latest.clone())
}

fn detect_install(exe: &Path) -> Install {
    let mut updater = AxoUpdater::new_for(APP);
    if updater.load_receipt().is_ok()
        && updater
            .check_receipt_is_for_this_executable()
            .unwrap_or(false)
    {
        authorize(&mut updater);
        return Install::Installer(Box::new(updater));
    }
    if exe.components().any(|c| c.as_os_str() == "Cellar") {
        return Install::Homebrew;
    }
    Install::Other
}

/// Unauthenticated GitHub API calls are rate-limited to 60/hour; use a token if the user has one.
fn authorize(updater: &mut AxoUpdater) {
    if let Ok(token) = std::env::var("GITHUB_TOKEN")
        && !token.is_empty()
    {
        updater.set_github_token(&token);
    }
}
