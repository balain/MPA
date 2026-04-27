use crate::config::Config;
use crate::task::Task;
use crate::vault;
use anyhow::Result;
use chrono::{Duration, NaiveDate};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default)]
pub struct ProjectSummary {
    pub name: String,
    pub open: Vec<Task>,
    pub overdue: Vec<Task>,
    pub due_soon: Vec<Task>,
    pub waiting: Vec<Task>,
}

pub fn summarize(tasks: &[Task], today: NaiveDate) -> BTreeMap<String, ProjectSummary> {
    let mut summaries: BTreeMap<String, ProjectSummary> = BTreeMap::new();
    let due_soon_end = today + Duration::days(7);
    for task in tasks
        .iter()
        .filter(|task| task.is_open())
        .filter_map(|task| task.project.as_ref().map(|project| (project, task)))
    {
        let summary = summaries
            .entry(task.0.clone())
            .or_insert_with(|| ProjectSummary {
                name: task.0.clone(),
                ..ProjectSummary::default()
            });
        summary.open.push(task.1.clone());
        if task.1.due.is_some_and(|due| due < today) {
            summary.overdue.push(task.1.clone());
        }
        if task
            .1
            .due
            .is_some_and(|due| due >= today && due <= due_soon_end)
        {
            summary.due_soon.push(task.1.clone());
        }
        if task.1.is_waiting() {
            summary.waiting.push(task.1.clone());
        }
    }
    summaries
}

pub fn load_summaries(
    config: &Config,
    today: NaiveDate,
) -> Result<BTreeMap<String, ProjectSummary>> {
    let tasks = vault::scan_tasks(config)?;
    Ok(summarize(&tasks, today))
}

pub fn render_projects(summaries: &BTreeMap<String, ProjectSummary>) -> String {
    if summaries.is_empty() {
        return "No active projects.".to_string();
    }
    let mut out = String::new();
    for summary in summaries.values() {
        out.push_str(&format!(
            "{}: open {}, overdue {}, due soon {}, waiting {}\n",
            summary.name,
            summary.open.len(),
            summary.overdue.len(),
            summary.due_soon.len(),
            summary.waiting.len()
        ));
    }
    out.trim_end().to_string()
}

pub fn render_project(summary: &ProjectSummary) -> String {
    let mut out = format!(
        "{}\nOpen: {}\nOverdue: {}\nDue soon: {}\nWaiting: {}\n",
        summary.name,
        summary.open.len(),
        summary.overdue.len(),
        summary.due_soon.len(),
        summary.waiting.len()
    );
    out.push_str("\nTasks\n");
    for task in &summary.open {
        out.push_str(&format!("- {} ({})\n", task.text, task.source_ref()));
    }
    out.trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn summarizes_project_counts() {
        let today = NaiveDate::from_ymd_opt(2026, 4, 25).unwrap();
        let tasks = vec![
            crate::task::parse_task_line(
                PathBuf::from("a.md"),
                1,
                "- [ ] Late #project/client 📅 2026-04-24",
            )
            .unwrap(),
            crate::task::parse_task_line(
                PathBuf::from("b.md"),
                1,
                "- [ ] Waiting on Amy: reply #waiting #project/client",
            )
            .unwrap(),
        ];
        let summaries = summarize(&tasks, today);
        let client = summaries.get("client").unwrap();
        assert_eq!(client.open.len(), 2);
        assert_eq!(client.overdue.len(), 1);
        assert_eq!(client.waiting.len(), 1);
    }
}
