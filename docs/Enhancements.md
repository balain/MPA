# Focus Enhancement Plan

## Milestone A: Core Usability

### Phase 1: Stable Task IDs

- Add Obsidian Tasks `🆔` support to the task model and parser.
- Generate compact stable IDs for every task captured by Focus.
- Add `focus ids backfill` to add IDs to existing open tasks while preserving existing IDs.
- Update task mutation paths to prefer task ID over file and line references.
- Keep file and line completion as a compatibility fallback.

Acceptance checks:

- Task IDs parse from Markdown.
- Captured tasks include IDs.
- Backfill preserves existing IDs.
- Updating by ID still works after a task moves to another line.

### Phase 2: Project Support

- Parse `#project/name` tags into task project metadata.
- Add `projects_folder` to config, defaulting to `Projects`.
- Add project commands:
  - `focus projects`
  - `focus project <name>`
  - `focus capture ... --project <name>`
- Show project summaries with open, overdue, due-soon, and waiting counts.
- Include project grouping in daily planning output.

Acceptance checks:

- Project tags parse correctly.
- Capture writes a project tag.
- Project summaries are generated from a fake vault.
- Daily plan includes project grouping.

### Phase 3: Real Today TUI

- Upgrade `focus today` from a static summary to a keyboard-driven command center.
- Add sections:
  - Inbox
  - Today
  - Overdue
  - Scheduled
  - Waiting
  - Projects
- Add task actions:
  - mark done
  - defer to tomorrow
  - schedule for today
  - refresh
- Use task IDs for all mutations.
- Keep non-interactive output readable and script-friendly.

Acceptance checks:

- TUI state navigation is tested without requiring an interactive terminal.
- Task actions mutate by ID.
- Non-interactive `focus today` still prints a summary.

## Milestone B: Organization and Visibility

### Phase 4: Obsidian Opener

- Add `obsidian_vault_name` to config.
- Add `focus open task <id>`, `focus open project <name>`, `focus open person <name>`, and `focus open today`.
- Generate `obsidian://open` URLs and invoke macOS `open`.
- Include dry-run output for testing.

### Phase 5: People CRM-Lite

- Add `focus people` and `focus person <name>`.
- Summarize waiting items, recent contact context, stale followups, and related projects.
- Add `focus waiting add --person <name> <text>`.

### Phase 6: Search and Reports

- Add `focus search <query>`, `focus status`, and `focus report weekly`.
- Report overdue, due today, waiting, stale people, active projects, completed tasks, and neglected projects.

## Milestone C: Polish and Safety

### Phase 7: Interactive Task Selection

- Add a searchable picker for commands such as done, defer, schedule, and open.
- Display task text, due date, project, person, and source.
- Return task IDs from selections.

### Phase 8: Safer Vault Writes

- Add `--dry-run` to write commands.
- Add optional backups before mutation.
- Detect ambiguous IDs, missing files, and changed task lines.
- Centralize writes through atomic temp-file replacement.

### Phase 9: Review Loops

- Add `focus morning`, `focus midday`, and `focus evening`.
- Persist structured review sections in the daily note.
- Carry forward relevant tasks into tomorrow candidates.

### Phase 10: Notification Quality

- Add quiet hours, notification dedupe, max notifications per run, and priority thresholds.
- Add explicit reminder metadata such as `🔔 YYYY-MM-DD HH:MM`.

## Milestone D: Advanced Automation

### Phase 11: Natural Language Capture

- Add `focus add <text>` for smart capture.
- Parse dates, priorities, tags, people, projects, and waiting-on intent.
- Keep explicit flags as overrides.

### Phase 12: Recurring Tasks

- Parse Obsidian Tasks recurrence syntax.
- Generate the next occurrence when completing recurring tasks.
- Assign a new stable ID to generated task instances.
