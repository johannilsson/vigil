use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Clone, PartialEq)]
pub enum Marker {
    Todo,
    InProgress,
    Done,
    Failed,
    Skipped,
}

#[derive(Debug, Clone)]
pub struct Step {
    pub marker: Marker,
    pub bold_name: Option<String>,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct Phase {
    pub name: String,
    pub steps: Vec<Step>,
}

#[derive(Debug, Clone)]
pub struct TodoFile {
    pub path: PathBuf,
    pub title: String,
    pub done: u32,
    pub total: u32,
    pub phases: Vec<Phase>,
    pub modified: Option<SystemTime>,
}

pub fn parse_file(path: &Path) -> io::Result<TodoFile> {
    let content = std::fs::read_to_string(path)?;
    let mut title: Option<String> = None;
    let mut progress_header: Option<(u32, u32)> = None;
    let mut phases: Vec<Phase> = Vec::new();
    let mut counted_done: u32 = 0;
    let mut counted_total: u32 = 0;

    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("# Plan:") {
            title = Some(rest.trim().to_string());
        } else if let Some(rest) = line.strip_prefix("## Progress:") {
            let rest = rest.trim();
            if let Some((a, b)) = rest.split_once('/')
                && let (Ok(d), Ok(t)) = (a.trim().parse::<u32>(), b.trim().parse::<u32>())
            {
                progress_header = Some((d, t));
            }
        } else if let Some(rest) = line.strip_prefix("## ") {
            phases.push(Phase { name: rest.trim().to_string(), steps: Vec::new() });
        } else if let Some(rest) = line.strip_prefix("- [") {
            let marker_char = rest.chars().next().unwrap_or(' ');
            let marker = match marker_char {
                'x' | 'X' => Marker::Done,
                '~' => Marker::InProgress,
                '!' => Marker::Failed,
                '-' => Marker::Skipped,
                _ => Marker::Todo,
            };
            // text starts after "] "
            let text = rest.get(2..).unwrap_or("").trim();
            let step = parse_step_text(marker, text);

            if step.marker == Marker::Done {
                counted_done += 1;
            }
            counted_total += 1;

            match phases.last_mut() {
                Some(phase) => phase.steps.push(step),
                None => {
                    phases.push(Phase { name: String::new(), steps: vec![step] });
                }
            }
        }
    }

    let (done, total) = progress_header.unwrap_or((counted_done, counted_total));

    let title = title.unwrap_or_else(|| {
        path.file_stem()
            .map(|s| s.to_string_lossy().trim_end_matches(".todo").to_string())
            .unwrap_or_default()
    });

    let modified = std::fs::metadata(path).and_then(|m| m.modified()).ok();
    Ok(TodoFile { path: path.to_path_buf(), title, done, total, phases, modified })
}

fn parse_step_text(marker: Marker, text: &str) -> Step {
    if let Some(after_open) = text.strip_prefix("**")
        && let Some(close) = after_open.find("**")
    {
        let bold_name = after_open[..close].to_string();
        let rest = &after_open[close + 2..];
        let description = rest
            .split_once(" \u{2014} ")
            .map(|(_, d)| d)
            .or_else(|| rest.split_once(" - ").map(|(_, d)| d))
            .unwrap_or(rest)
            .trim()
            .to_string();
        return Step { marker, bold_name: Some(bold_name), description };
    }
    // No bold: split on em-dash for description, or use whole text
    let description = text
        .split_once(" \u{2014} ")
        .map(|(_, d)| d)
        .unwrap_or(text)
        .trim()
        .to_string();
    Step { marker, bold_name: None, description }
}

pub fn parse_directory(dir: &Path) -> io::Result<Vec<TodoFile>> {
    let mut files: Vec<TodoFile> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .ends_with(".todo.md")
        })
        .filter_map(|e| parse_file(&e.path()).ok())
        .collect();
    // Most recently modified first; fall back to title order for equal/missing mtimes.
    files.sort_by(|a, b| {
        b.modified.cmp(&a.modified).then_with(|| a.title.to_lowercase().cmp(&b.title.to_lowercase()))
    });
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn parse_str(content: &str, stem: &str) -> TodoFile {
        let dir = std::env::temp_dir().join(format!("vigil_test_{stem}"));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("{stem}.todo.md"));
        fs::write(&path, content).unwrap();
        let result = parse_file(&path).unwrap();
        fs::remove_dir_all(&dir).unwrap();
        result
    }

    #[test]
    fn parse_title() {
        let f = parse_str("# Plan: My Plan\n", "t1");
        assert_eq!(f.title, "My Plan");
    }

    #[test]
    fn parse_title_fallback() {
        let f = parse_str("no heading\n", "my-plan");
        assert_eq!(f.title, "my-plan");
    }

    #[test]
    fn parse_progress_header() {
        let content = "# Plan: X\n## Progress: 3/7\n- [ ] **A** — a\n";
        let f = parse_str(content, "t3");
        assert_eq!(f.done, 3);
        assert_eq!(f.total, 7);
    }

    #[test]
    fn parse_progress_counted() {
        let content = "# Plan: X\n## Phase\n- [x] a\n- [ ] b\n- [~] c\n";
        let f = parse_str(content, "t4");
        assert_eq!(f.done, 1);
        assert_eq!(f.total, 3);
    }

    #[test]
    fn marker_done_only_x() {
        let content = "## P\n- [x] a\n- [ ] b\n- [~] c\n- [!] d\n- [-] e\n";
        let f = parse_str(content, "t5");
        assert_eq!(f.done, 1);
        assert_eq!(f.total, 5);
    }

    #[test]
    fn step_bold_name() {
        let content = "## P\n- [x] **Step Name** \u{2014} the desc\n";
        let f = parse_str(content, "t6");
        let s = &f.phases[0].steps[0];
        assert_eq!(s.bold_name.as_deref(), Some("Step Name"));
        assert_eq!(s.description, "the desc");
    }

    #[test]
    fn step_no_bold() {
        let content = "## P\n- [ ] plain text\n";
        let f = parse_str(content, "t7");
        let s = &f.phases[0].steps[0];
        assert!(s.bold_name.is_none());
        assert_eq!(s.description, "plain text");
    }

    #[test]
    fn step_multiple_dashes() {
        let content = "## P\n- [x] **Name** \u{2014} part one \u{2014} part two\n";
        let f = parse_str(content, "t8");
        let s = &f.phases[0].steps[0];
        assert_eq!(s.bold_name.as_deref(), Some("Name"));
        assert_eq!(s.description, "part one \u{2014} part two");
    }

    #[test]
    fn phase_grouping() {
        let content = "## Alpha\n- [ ] a1\n## Beta\n- [x] b1\n- [ ] b2\n";
        let f = parse_str(content, "t9");
        assert_eq!(f.phases.len(), 2);
        assert_eq!(f.phases[0].name, "Alpha");
        assert_eq!(f.phases[0].steps.len(), 1);
        assert_eq!(f.phases[1].name, "Beta");
        assert_eq!(f.phases[1].steps.len(), 2);
    }

    #[test]
    fn empty_file() {
        let f = parse_str("", "t10");
        assert_eq!(f.done, 0);
        assert_eq!(f.total, 0);
        assert!(f.phases.is_empty());
    }

    #[test]
    fn unknown_marker_uppercase() {
        let content = "## P\n- [X] a\n";
        let f = parse_str(content, "t11");
        assert_eq!(f.phases[0].steps[0].marker, Marker::Done);
    }

    #[test]
    fn parse_directory_sorted() {
        let dir = std::env::temp_dir().join("vigil_test_dir_sort");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("z-plan.todo.md"), "# Plan: Zebra\n").unwrap();
        fs::write(dir.join("a-plan.todo.md"), "# Plan: Apple\n").unwrap();
        let files = parse_directory(&dir).unwrap();
        fs::remove_dir_all(&dir).unwrap();
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].title, "Apple");
        assert_eq!(files[1].title, "Zebra");
    }
}
