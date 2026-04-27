# Focus Architecture

## Decisions

### ADR-001: Rust CLI

Focus is implemented in Rust for a fast single-binary CLI, reliable filesystem behavior, strong domain modeling, and good long-term maintainability.

### ADR-002: Direct Vault Files

Focus reads and writes Markdown files directly in the Obsidian vault. This avoids plugin setup, keeps data portable, and makes the CLI usable when Obsidian is closed.

### ADR-003: Line-Preserving Task Parser

Focus parses checkbox task lines directly instead of relying on a Markdown AST for edits. The parser preserves file path, line number, raw line, checkbox status, indentation, tags, dates, and priority so commands can safely update individual lines.

### ADR-004: Generated Section Markers

Focus-owned sections use HTML comment markers. Commands replace only bounded generated sections and leave surrounding note content unchanged.

### ADR-005: macOS Notifications via `osascript`

The initial notification backend shells out to `osascript` rather than linking a platform notification crate. This keeps the delivery path transparent and easy to replace.

## Crates

- `clap`: CLI parsing
- `chrono`: date handling
- `serde` and `toml`: configuration
- `regex`: line and metadata parsing
- `walkdir`: vault scanning
- `ratatui` and `crossterm`: daily command center TUI
- `anyhow` and `thiserror`: errors

## Modules

- `cli`: command definitions
- `config`: config path, load, save, and path resolution
- `task`: task model and Obsidian Tasks metadata parsing
- `vault`: Markdown scanning and safe file updates
- `planner`: daily planning bucket computation and rendered plan sections
- `waiting`: waiting-on grouping and ledger rendering
- `notifications`: notification candidate selection and macOS delivery
- `tui`: interactive daily command center

## File Editing Strategy

- Read files as UTF-8 strings.
- Preserve line endings by writing normalized `\n` output.
- Complete tasks by replacing only the requested line.
- Replace generated sections with marker-aware string operations.
- Create missing parent folders for Focus-owned generated files.

## Task Identity

v1 now prefers stable Obsidian Tasks `🆔` IDs for task mutations. File and line references remain as a fallback for compatibility and debugging.

### ADR-006: Generated Sections Are Not Source Tasks

Focus scans the capture inbox as source content but skips generated plan and ledger sections. This prevents duplicated generated task lines from becoming additional mutable source tasks.

## Known Tradeoffs

- A line parser is safer for task edits but does not understand every Markdown edge case.
- `osascript` is macOS-specific by design.
- TUI selection is intentionally minimal in the first implementation; plain commands remain fully usable.

## Implementation Log

### 2026-04-25

- Implemented the initial Rust CLI package.
- Added config creation and loading with `FOCUS_CONFIG` test override.
- Added line-preserving task parsing for Obsidian Tasks emoji metadata.
- Added safe generated-section replacement and task-line completion.
- Added daily planning, waiting ledger generation, notification candidate selection, macOS notification delivery, LaunchAgent plist generation, and a minimal TUI command center.
- Added unit tests plus a fake-vault CLI workflow test covering init, capture, plan, waiting ledger generation, and done.

### 2026-04-26

- Added the enhancement roadmap in `docs/Enhancements.md`.
- Implemented stable task IDs with capture-time ID generation and `focus ids backfill`.
- Updated task mutation to support `focus done --id` while preserving file and line fallback.
- Added project parsing from `#project/name`, `projects_folder` config, and `focus projects` / `focus project <name>`.
- Expanded `focus today` into a sectioned TUI with task navigation and ID-based done, defer, schedule, and refresh actions.
- Updated scanning to ignore generated Focus sections except the capture inbox.
