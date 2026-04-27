# Focus Product and Technical Specification

## Purpose

Focus is a macOS-first command-line productivity app that uses an Obsidian vault as the source of truth. It helps with fast capture, daily planning, TODO review, waiting-on tracking, and proactive reminders without requiring an Obsidian plugin or external service.

## Goals

- Keep all durable user data in Markdown files inside the configured Obsidian vault.
- Use Obsidian Tasks-compatible checkbox syntax so notes remain useful without Focus.
- Make common workflows fast from the terminal while supporting an interactive daily command center.
- Track obligations from other people through person notes and a generated central waiting ledger.
- Surface overdue, due-today, scheduled, and stale waiting-on items through CLI output and macOS notifications.

## Non-Goals for v1

- No email, Messages, Slack, or Calendar integration.
- No background daemon beyond an optional macOS LaunchAgent that invokes the CLI.
- No database required for normal operation.
- No proprietary Obsidian plugin API dependency.

## Installation and Binary

- Binary name: `focus`.
- Distributed initially as a Cargo-built executable.
- Configuration is stored in `~/.config/focus/config.toml`.
- Tests may override the config path with `FOCUS_CONFIG`.

## Configuration

```toml
vault_path = "/Users/example/Obsidian"
daily_folder = "Daily Notes"
people_folder = "People"
projects_folder = "Projects"
ledger_path = "Waiting.md"
notifications_enabled = true
stale_waiting_days = 7
```

All folder and ledger paths are relative to `vault_path` unless absolute paths are supplied.

## Vault Layout

- Daily notes live in the configured daily folder and use `YYYY-MM-DD DDD.md`, for example `Daily Notes/2026-04-26 Sun.md`.
- People notes live in the configured people folder and use one note per person.
- Project notes live in the configured projects folder.
- The waiting ledger is generated at `ledger_path`.
- Focus-owned generated content is bounded by HTML comments:
  - `<!-- focus:<section>:start -->`
  - `<!-- focus:<section>:end -->`

## Task Format

Tasks are Markdown checkbox lines:

```markdown
- [ ] Send agenda to Alice ⏳ 2026-04-25 📅 2026-04-26 #work
- [x] File taxes 📅 2026-04-15 ✅ 2026-04-20
```

Supported Obsidian Tasks emoji metadata:

- Due date: `📅 YYYY-MM-DD`
- Scheduled date: `⏳ YYYY-MM-DD`
- Start date: `🛫 YYYY-MM-DD`
- Created date: `➕ YYYY-MM-DD`
- Done date: `✅ YYYY-MM-DD`
- Priority: `🔺`, `⏫`, `🔼`, `🔽`, `⏬`
- Stable task ID: `🆔 <id>`

Waiting-on tasks use `#waiting` and should preferably start with `Waiting on Name:` or `Waiting for Name:`.
Project tasks use `#project/name`.

## Commands

### `focus init`

Creates the config file. Supports flags for non-interactive use:

- `--vault <path>`
- `--daily-folder <folder>`
- `--people-folder <folder>`
- `--projects-folder <folder>`
- `--ledger-path <path>`
- `--notifications`

The command creates configured folders when possible.

### `focus capture <text>`

Appends a task to today’s daily note inbox. Supported flags:

- `--due YYYY-MM-DD`
- `--scheduled YYYY-MM-DD`
- `--start YYYY-MM-DD`
- `--priority highest|high|medium|low|lowest`
- `--person <name>`
- `--project <name>`
- `--waiting`

Captured tasks are placed between the daily inbox markers.
Captured tasks include a generated `🆔` ID.

### `focus ids backfill`

Scans source Markdown task lines and adds missing `🆔` IDs to open tasks. Existing IDs are preserved. Generated Focus sections are skipped except for the capture inbox.

### `focus plan`

Scans the vault, computes daily planning buckets, and writes today’s generated plan section.

Buckets:

- Overdue open tasks
- Due today
- Scheduled today
- Waiting-on followups
- Inbox tasks
- Project groups

### `focus today`

Opens a keyboard-driven daily command center when attached to a terminal. In non-interactive contexts it prints the same summary as plain output.

Initial TUI controls:

- `q` exits
- `tab` and `shift-tab` change section
- Up/down changes selected task
- `d` marks the selected task done
- `f` defers the selected task to tomorrow
- `s` schedules the selected task for today
- `r` refreshes from the vault

### `focus projects`

Lists active projects found from `#project/name` task tags, including open, overdue, due-soon, and waiting counts.

### `focus project <name>`

Shows a project detail report with open project tasks and source references.

### `focus waiting`

Scans all vault tasks with `#waiting`, groups them by person, writes the central waiting ledger, and prints a summary.

### `focus done`

Marks a task complete in its source file and appends `✅ YYYY-MM-DD` when absent. The initial implementation accepts:

- `--id <task-id>`
- `--file <path>`
- `--line <line-number>`

Task ID is preferred. File and line remain available as a fallback.

### `focus review`

Prints completed-today items, remaining open items due today or earlier, and waiting-on items.

### `focus notify run`

Computes notification candidates and sends macOS notifications through `osascript`.

### `focus notify install`

Writes a LaunchAgent plist that invokes `focus notify run` periodically.

## Notification Rules

Notify for:

- open tasks due before today
- open tasks due today
- open tasks scheduled today
- open waiting-on tasks whose due date is today or earlier

Notification selection is independent from delivery so it can be unit-tested.

## Safe Editing Rules

- Task completion updates only the target line.
- Task-ID mutations scan the vault, reject missing IDs, and reject duplicate IDs.
- Generated sections are replaced only between Focus-owned markers.
- Vault scanning ignores generated Focus sections except the capture inbox.
- Capture appends within the Focus inbox section.
- Focus must not rewrite unrelated hand-authored note content.

## Acceptance Criteria

- `cargo test` passes.
- `focus init --vault <tmp>` creates usable config and folders.
- `focus capture "Task" --due YYYY-MM-DD` appends a valid Obsidian Tasks line.
- `focus ids backfill` adds IDs to existing open source tasks.
- `focus projects` and `focus project <name>` report project tasks.
- `focus plan` writes a generated daily plan section.
- `focus waiting` writes a grouped central ledger.
- `focus done --file <file> --line <n>` completes the source task line.
- `focus notify run --dry-run` prints notification candidates without sending notifications.
