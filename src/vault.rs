use crate::config::Config;
use crate::task::{
    Task, complete_line, generate_task_id, line_with_due, line_with_id, line_with_scheduled,
    parse_task_line,
};
use anyhow::{Context, Result, anyhow};
use chrono::NaiveDate;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub fn scan_tasks(config: &Config) -> Result<Vec<Task>> {
    let mut tasks = Vec::new();
    for entry in WalkDir::new(&config.vault_path)
        .into_iter()
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if !entry.file_type().is_file()
            || path.extension().and_then(|ext| ext.to_str()) != Some("md")
        {
            continue;
        }
        if path
            .components()
            .any(|part| part.as_os_str() == ".obsidian")
        {
            continue;
        }
        let raw = fs::read_to_string(path)
            .with_context(|| format!("could not read Markdown file {}", path.display()))?;
        let mut focus_section: Option<String> = None;
        for (idx, line) in raw.lines().enumerate() {
            if let Some(section) = focus_section_start(line) {
                focus_section = Some(section);
                continue;
            }
            if focus_section_end(line) {
                focus_section = None;
                continue;
            }
            if focus_section
                .as_deref()
                .is_some_and(|section| section != "inbox")
            {
                continue;
            }
            if let Some(task) = parse_task_line(path.to_path_buf(), idx + 1, line) {
                tasks.push(task);
            }
        }
    }
    Ok(tasks)
}

pub fn daily_note_path(config: &Config, date: NaiveDate) -> PathBuf {
    config
        .daily_dir()
        .join(format!("{}.md", date.format("%Y-%m-%d %a")))
}

pub fn append_to_section(path: &Path, section: &str, heading: &str, line: &str) -> Result<()> {
    let start = marker_start(section);
    let end = marker_end(section);
    ensure_parent(path)?;
    let mut raw = if path.exists() {
        fs::read_to_string(path).with_context(|| format!("could not read {}", path.display()))?
    } else {
        String::new()
    };
    if let Some((_start_idx, end_marker_idx, _end_idx)) = find_section_parts(&raw, section) {
        let insert_at = end_marker_idx;
        let prefix = &raw[..insert_at];
        let suffix = &raw[insert_at..];
        let separator = if prefix.ends_with('\n') { "" } else { "\n" };
        raw = format!("{}{}{}\n{}", prefix, separator, line, suffix);
    } else {
        if !raw.is_empty() && !raw.ends_with('\n') {
            raw.push('\n');
        }
        if !raw.is_empty() {
            raw.push('\n');
        }
        raw.push_str(&format!("{start}\n## {heading}\n{line}\n{end}\n"));
    }
    fs::write(path, raw).with_context(|| format!("could not write {}", path.display()))
}

pub fn replace_section(path: &Path, section: &str, body: &str) -> Result<()> {
    ensure_parent(path)?;
    let start = marker_start(section);
    let end = marker_end(section);
    let raw = if path.exists() {
        fs::read_to_string(path).with_context(|| format!("could not read {}", path.display()))?
    } else {
        String::new()
    };
    let replacement = format!("{start}\n{body}\n{end}");
    let updated = if let Some((start_idx, end_idx)) = find_section(&raw, section) {
        format!("{}{}{}", &raw[..start_idx], replacement, &raw[end_idx..])
    } else {
        let mut next = raw;
        if !next.is_empty() && !next.ends_with('\n') {
            next.push('\n');
        }
        if !next.is_empty() {
            next.push('\n');
        }
        next.push_str(&replacement);
        next.push('\n');
        next
    };
    fs::write(path, updated).with_context(|| format!("could not write {}", path.display()))
}

pub fn complete_task_at(path: &Path, line_number: usize, today: NaiveDate) -> Result<()> {
    if line_number == 0 {
        return Err(anyhow!("line number must be 1 or greater"));
    }
    let raw =
        fs::read_to_string(path).with_context(|| format!("could not read {}", path.display()))?;
    let mut lines: Vec<String> = raw.lines().map(ToString::to_string).collect();
    let target = lines
        .get_mut(line_number - 1)
        .ok_or_else(|| anyhow!("{} does not have line {}", path.display(), line_number))?;
    let completed = complete_line(target, today)
        .ok_or_else(|| anyhow!("line {} in {} is not a task", line_number, path.display()))?;
    *target = completed;
    let mut updated = lines.join("\n");
    if raw.ends_with('\n') {
        updated.push('\n');
    }
    fs::write(path, updated).with_context(|| format!("could not write {}", path.display()))
}

pub fn complete_task_by_id(config: &Config, id: &str, today: NaiveDate) -> Result<Task> {
    update_task_line_by_id(config, id, |line| complete_line(line, today))
}

pub fn schedule_task_by_id(config: &Config, id: &str, date: NaiveDate) -> Result<Task> {
    update_task_line_by_id(config, id, |line| line_with_scheduled(line, date))
}

pub fn defer_task_by_id(config: &Config, id: &str, date: NaiveDate) -> Result<Task> {
    update_task_line_by_id(config, id, |line| line_with_due(line, date))
}

pub fn backfill_task_ids(config: &Config) -> Result<usize> {
    let mut changed = 0;
    for entry in WalkDir::new(&config.vault_path)
        .into_iter()
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if !entry.file_type().is_file()
            || path.extension().and_then(|ext| ext.to_str()) != Some("md")
        {
            continue;
        }
        if path
            .components()
            .any(|part| part.as_os_str() == ".obsidian")
        {
            continue;
        }

        let raw = fs::read_to_string(path)
            .with_context(|| format!("could not read Markdown file {}", path.display()))?;
        let mut lines: Vec<String> = raw.lines().map(ToString::to_string).collect();
        let mut focus_section: Option<String> = None;
        let mut file_changed = false;
        for (idx, line) in lines.iter_mut().enumerate() {
            if let Some(section) = focus_section_start(line) {
                focus_section = Some(section);
                continue;
            }
            if focus_section_end(line) {
                focus_section = None;
                continue;
            }
            if focus_section
                .as_deref()
                .is_some_and(|section| section != "inbox")
            {
                continue;
            }
            if let Some(task) = parse_task_line(path.to_path_buf(), idx + 1, line) {
                if task.is_open() && task.id.is_none() {
                    let id = generate_task_id();
                    *line = line_with_id(line, &id).expect("parsed line accepts ID");
                    file_changed = true;
                    changed += 1;
                }
            }
        }
        if file_changed {
            write_lines_preserving_final_newline(path, &lines, raw.ends_with('\n'))?;
        }
    }
    Ok(changed)
}

fn update_task_line_by_id<F>(config: &Config, id: &str, update: F) -> Result<Task>
where
    F: Fn(&str) -> Option<String>,
{
    let matches: Vec<Task> = scan_tasks(config)?
        .into_iter()
        .filter(|task| task.id.as_deref() == Some(id))
        .collect();
    if matches.is_empty() {
        return Err(anyhow!("no task found with ID {}", id));
    }
    if matches.len() > 1 {
        return Err(anyhow!("multiple tasks found with ID {}", id));
    }
    let task = matches.into_iter().next().expect("checked non-empty");
    let raw = fs::read_to_string(&task.path)
        .with_context(|| format!("could not read {}", task.path.display()))?;
    let mut lines: Vec<String> = raw.lines().map(ToString::to_string).collect();
    let target = lines.get_mut(task.line_number - 1).ok_or_else(|| {
        anyhow!(
            "{} does not have line {}",
            task.path.display(),
            task.line_number
        )
    })?;
    let updated = update(target).ok_or_else(|| {
        anyhow!(
            "line {} in {} is no longer a task",
            task.line_number,
            task.path.display()
        )
    })?;
    *target = updated;
    write_lines_preserving_final_newline(&task.path, &lines, raw.ends_with('\n'))?;
    Ok(task)
}

fn write_lines_preserving_final_newline(
    path: &Path,
    lines: &[String],
    final_newline: bool,
) -> Result<()> {
    let mut updated = lines.join("\n");
    if final_newline {
        updated.push('\n');
    }
    fs::write(path, updated).with_context(|| format!("could not write {}", path.display()))
}

fn ensure_parent(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("could not create directory {}", parent.display()))?;
    }
    Ok(())
}

fn marker_start(section: &str) -> String {
    format!("<!-- focus:{section}:start -->")
}

fn marker_end(section: &str) -> String {
    format!("<!-- focus:{section}:end -->")
}

fn find_section(raw: &str, section: &str) -> Option<(usize, usize)> {
    let (start_idx, _end_marker_idx, end_idx) = find_section_parts(raw, section)?;
    Some((start_idx, end_idx))
}

fn find_section_parts(raw: &str, section: &str) -> Option<(usize, usize, usize)> {
    let start = marker_start(section);
    let end = marker_end(section);
    let start_idx = raw.find(&start)?;
    let end_marker_idx = raw[start_idx..].find(&end)? + start_idx;
    Some((start_idx, end_marker_idx, end_marker_idx + end.len()))
}

fn focus_section_start(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let prefix = "<!-- focus:";
    let suffix = ":start -->";
    trimmed
        .strip_prefix(prefix)?
        .strip_suffix(suffix)
        .map(ToString::to_string)
}

fn focus_section_end(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with("<!-- focus:") && trimmed.ends_with(":end -->")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use tempfile::tempdir;

    #[test]
    fn replaces_only_generated_section() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("note.md");
        fs::write(
            &path,
            "before\n<!-- focus:plan:start -->\nold\n<!-- focus:plan:end -->\nafter\n",
        )
        .unwrap();
        replace_section(&path, "plan", "new").unwrap();
        let raw = fs::read_to_string(path).unwrap();
        assert_eq!(
            raw,
            "before\n<!-- focus:plan:start -->\nnew\n<!-- focus:plan:end -->\nafter\n"
        );
    }

    #[test]
    fn daily_note_path_uses_weekday_suffix() {
        let config = Config {
            vault_path: PathBuf::from("/vault"),
            ..Config::default()
        };
        let date = NaiveDate::from_ymd_opt(2026, 4, 26).unwrap();
        assert_eq!(
            daily_note_path(&config, date),
            PathBuf::from("/vault/Daily Notes/2026-04-26 Sun.md")
        );
    }
}
