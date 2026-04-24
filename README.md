# Vigil

Terminal UI for viewing and navigating `.todo.md` plan files.

## Usage

```
vigil [path]
```

- **No path / directory** — browse all `*.todo.md` files in that directory
- **File path** — open the file directly in watch mode

## Watch mode

Polls for file changes at ~4 Hz and re-renders live. Useful for monitoring an agent working through a plan.

```
My Plan
[████████████░░░░░░░░░░░░░░░░░░░░░░░░░░] 3/7 steps complete
────────────────────────────────────────
Phase 1 — Setup
  ✓ Initialize repo — done
  ✓ Add dependencies — done
  ✓ Scaffold modules — done
Phase 2 — Implementation
  ◐ Parser — in progress
    · Verify: unit tests pass
    ✓ Verify: file roundtrips correctly
  ○ Terminal layer
  ○ List view
  ○ Watch view
────────────────────────────────────────
my-plan.todo.md  ↑↓ scroll  •  q back to list
```

Keys: `↑`/`↓` or `k`/`j` to scroll, `q` to go back (or quit).

## List mode

```
Todos
▶ My Plan                            3/7
  Another Plan                       0/4
```

Keys: `↑`/`↓` or `k`/`j` to move, `Enter` to open, `q` to quit.

## File format

```markdown
# Plan: My Plan

## Progress: 3/7

## Phase 1 — Setup
- [x] **Initialize repo** — create the git repo
- [x] **Add dependencies** — Cargo.toml
- [~] **Scaffold modules** — in progress
  - [ ] Verify: tests pass
  - [x] Verify: compiles cleanly

## Phase 2 — Implementation
- [ ] **Parser** — parse .todo.md files
- [-] **Old approach** — skipped
- [!] **Broken step** — reason it failed
```

| Marker | Symbol | Meaning  |
|--------|--------|----------|
| `[ ]`  | ○      | Pending  |
| `[~]`  | ◐      | In progress |
| `[x]`  | ✓      | Done     |
| `[!]`  | ✗      | Failed   |
| `[-]`  | –      | Skipped  |

### Sub-tasks

Indent list items by one or more spaces to create sub-tasks beneath a step:

```markdown
- [~] **Deploy** — push to production
  - [x] Verify: staging smoke test passed
  - [ ] Verify: metrics look healthy
```

Sub-tasks are displayed indented under their parent step and support all the same markers. They are not counted in the overall progress total — only top-level steps are.

## Install

```
cargo install --path .
```
