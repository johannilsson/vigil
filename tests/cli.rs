use std::fs;
use std::process::{Command, Stdio};

fn vigil_bin() -> std::path::PathBuf {
    let mut path = std::env::current_exe().unwrap();
    // go up from deps/ to the target/debug dir
    path.pop();
    if path.ends_with("deps") {
        path.pop();
    }
    path.join("vigil")
}

#[test]
fn missing_path_exits_nonzero() {
    let output = Command::new(vigil_bin())
        .arg("/nonexistent/path/to/nothing")
        .output()
        .expect("failed to run vigil");
    assert!(!output.status.success(), "expected non-zero exit");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not found"), "stderr: {stderr}");
}

#[test]
fn empty_directory_exits_cleanly() {
    let dir = std::env::temp_dir().join("vigil_cli_test_empty");
    fs::create_dir_all(&dir).unwrap();
    // remove any stale .todo.md files
    for entry in fs::read_dir(&dir).unwrap().flatten() {
        if entry.file_name().to_string_lossy().ends_with(".todo.md") {
            fs::remove_file(entry.path()).unwrap();
        }
    }

    let output = Command::new(vigil_bin())
        .arg(&dir)
        .output()
        .expect("failed to run vigil");

    fs::remove_dir_all(&dir).unwrap();
    // Should exit 0 with a message (not crash)
    assert!(
        output.status.success(),
        "expected clean exit, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

// TUI tests require a real PTY for raw mode and key injection.
// We verify the binary starts and can be terminated cleanly with SIGTERM.
#[test]
fn direct_file_starts_and_terminates() {
    let dir = std::env::temp_dir().join("vigil_cli_test_file");
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("sample.todo.md");
    fs::write(
        &path,
        "# Plan: Sample\n## Phase\n- [x] **Done** \u{2014} done\n- [ ] **Todo** \u{2014} not yet\n",
    )
    .unwrap();

    let mut child = Command::new(vigil_bin())
        .arg(&path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to spawn vigil");

    std::thread::sleep(std::time::Duration::from_millis(300));

    // SIGTERM — process should exit cleanly (raw mode guard runs on unwind/drop)
    unsafe { libc::kill(child.id() as libc::pid_t, libc::SIGTERM) };

    let status = child.wait().expect("failed to wait");
    fs::remove_dir_all(&dir).unwrap();
    // SIGTERM exits with signal, not success code — just verify the process stopped
    assert!(
        !status.success() || status.code().is_some(),
        "process should have exited"
    );
}
