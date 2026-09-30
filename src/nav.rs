//! `grove cd`, `grove ls` and the no-argument picker.

use crate::repo::Repo;
use crate::worktrees::{self, Worktree};
use crate::{name, tui};
use anyhow::{Result, bail};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub fn cd(repo: &Repo, input: &str) -> Result<PathBuf> {
    let all = worktrees::list(repo)?;
    let wanted = name::normalize(input).map(|n| n.folder).unwrap_or_default();
    if let Some(w) = all.iter().find(|w| w.name == wanted) {
        return Ok(w.path.clone());
    }
    let close: Vec<&str> = all
        .iter()
        .map(|w| w.name.as_str())
        .filter(|n| !wanted.is_empty() && is_close(&wanted, n))
        .collect();
    if close.is_empty() {
        bail!("no worktree `{input}` in {} (see `grove ls`)", repo.name);
    }
    bail!(
        "no worktree `{input}` in {}; did you mean: {}",
        repo.name,
        close.join(", ")
    );
}

fn is_close(wanted: &str, candidate: &str) -> bool {
    candidate.contains(wanted)
        || wanted.contains(candidate)
        || edit_distance(wanted, candidate) <= 2
}

fn edit_distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut cur = vec![i + 1];
        for (j, cb) in b.iter().enumerate() {
            let sub = prev[j] + usize::from(ca != *cb);
            cur.push(sub.min(prev[j + 1] + 1).min(cur[j] + 1));
        }
        prev = cur;
    }
    prev[b.len()]
}

/// Interactive picker over the repo's worktrees, most recently active first.
pub fn pick(repo: &Repo) -> Result<Option<PathBuf>> {
    let all = worktrees::list(repo)?;
    let rows = by_last_active(&all)?;
    if rows.is_empty() {
        bail!(
            "no grove worktrees for {} yet; create one with `grove new <name>`",
            repo.name
        );
    }
    let names = rows.iter().map(|(w, _)| w.name.clone()).collect();
    let details = rows
        .iter()
        .map(|(w, d)| {
            format!(
                "{}  {}",
                w.branch.as_deref().unwrap_or("(detached)"),
                d.status()
            )
        })
        .collect();
    let choice = tui::pick(&format!("grove · {}", repo.name), names, details)?;
    Ok(choice.map(|i| rows[i].0.path.clone()))
}

fn by_last_active(all: &[Worktree]) -> Result<Vec<(&Worktree, worktrees::Details)>> {
    let mut rows: Vec<_> = all
        .iter()
        .map(|w| Ok((w, w.details()?)))
        .collect::<Result<_>>()?;
    rows.sort_by(|a, b| b.1.last_active.cmp(&a.1.last_active));
    Ok(rows)
}

pub fn ls(repo: &Repo, cwd: &Path, paths_only: bool) -> Result<()> {
    let all = worktrees::list(repo)?;
    let rows = by_last_active(&all)?;

    if paths_only {
        for (w, _) in &rows {
            println!("{}", w.path.display());
        }
        return Ok(());
    }
    if rows.is_empty() {
        eprintln!(
            "no grove worktrees for {} yet; create one with `grove new <name>`",
            repo.name
        );
        return Ok(());
    }

    let current = worktrees::current(&all, cwd).map(|w| w.path.clone());
    let now = SystemTime::now();
    let table: Vec<[String; 6]> = rows
        .iter()
        .map(|(w, d)| {
            [
                if Some(&w.path) == current.as_ref() {
                    "*"
                } else {
                    " "
                }
                .to_string(),
                w.name.clone(),
                w.branch.clone().unwrap_or_else(|| "(detached)".into()),
                ago(now, d.created),
                ago(now, d.last_active),
                d.status(),
            ]
        })
        .collect();
    let header = [" ", "NAME", "BRANCH", "CREATED", "LAST ACTIVE", "STATUS"].map(String::from);
    let widths: Vec<usize> = (0..6)
        .map(|i| {
            std::iter::once(&header)
                .chain(&table)
                .map(|r| r[i].chars().count())
                .max()
                .unwrap()
        })
        .collect();
    for row in std::iter::once(&header).chain(&table) {
        let line = format!(
            "{} {:<w1$}  {:<w2$}  {:<w3$}  {:<w4$}  {}",
            row[0],
            row[1],
            row[2],
            row[3],
            row[4],
            row[5],
            w1 = widths[1],
            w2 = widths[2],
            w3 = widths[3],
            w4 = widths[4],
        );
        println!("{}", line.trim_end());
    }
    Ok(())
}

fn ago(now: SystemTime, then: Option<SystemTime>) -> String {
    let Some(then) = then else { return "-".into() };
    let secs = now.duration_since(then).map(|d| d.as_secs()).unwrap_or(0);
    match secs {
        0..60 => "just now".into(),
        60..3600 => format!("{}m ago", secs / 60),
        3600..86400 => format!("{}h ago", secs / 3600),
        86400..2_592_000 => format!("{}d ago", secs / 86400),
        2_592_000..31_536_000 => format!("{}mo ago", secs / 2_592_000),
        _ => format!("{}y ago", secs / 31_536_000),
    }
}
