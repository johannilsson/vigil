use crate::parser;
use crate::term::{self, RawModeGuard, Writer};
use crossterm::{
    event::{read, Event, KeyCode, KeyEvent},
    style::Color,
};
use std::io;
use std::path::{Path, PathBuf};

/// Returns `Some((path, cursor))` when the user opens a file, `None` when they quit.
pub fn run(dir: &Path, initial_cursor: usize) -> io::Result<Option<(PathBuf, usize)>> {
    let files = parser::parse_directory(dir)?;

    if files.is_empty() {
        eprintln!("No .todo.md files found in {}", dir.display());
        return Ok(None);
    }

    let _guard = RawModeGuard::enter().map_err(|e| io::Error::other(e.to_string()))?;
    let mut w = Writer::new();
    let mut cursor: usize = initial_cursor.min(files.len() - 1);

    loop {
        render(&mut w, &files, cursor);

        match read().map_err(|e| io::Error::other(e.to_string()))? {
            Event::Key(KeyEvent { code, .. }) => match code {
                KeyCode::Up | KeyCode::Char('k') => {
                    cursor = (cursor + files.len() - 1) % files.len();
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    cursor = (cursor + 1) % files.len();
                }
                KeyCode::Enter => return Ok(Some((files[cursor].path.clone(), cursor))),
                KeyCode::Char('q') => return Ok(None),
                _ => {}
            },
            Event::Resize(_, _) => {}
            _ => {}
        }
    }
}

fn render(w: &mut Writer, files: &[parser::TodoFile], cursor: usize) {
    let (cols, _) = term::terminal_size();

    w.clear_screen().move_to(0, 0);
    w.bold().print("Todos").reset_attr();
    w.print("\r\n");

    for (i, file) in files.iter().enumerate() {
        let selected = i == cursor;

        if selected {
            w.color(Color::Cyan).bold().print("\u{25b6} ").reset_attr();
            w.color(Color::Cyan).bold();
        } else {
            w.print("  ");
        }

        let progress_str = if file.total > 0 {
            format!(" {}/{}", file.done, file.total)
        } else {
            String::new()
        };

        let max_title = (cols as usize).saturating_sub(2 + progress_str.len() + 2);
        let title = truncate(&file.title, max_title);

        w.print(&title);
        if selected {
            w.reset_attr();
        }

        if !progress_str.is_empty() {
            w.dim().print(&progress_str).reset_attr();
        }

        w.print("\r\n");
    }

    w.flush().ok();
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}\u{2026}", &s[..max.saturating_sub(1)])
    }
}
