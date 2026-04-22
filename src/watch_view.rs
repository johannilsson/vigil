use crate::parser::{self, Marker, Phase, Step, TodoFile};
use crate::term::{self, hrule, progress_bar_string, RawModeGuard, Writer};
use crossterm::{
    event::{poll, read, Event, KeyCode, KeyEvent},
    style::Color,
};
use std::io;
use std::path::Path;
use std::time::{Duration, SystemTime};

// One terminal row of renderable content.
enum DisplayLine<'a> {
    Phase(&'a str),
    StepFirst { step: &'a Step, text: String },
    StepCont(String),
}

pub fn run(path: &Path, has_list: bool) -> io::Result<()> {
    let mut todo = parser::parse_file(path)?;
    let _guard = RawModeGuard::enter().map_err(|e| io::Error::other(e.to_string()))?;
    let mut w = Writer::new();
    let mut last_mtime = mtime(path);
    let mut scroll: usize = 0;

    loop {
        // Block up to 250 ms for a key event, then fall through to file check + render.
        // Using poll here (instead of a background thread) means this function returns
        // cleanly with no lingering threads that could steal the first keypress in list view.
        if poll(Duration::from_millis(250)).map_err(|e| io::Error::other(e.to_string()))?
            && let Ok(Event::Key(KeyEvent { code, .. })) = read()
        {
            match code {
                KeyCode::Char('q') => return Ok(()),
                KeyCode::Up | KeyCode::Char('k') => scroll = scroll.saturating_sub(1),
                KeyCode::Down | KeyCode::Char('j') => scroll = scroll.saturating_add(1),
                _ => {}
            }
        }

        // check for file changes
        let current_mtime = mtime(path);
        if current_mtime != last_mtime {
            if let Ok(updated) = parser::parse_file(path) {
                todo = updated;
            }
            last_mtime = current_mtime;
        }

        let (cols, rows) = term::terminal_size();
        let display_lines = collect_display_lines(&todo.phases, cols);
        let total = display_lines.len();
        let visible = (rows as usize).saturating_sub(5);
        scroll = scroll.min(total.saturating_sub(visible));

        let vs = ViewState { scroll, visible, total };
        render(&mut w, &todo, path, has_list, cols, &vs, &display_lines);
    }
}

fn collect_display_lines<'a>(phases: &'a [Phase], cols: u16) -> Vec<DisplayLine<'a>> {
    const PREFIX: usize = 4;
    let content_width = (cols as usize).saturating_sub(PREFIX);
    let mut out = Vec::new();
    for phase in phases {
        if !phase.name.is_empty() {
            out.push(DisplayLine::Phase(&phase.name));
        }
        for step in &phase.steps {
            let full_text = match (&step.bold_name, step.description.is_empty()) {
                (Some(name), false) => format!("{name} \u{2014} {}", step.description),
                (Some(name), true) => name.clone(),
                (None, _) => step.description.clone(),
            };
            let mut wrapped = word_wrap(&full_text, content_width).into_iter();
            if let Some(first) = wrapped.next() {
                out.push(DisplayLine::StepFirst { step, text: first });
            }
            for cont in wrapped {
                out.push(DisplayLine::StepCont(cont));
            }
        }
    }
    out
}

fn mtime(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

struct ViewState {
    scroll: usize,
    visible: usize,
    total: usize,
}

fn render(
    w: &mut Writer,
    file: &TodoFile,
    path: &Path,
    has_list: bool,
    cols: u16,
    vs: &ViewState,
    display_lines: &[DisplayLine<'_>],
) {
    w.clear_screen().move_to(0, 0);

    // Title
    w.color(Color::Cyan).bold().print(&file.title).reset_attr();
    w.print("\r\n");

    // Progress bar
    let bar = progress_bar_string(file.done, file.total);
    let bar_color = if file.total > 0 && file.done == file.total {
        Color::Green
    } else if file.done > 0 {
        Color::Yellow
    } else {
        Color::White
    };
    w.color(bar_color).print(&bar).reset_attr();
    w.print(&format!(" {}/{} steps complete\r\n", file.done, file.total));

    // Top rule
    hrule(w, cols);
    w.print("\r\n");

    // Scrolled content (each DisplayLine is exactly one terminal row)
    let end = (vs.scroll + vs.visible).min(vs.total);
    for dl in &display_lines[vs.scroll..end] {
        match dl {
            DisplayLine::Phase(name) => {
                w.color(Color::Blue).bold().print(name).reset_attr();
                w.print("\r\n");
            }
            DisplayLine::StepFirst { step, text } => {
                render_step_first(w, step, text);
            }
            DisplayLine::StepCont(text) => {
                w.print("    ").print(text).print("\r\n");
            }
        }
    }

    // Bottom rule
    hrule(w, cols);
    w.print("\r\n");

    // Footer
    let path_str = path.display().to_string();
    let scroll_hint = if vs.total > vs.visible { "  ↑↓ scroll" } else { "" };
    let action = if has_list { "q back to list" } else { "q quit" };
    let footer = format!("{path_str}{scroll_hint}  \u{2022}  {action}");
    let footer = truncate_path(&footer, cols as usize);
    w.dim().print(&footer).reset_attr();

    w.flush().ok();
}

fn render_step_first(w: &mut Writer, step: &Step, text: &str) {
    let (symbol, color, extra_bold) = match step.marker {
        Marker::Todo => ("\u{25cb}", Color::DarkGrey, false),
        Marker::InProgress => ("\u{25d0}", Color::Yellow, false),
        Marker::Done => ("\u{2713}", Color::Green, false),
        Marker::Failed => ("\u{2717}", Color::Red, true),
        Marker::Skipped => ("\u{2013}", Color::DarkGrey, false),
    };
    w.print("  ").color(color);
    if extra_bold {
        w.bold();
    }
    w.print(symbol).reset_attr().reset_color().print(" ");
    render_first_line(w, text, step.bold_name.as_deref());
    w.print("\r\n");
}

// Prints the first wrapped line, bolding the name portion if present.
fn render_first_line(w: &mut Writer, line: &str, bold_name: Option<&str>) {
    let Some(name) = bold_name else {
        w.print(line);
        return;
    };
    let name_chars = name.chars().count();
    let line_chars = line.chars().count();
    if line_chars <= name_chars {
        // Entire line is within the bold name (very long name edge case).
        w.bold().print(line).reset_attr();
    } else {
        let split = line
            .char_indices()
            .nth(name_chars)
            .map(|(i, _)| i)
            .unwrap_or(line.len());
        w.bold().print(&line[..split]).reset_attr().print(&line[split..]);
    }
}

fn word_wrap(text: &str, width: usize) -> Vec<String> {
    if text.is_empty() || width == 0 {
        return vec![text.to_string()];
    }
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    for word in text.split(' ') {
        let word_len = word.chars().count();
        let cur_len = current.chars().count();
        if current.is_empty() {
            current.push_str(word);
        } else if cur_len + 1 + word_len <= width {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(std::mem::take(&mut current));
            current.push_str(word);
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

fn truncate_path(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("\u{2026}{}", &s[s.len().saturating_sub(max.saturating_sub(1))..])
    }
}
