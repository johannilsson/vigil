use crossterm::{
    cursor::{Hide, MoveTo, Show},
    execute, queue,
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor},
    terminal::{
        disable_raw_mode, enable_raw_mode, size, Clear, ClearType, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};
use std::io::{self, BufWriter, Write};

pub struct RawModeGuard;

impl RawModeGuard {
    pub fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen, Hide)?;
        Ok(RawModeGuard)
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, Show);
    }
}

pub struct Writer(BufWriter<io::Stdout>);

impl Writer {
    pub fn new() -> Self {
        Writer(BufWriter::new(io::stdout()))
    }

    pub fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }

    pub fn clear_screen(&mut self) -> &mut Self {
        let _ = queue!(self.0, Clear(ClearType::All));
        self
    }

    pub fn move_to(&mut self, col: u16, row: u16) -> &mut Self {
        let _ = queue!(self.0, MoveTo(col, row));
        self
    }

    pub fn print(&mut self, s: &str) -> &mut Self {
        let _ = queue!(self.0, Print(s));
        self
    }

    pub fn color(&mut self, c: Color) -> &mut Self {
        let _ = queue!(self.0, SetForegroundColor(c));
        self
    }

    pub fn reset_color(&mut self) -> &mut Self {
        let _ = queue!(self.0, ResetColor);
        self
    }

    pub fn bold(&mut self) -> &mut Self {
        let _ = queue!(self.0, SetAttribute(Attribute::Bold));
        self
    }

    pub fn dim(&mut self) -> &mut Self {
        let _ = queue!(self.0, SetAttribute(Attribute::Dim));
        self
    }

    pub fn reset_attr(&mut self) -> &mut Self {
        let _ = queue!(self.0, SetAttribute(Attribute::Reset));
        self
    }
}

pub fn terminal_size() -> (u16, u16) {
    size().unwrap_or((80, 24))
}

pub fn hrule(w: &mut Writer, cols: u16) {
    w.print(&"\u{2500}".repeat(cols as usize));
}

pub fn progress_bar_string(done: u32, total: u32) -> String {
    const WIDTH: usize = 38;
    let filled = if total == 0 {
        0
    } else {
        ((done as f32 / total as f32) * WIDTH as f32).round() as usize
    };
    let filled = filled.min(WIDTH);
    format!(
        "[{}{}]",
        "\u{2588}".repeat(filled),
        "\u{2591}".repeat(WIDTH - filled)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_bar_full() {
        let s = progress_bar_string(7, 7);
        assert!(s.starts_with('['));
        assert!(s.ends_with(']'));
        // 38 filled blocks
        let inner = &s[1..s.len() - 1];
        let blocks: Vec<&str> = inner.split('\u{2591}').collect();
        assert_eq!(blocks.len(), 1, "should have no empty blocks");
        // count filled chars
        let filled_count = inner.chars().filter(|&c| c == '\u{2588}').count();
        assert_eq!(filled_count, 38);
    }

    #[test]
    fn progress_bar_half() {
        let s = progress_bar_string(4, 8);
        let inner = &s[1..s.len() - 1];
        let filled = inner.chars().filter(|&c| c == '\u{2588}').count();
        let empty = inner.chars().filter(|&c| c == '\u{2591}').count();
        assert_eq!(filled, 19);
        assert_eq!(empty, 19);
    }

    #[test]
    fn progress_bar_zero() {
        let s = progress_bar_string(0, 5);
        let inner = &s[1..s.len() - 1];
        let filled = inner.chars().filter(|&c| c == '\u{2588}').count();
        assert_eq!(filled, 0);
        let empty = inner.chars().filter(|&c| c == '\u{2591}').count();
        assert_eq!(empty, 38);
    }

    #[test]
    fn progress_bar_zero_total() {
        let s = progress_bar_string(0, 0);
        let inner = &s[1..s.len() - 1];
        let filled = inner.chars().filter(|&c| c == '\u{2588}').count();
        assert_eq!(filled, 0);
    }
}
