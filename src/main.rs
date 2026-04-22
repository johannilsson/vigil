mod list_view;
mod parser;
mod term;
mod watch_view;

use std::io;
use std::path::PathBuf;

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1) {
        None => run_list_mode(std::env::current_dir()?),
        Some(p) => {
            let path = PathBuf::from(p);
            if !path.exists() {
                eprintln!("vigil: path not found: {}", path.display());
                std::process::exit(1);
            }
            if path.is_dir() {
                run_list_mode(path)
            } else {
                watch_view::run(&path, false)
            }
        }
    }
}

fn run_list_mode(dir: PathBuf) -> io::Result<()> {
    let mut cursor: usize = 0;
    loop {
        match list_view::run(&dir, cursor)? {
            None => return Ok(()),
            Some((path, selected)) => {
                cursor = selected;
                watch_view::run(&path, true)?;
            }
        }
    }
}
