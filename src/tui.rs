//! Draws the picker on `/dev/tty` — stdout is captured by the shell wrapper.

use crate::picker::{Key, Outcome, Picker};
use anyhow::{Context, Result, bail};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::style::{Attribute, Print, SetAttribute};
use crossterm::terminal::{self, Clear, ClearType};
use crossterm::{cursor, execute, queue};
use std::fs::File;
use std::io::{IsTerminal, Write};

/// Let the user pick one of `names` (shown with `details`). `None` if cancelled.
pub fn pick(title: &str, names: Vec<String>, details: Vec<String>) -> Result<Option<usize>> {
    if !std::io::stdin().is_terminal() {
        bail!("the picker needs a terminal; use `grove cd <name>` or `grove ls`");
    }
    let mut tty = File::options()
        .write(true)
        .open("/dev/tty")
        .context("the picker needs a terminal (/dev/tty)")?;
    let mut picker = Picker::new(names);
    let _guard = Screen::enter(&mut tty)?;
    loop {
        draw(&mut tty, title, &picker, &details)?;
        let Event::Key(key) = event::read()? else {
            continue;
        };
        let Some(key) = map_key(key) else { continue };
        match picker.handle(key) {
            Outcome::Pending => {}
            Outcome::Selected(i) => return Ok(Some(i)),
            Outcome::Cancelled => return Ok(None),
        }
    }
}

fn map_key(k: KeyEvent) -> Option<Key> {
    if k.kind != KeyEventKind::Press {
        return None;
    }
    let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
    Some(match k.code {
        KeyCode::Up => Key::Up,
        KeyCode::Down => Key::Down,
        KeyCode::Char('p' | 'k') if ctrl => Key::Up,
        KeyCode::Char('n' | 'j') if ctrl => Key::Down,
        KeyCode::Char('c') if ctrl => Key::Cancel,
        KeyCode::Char(_) if ctrl => return None,
        KeyCode::Char(c) => Key::Char(c),
        KeyCode::Backspace => Key::Backspace,
        KeyCode::Enter => Key::Enter,
        KeyCode::Esc => Key::Cancel,
        _ => return None,
    })
}

fn draw(tty: &mut File, title: &str, p: &Picker, details: &[String]) -> Result<()> {
    // Some ptys report 0x0; don't clip everything away.
    let (cols, rows) = match terminal::size() {
        Ok((c, r)) if c > 0 && r > 0 => (c, r),
        _ => (80, 24),
    };
    let cols = cols as usize;
    let height = (rows as usize).saturating_sub(2).max(1);
    let visible = p.visible();
    // Scroll so the highlight stays on screen.
    let offset = p.cursor().saturating_sub(height - 1);
    let width = visible
        .iter()
        .map(|&i| p.items()[i].chars().count())
        .max()
        .unwrap_or(0);

    queue!(tty, cursor::MoveTo(0, 0), Clear(ClearType::All))?;
    queue!(tty, Print(clip(&format!("{title} > {}", p.query()), cols)))?;
    for (row, &i) in visible.iter().enumerate().skip(offset).take(height) {
        let selected = row == p.cursor();
        let line = format!(
            "{} {:<width$}  {}",
            if selected { ">" } else { " " },
            p.items()[i],
            details[i]
        );
        queue!(tty, cursor::MoveTo(0, (row - offset + 1) as u16))?;
        if selected {
            queue!(tty, SetAttribute(Attribute::Reverse))?;
        }
        queue!(
            tty,
            Print(clip(&line, cols)),
            SetAttribute(Attribute::Reset)
        )?;
    }
    if visible.is_empty() {
        queue!(tty, cursor::MoveTo(0, 1), Print("  (no matches)"))?;
    }
    let hint = "↑/↓ j/k move · type to filter · enter select · esc/q cancel";
    queue!(
        tty,
        cursor::MoveTo(0, rows.saturating_sub(1)),
        Print(clip(hint, cols))
    )?;
    tty.flush()?;
    Ok(())
}

fn clip(s: &str, cols: usize) -> String {
    s.chars().take(cols).collect()
}

/// Raw mode + alternate screen for the picker's lifetime, restored on drop.
struct Screen {
    tty: File,
}

impl Screen {
    fn enter(tty: &mut File) -> Result<Self> {
        terminal::enable_raw_mode()?;
        let screen = Screen {
            tty: tty.try_clone()?,
        };
        execute!(tty, terminal::EnterAlternateScreen, cursor::Hide)?;
        Ok(screen)
    }
}

impl Drop for Screen {
    fn drop(&mut self) {
        let _ = execute!(self.tty, cursor::Show, terminal::LeaveAlternateScreen);
        let _ = terminal::disable_raw_mode();
    }
}
