use chrono::NaiveDate;
use regex::Regex;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Lowest,
    Low,
    Medium,
    High,
    Highest,
}

impl Priority {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "lowest" => Some(Self::Lowest),
            "low" => Some(Self::Low),
            "medium" => Some(Self::Medium),
            "high" => Some(Self::High),
            "highest" => Some(Self::Highest),
            _ => None,
        }
    }

    pub fn emoji(self) -> &'static str {
        match self {
            Self::Highest => "🔺",
            Self::High => "⏫",
            Self::Medium => "🔼",
            Self::Low => "🔽",
            Self::Lowest => "⏬",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    pub path: PathBuf,
    pub line_number: usize,
    pub indent: String,
    pub marker: char,
    pub status: char,
    pub text: String,
    pub raw: String,
    pub due: Option<NaiveDate>,
    pub scheduled: Option<NaiveDate>,
    pub start: Option<NaiveDate>,
    pub created: Option<NaiveDate>,
    pub done: Option<NaiveDate>,
    pub id: Option<String>,
    pub priority: Option<Priority>,
    pub tags: Vec<String>,
    pub person: Option<String>,
    pub project: Option<String>,
}

impl Task {
    pub fn is_open(&self) -> bool {
        self.status == ' ' || self.status == '/'
    }

    pub fn is_done(&self) -> bool {
        self.status == 'x' || self.status == 'X'
    }

    pub fn is_waiting(&self) -> bool {
        self.tags
            .iter()
            .any(|tag| tag == "#waiting" || tag.starts_with("#waiting/"))
    }

    pub fn source_ref(&self) -> String {
        if let Some(id) = &self.id {
            format!("🆔 {}", id)
        } else {
            format!("{}:{}", self.path.display(), self.line_number)
        }
    }
}

pub fn parse_task_line(path: PathBuf, line_number: usize, line: &str) -> Option<Task> {
    static TASK_RE: OnceLock<Regex> = OnceLock::new();
    let task_re = TASK_RE.get_or_init(|| {
        Regex::new(r"^(?P<indent>\s*)(?P<marker>[-*+]) \[(?P<status>.{1})\] (?P<text>.*)$")
            .expect("valid task regex")
    });
    let caps = task_re.captures(line)?;
    let text = caps.name("text")?.as_str().to_string();
    let tags = extract_tags(&text);
    Some(Task {
        path,
        line_number,
        indent: caps.name("indent")?.as_str().to_string(),
        marker: caps.name("marker")?.as_str().chars().next()?,
        status: caps.name("status")?.as_str().chars().next()?,
        due: extract_date(&text, '📅'),
        scheduled: extract_date(&text, '⏳'),
        start: extract_date(&text, '🛫'),
        created: extract_date(&text, '➕'),
        done: extract_date(&text, '✅'),
        id: extract_id(&text),
        priority: extract_priority(&text),
        person: extract_person(&text, &tags),
        project: extract_project(&tags),
        text,
        raw: line.to_string(),
        tags,
    })
}

pub fn build_task_line(
    text: &str,
    due: Option<NaiveDate>,
    scheduled: Option<NaiveDate>,
    start: Option<NaiveDate>,
    priority: Option<Priority>,
    person: Option<&str>,
    project: Option<&str>,
    waiting: bool,
) -> String {
    build_task_line_with_id(
        text,
        due,
        scheduled,
        start,
        priority,
        person,
        project,
        waiting,
        Some(&generate_task_id()),
    )
}

pub fn build_task_line_with_id(
    text: &str,
    due: Option<NaiveDate>,
    scheduled: Option<NaiveDate>,
    start: Option<NaiveDate>,
    priority: Option<Priority>,
    person: Option<&str>,
    project: Option<&str>,
    waiting: bool,
    id: Option<&str>,
) -> String {
    let mut content = String::new();
    if waiting {
        if let Some(person) = person {
            content.push_str(&format!("Waiting on {}: {}", person, text));
        } else {
            content.push_str(text);
        }
    } else {
        content.push_str(text);
    }
    if let Some(priority) = priority {
        content.push(' ');
        content.push_str(priority.emoji());
    }
    if let Some(start) = start {
        content.push_str(&format!(" 🛫 {}", start));
    }
    if let Some(scheduled) = scheduled {
        content.push_str(&format!(" ⏳ {}", scheduled));
    }
    if let Some(due) = due {
        content.push_str(&format!(" 📅 {}", due));
    }
    if waiting && !content.contains("#waiting") {
        content.push_str(" #waiting");
    }
    if let Some(project) = project {
        content.push_str(&format!(" #project/{}", slug(project)));
    }
    if let Some(id) = id {
        if !content.contains("🆔") {
            content.push_str(&format!(" 🆔 {}", id));
        }
    }
    format!("- [ ] {}", content)
}

pub fn complete_line(line: &str, today: NaiveDate) -> Option<String> {
    let task = parse_task_line(PathBuf::new(), 1, line)?;
    let mut updated = format!("{}{} [x] {}", task.indent, task.marker, task.text);
    if task.done.is_none() {
        updated.push_str(&format!(" ✅ {}", today));
    }
    Some(updated)
}

pub fn line_with_id(line: &str, id: &str) -> Option<String> {
    let task = parse_task_line(PathBuf::new(), 1, line)?;
    if task.id.is_some() {
        return Some(line.to_string());
    }
    Some(format!("{line} 🆔 {id}"))
}

pub fn line_with_scheduled(line: &str, date: NaiveDate) -> Option<String> {
    let _ = parse_task_line(PathBuf::new(), 1, line)?;
    Some(replace_or_append_date(line, '⏳', date))
}

pub fn line_with_due(line: &str, date: NaiveDate) -> Option<String> {
    let _ = parse_task_line(PathBuf::new(), 1, line)?;
    Some(replace_or_append_date(line, '📅', date))
}

pub fn generate_task_id() -> String {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("f{:x}{:x}", duration.as_secs(), duration.subsec_nanos())
}

fn extract_date(text: &str, marker: char) -> Option<NaiveDate> {
    let needle = marker.to_string();
    let start = text.find(&needle)? + needle.len();
    let rest = text[start..].trim_start();
    let date = rest.get(0..10)?;
    NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()
}

fn extract_priority(text: &str) -> Option<Priority> {
    if text.contains("🔺") {
        Some(Priority::Highest)
    } else if text.contains("⏫") {
        Some(Priority::High)
    } else if text.contains("🔼") {
        Some(Priority::Medium)
    } else if text.contains("🔽") {
        Some(Priority::Low)
    } else if text.contains("⏬") {
        Some(Priority::Lowest)
    } else {
        None
    }
}

fn extract_id(text: &str) -> Option<String> {
    static ID_RE: OnceLock<Regex> = OnceLock::new();
    let id_re = ID_RE.get_or_init(|| Regex::new(r"🆔\s+(?P<id>\S+)").expect("valid ID regex"));
    id_re
        .captures(text)
        .and_then(|caps| caps.name("id").map(|value| value.as_str().to_string()))
}

fn extract_tags(text: &str) -> Vec<String> {
    text.split_whitespace()
        .filter(|part| part.starts_with('#') && part.len() > 1)
        .map(|part| {
            part.trim_matches(|c: char| c == ',' || c == '.' || c == ';')
                .to_string()
        })
        .collect()
}

fn extract_project(tags: &[String]) -> Option<String> {
    tags.iter().find_map(|tag| {
        tag.strip_prefix("#project/")
            .map(|value| value.replace('-', " "))
    })
}

fn extract_person(text: &str, tags: &[String]) -> Option<String> {
    static WAITING_RE: OnceLock<Regex> = OnceLock::new();
    let waiting_re = WAITING_RE.get_or_init(|| {
        Regex::new(r"(?i)^waiting (?:on|for) (?P<name>[^:]+):").expect("valid waiting regex")
    });
    if let Some(caps) = waiting_re.captures(text) {
        return Some(caps.name("name")?.as_str().trim().to_string());
    }
    tags.iter().find_map(|tag| {
        tag.strip_prefix("#waiting/")
            .map(|value| value.replace('-', " "))
    })
}

fn slug(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn replace_or_append_date(line: &str, marker: char, date: NaiveDate) -> String {
    let marker_string = marker.to_string();
    if let Some(start) = line.find(&marker_string) {
        let date_start = start + marker_string.len();
        let rest = &line[date_start..];
        let whitespace_len = rest.len() - rest.trim_start().len();
        let actual_date_start = date_start + whitespace_len;
        if line
            .get(actual_date_start..actual_date_start + 10)
            .is_some()
            && NaiveDate::parse_from_str(
                &line[actual_date_start..actual_date_start + 10],
                "%Y-%m-%d",
            )
            .is_ok()
        {
            return format!(
                "{}{}{}",
                &line[..actual_date_start],
                date,
                &line[actual_date_start + 10..]
            );
        }
    }
    format!("{line} {marker} {date}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_obsidian_task_metadata() {
        let task = parse_task_line(
            PathBuf::from("Daily Notes/2026-04-25 Sat.md"),
            3,
            "- [ ] Waiting on Alice: approve draft ⏳ 2026-04-25 📅 2026-04-26 ⏫ #waiting #work",
        )
        .unwrap();
        assert!(task.is_open());
        assert_eq!(task.person.as_deref(), Some("Alice"));
        assert_eq!(task.priority, Some(Priority::High));
        assert_eq!(task.scheduled.unwrap().to_string(), "2026-04-25");
        assert_eq!(task.due.unwrap().to_string(), "2026-04-26");
        assert!(task.is_waiting());
    }

    #[test]
    fn parses_task_id_and_project() {
        let task = parse_task_line(
            PathBuf::from("Projects/client.md"),
            1,
            "- [ ] Draft plan #project/client-work 🆔 abc123",
        )
        .unwrap();
        assert_eq!(task.id.as_deref(), Some("abc123"));
        assert_eq!(task.project.as_deref(), Some("client work"));
    }

    #[test]
    fn build_task_line_adds_id() {
        let line = build_task_line_with_id(
            "Write brief",
            None,
            None,
            None,
            None,
            None,
            Some("Client Work"),
            false,
            Some("abc123"),
        );
        assert!(line.contains("#project/client-work"));
        assert!(line.contains("🆔 abc123"));
    }

    #[test]
    fn completes_line_and_adds_done_date() {
        let today = NaiveDate::from_ymd_opt(2026, 4, 25).unwrap();
        let line = complete_line("- [ ] File taxes 📅 2026-04-15", today).unwrap();
        assert_eq!(line, "- [x] File taxes 📅 2026-04-15 ✅ 2026-04-25");
    }
}
