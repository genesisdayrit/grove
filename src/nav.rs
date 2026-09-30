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

/// What `grove` and `grove ls` cover: one repo, or every registered repo
/// (each with its clone listed as `<repo>/@repo`).
pub enum Scope {
    One(Repo),
    All(Vec<Repo>),
}

struct Row {
    /// Repo id; only shown for `Scope::All`.
    repo: String,
    wt: Worktree,
    details: worktrees::Details,
}

impl Row {
    fn label(&self, scope: &Scope) -> String {
        match scope {
            Scope::One(_) => self.wt.name.clone(),
            Scope::All(_) => format!("{}/{}", self.repo, self.wt.name),
        }
    }
}

/// Rows for `scope`, most recently active first.
fn rows(scope: &Scope) -> Result<Vec<Row>> {
    let mut out = Vec::new();
    let mut push = |repo: &Repo, wt: Worktree| -> Result<()> {
        out.push(Row {
            repo: repo.name.clone(),
            details: wt.details()?,
            wt,
        });
        Ok(())
    };
    match scope {
        Scope::One(repo) => {
            for wt in worktrees::list(repo)? {
                push(repo, wt)?;
            }
        }
        Scope::All(repos) => {
            for repo in repos {
                push(repo, worktrees::clone_entry(repo))?;
                for wt in worktrees::list(repo)? {
                    push(repo, wt)?;
                }
            }
        }
    }
    out.sort_by(|a, b| b.details.last_active.cmp(&a.details.last_active));
    Ok(out)
}

fn empty(scope: &Scope) -> String {
    match scope {
        Scope::One(repo) => format!(
            "no grove worktrees for {} yet; create one with `grove worktree add <name>`",
            repo.name
        ),
        Scope::All(_) => "no repos registered yet; run `grove repo add` in a clone".into(),
    }
}

/// Interactive picker, most recently active first.
pub fn pick(scope: &Scope) -> Result<Option<PathBuf>> {
    let rows = rows(scope)?;
    if rows.is_empty() {
        bail!("{}", empty(scope));
    }
    let names = rows.iter().map(|r| r.label(scope)).collect();
    let details = rows
        .iter()
        .map(|r| format!("{}  {}", r.wt.branch_label(), r.details.status()))
        .collect();
    let title = match scope {
        Scope::One(repo) => format!("grove · {}", repo.name),
        Scope::All(_) => "grove · all repos".to_string(),
    };
    let choice = tui::pick(&title, names, details)?;
    Ok(choice.map(|i| rows[i].wt.path.clone()))
}

pub fn ls(scope: &Scope, cwd: &Path, paths_only: bool) -> Result<()> {
    let rows = rows(scope)?;

    if paths_only {
        for r in &rows {
            println!("{}", r.wt.path.display());
        }
        return Ok(());
    }
    if rows.is_empty() {
        eprintln!("{}", empty(scope));
        return Ok(());
    }

    // The deepest row containing cwd (a worktree beats nothing; clones and
    // worktrees never nest).
    let cwd = cwd.canonicalize().ok();
    let current = rows
        .iter()
        .filter(|r| cwd.as_ref().is_some_and(|c| c.starts_with(&r.wt.path)))
        .max_by_key(|r| r.wt.path.as_os_str().len())
        .map(|r| r.wt.path.clone());
    let global = matches!(scope, Scope::All(_));
    let now = SystemTime::now();
    let mut table: Vec<Vec<String>> = Vec::new();
    let mut header = vec![
        " ",
        "REPO",
        "NAME",
        "BRANCH",
        "CREATED",
        "LAST ACTIVE",
        "STATUS",
    ];
    if !global {
        header.remove(1);
    }
    table.push(header.into_iter().map(String::from).collect());
    for r in &rows {
        let mut row = vec![
            if Some(&r.wt.path) == current.as_ref() {
                "*"
            } else {
                " "
            }
            .to_string(),
            r.repo.clone(),
            r.wt.name.clone(),
            r.wt.branch_label().to_string(),
            ago(now, r.details.created),
            ago(now, r.details.last_active),
            r.details.status(),
        ];
        if !global {
            row.remove(1);
        }
        table.push(row);
    }
    print_table(&table);
    Ok(())
}

/// Left-aligned columns separated by two spaces; the first (marker) column by one.
pub fn print_table(table: &[Vec<String>]) {
    let cols = table[0].len();
    let widths: Vec<usize> = (0..cols)
        .map(|i| table.iter().map(|r| r[i].chars().count()).max().unwrap())
        .collect();
    for row in table {
        let mut line = row[0].clone();
        line.push(' ');
        for (i, cell) in row.iter().enumerate().skip(1) {
            if i > 1 {
                line.push_str("  ");
            }
            line.push_str(&format!("{cell:<w$}", w = widths[i]));
        }
        println!("{}", line.trim_end());
    }
}

pub fn ago(now: SystemTime, then: Option<SystemTime>) -> String {
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
