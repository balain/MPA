# Focus

Focus is a macOS-first command-line productivity app for Obsidian vaults. It helps capture tasks, plan the day, review open work, track waiting-on items, group tasks by project, and surface reminders while keeping durable data in plain Markdown.

Focus reads and writes vault files directly. It does not require an Obsidian plugin.

## Status

Focus is an early Rust implementation. The core CLI, task parsing, daily note capture, project summaries, waiting ledger, notifications, and a basic terminal UI are implemented.

## Features

- Capture tasks into today's Obsidian daily note.
- Use Obsidian Tasks-compatible Markdown checkbox syntax.
- Add stable task IDs with `🆔`.
- Backfill IDs onto existing open tasks.
- Parse due, scheduled, start, created, done, priority, project, waiting, and person metadata.
- Build a daily plan from overdue, due-today, scheduled, waiting, project, and inbox tasks.
- Track waiting-on items with `#waiting`.
- Group project work with `#project/name`.
- Generate a central waiting ledger.
- Show project summaries and project detail reports.
- Mark tasks done by stable task ID.
- Use a terminal UI for daily review and task actions.
- Send macOS notifications through the local notification system.
- Install a macOS LaunchAgent for periodic notification checks.

## Data Model

Focus stores tasks as normal Markdown checklist items:

```md
- [ ] Send agenda to Alice ⏳ 2026-04-25 📅 2026-04-26 #work 🆔 f123
- [x] File taxes 📅 2026-04-15 ✅ 2026-04-20 🆔 f124
```

Supported Obsidian Tasks metadata:

| Meaning | Syntax |
| --- | --- |
| Due date | `📅 YYYY-MM-DD` |
| Scheduled date | `⏳ YYYY-MM-DD` |
| Start date | `🛫 YYYY-MM-DD` |
| Created date | `➕ YYYY-MM-DD` |
| Done date | `✅ YYYY-MM-DD` |
| Stable ID | `🆔 <id>` |
| Highest priority | `🔺` |
| High priority | `⏫` |
| Medium priority | `🔼` |
| Low priority | `🔽` |
| Lowest priority | `⏬` |

Waiting-on tasks use `#waiting`:

```md
- [ ] Waiting on Alice: approve proposal 📅 2026-04-30 #waiting 🆔 f125
```

Project tasks use `#project/name`:

```md
- [ ] Draft milestone plan #project/product-launch 🆔 f126
```

## Vault Layout

Default vault-relative layout:

```text
Daily Notes/
  YYYY-MM-DD DDD.md
People/
Projects/
Waiting.md
```

Daily notes use this format:

```text
Daily Notes/2026-04-26 Sun.md
```

Generated Focus sections are bounded by markers:

```md
<!-- focus:plan:start -->
...
<!-- focus:plan:end -->
```

Focus only replaces content inside its own generated markers. It ignores generated plan and ledger sections while scanning source tasks, except for the capture inbox.

## Installation

Build and install with Cargo:

```sh
cargo install --path . --force
```

Then run:

```sh
focus --help
```

## Configuration

Initialize Focus for a vault:

```sh
focus init --vault /path/to/vault
```

Optional flags:

```sh
focus init \
  --vault /path/to/vault \
  --daily-folder "Daily Notes" \
  --people-folder "People" \
  --projects-folder "Projects" \
  --ledger-path "Waiting.md" \
  --notifications
```

Configuration is stored in the user config directory under `focus/config.toml`.

Example config:

```toml
vault_path = "/path/to/vault"
daily_folder = "Daily Notes"
people_folder = "People"
projects_folder = "Projects"
ledger_path = "Waiting.md"
notifications_enabled = true
stale_waiting_days = 7
```

For tests or automation, `FOCUS_CONFIG` can point to an alternate config file.

## Commands

### `focus init`

Creates the config file and configured folders.

```sh
focus init --vault /path/to/vault
```

### `focus capture <text>`

Captures a task into today's daily note.

```sh
focus capture "Draft launch plan"
```

Supported flags:

```sh
focus capture "Draft launch plan" \
  --due 2026-04-30 \
  --scheduled 2026-04-28 \
  --start 2026-04-27 \
  --priority high \
  --project "Product Launch"
```

Capture a waiting-on item:

```sh
focus capture "Approve proposal" --waiting --person "Alice" --due 2026-04-30
```

### `focus ids backfill`

Adds stable `🆔` IDs to existing open tasks that do not have one.

```sh
focus ids backfill
```

Run this once before using ID-based task actions on older vault tasks.

### `focus plan`

Scans the vault and writes today's generated Focus plan into today's daily note.

```sh
focus plan
```

The plan includes:

- overdue tasks
- due-today tasks
- scheduled-today tasks
- waiting-on tasks
- project groups
- inbox tasks

### `focus today`

Opens the daily command center terminal UI.

```sh
focus today
```

Controls:

| Key | Action |
| --- | --- |
| `tab` | Next section |
| `shift-tab` | Previous section |
| `up` / `down` | Move task selection |
| `d` | Mark selected task done |
| `f` | Defer selected task to tomorrow |
| `s` | Schedule selected task for today |
| `r` | Refresh from vault |
| `q` | Quit |

Sections:

- Inbox
- Today
- Overdue
- Scheduled
- Waiting
- Projects

Task actions require a stable task ID. New captured tasks get IDs automatically; existing tasks can be updated with `focus ids backfill`.

### `focus waiting`

Builds and writes the central waiting ledger.

```sh
focus waiting
```

Waiting items are grouped by person when Focus can infer a person from the task text or tags.

### `focus projects`

Lists active projects discovered from `#project/name` tags.

```sh
focus projects
```

Each project summary includes:

- open task count
- overdue task count
- due-soon task count
- waiting task count

### `focus project <name>`

Shows open tasks for one project.

```sh
focus project product-launch
```

### `focus done`

Marks a task done.

Preferred:

```sh
focus done --id <task-id>
```

Fallback:

```sh
focus done --file path/to/note.md --line 42
```

### `focus review`

Prints a simple end-of-day style summary:

```sh
focus review
```

Includes:

- completed today
- due or overdue remaining
- waiting items

### `focus notify run`

Computes notification candidates and sends macOS notifications.

```sh
focus notify run
```

Dry run:

```sh
focus notify run --dry-run
```

Notification candidates include:

- open tasks overdue before today
- open tasks due today
- open tasks scheduled today
- waiting-on tasks due today or earlier

### `focus notify install`

Writes a macOS LaunchAgent plist that periodically runs notification checks.

```sh
focus notify install
```

## Safety Rules

- Focus writes only to the configured vault.
- Generated sections are replaced only between Focus-owned markers.
- Task completion updates only the target task line.
- ID-based mutations reject missing or duplicate task IDs.
- Generated plan and ledger sections are skipped during source task scanning.
- `target/` build output is ignored by Git.

## Development

Run tests:

```sh
cargo test
```

Format:

```sh
cargo fmt
```

Build:

```sh
cargo build
```

## Roadmap

Planned enhancements include:

- Obsidian URL opener.
- People-focused reports.
- Search and weekly reports.
- Interactive task picker.
- Safer dry-run and backup flows.
- Morning, midday, and evening review loops.
- Notification quiet hours and deduplication.
- Natural-language capture.
- Recurring task handling.

See `docs/Enhancements.md` for the detailed enhancement plan.
