use crate::config::Config;
use crate::task::Task;
use crate::vault;
use anyhow::Result;
use chrono::NaiveDate;
use std::collections::BTreeMap;

#[derive(Debug, Default, Clone)]
pub struct Plan {
    pub overdue: Vec<Task>,
    pub due_today: Vec<Task>,
    pub scheduled_today: Vec<Task>,
    pub inbox: Vec<Task>,
    pub waiting: Vec<Task>,
    pub projects: BTreeMap<String, Vec<Task>>,
}

impl Plan {
    pub fn total(&self) -> usize {
        self.overdue.len()
            + self.due_today.len()
            + self.scheduled_today.len()
            + self.inbox.len()
            + self.waiting.len()
            + self.projects.values().map(Vec::len).sum::<usize>()
    }
}

pub fn build_plan(tasks: &[Task], today: NaiveDate) -> Plan {
    let mut plan = Plan::default();
    for task in tasks.iter().filter(|task| task.is_open()) {
        if task.due.is_some_and(|due| due < today) {
            plan.overdue.push(task.clone());
        }
        if task.due == Some(today) {
            plan.due_today.push(task.clone());
        }
        if task.scheduled == Some(today) {
            plan.scheduled_today.push(task.clone());
        }
        if task.is_waiting() {
            plan.waiting.push(task.clone());
        }
        if let Some(project) = &task.project {
            plan.projects
                .entry(project.clone())
                .or_default()
                .push(task.clone());
        }
        if task.path.to_string_lossy().contains(&today.to_string())
            && task.tags.iter().all(|tag| tag != "#waiting")
        {
            plan.inbox.push(task.clone());
        }
    }
    plan
}

pub fn write_daily_plan(config: &Config, today: NaiveDate) -> Result<Plan> {
    let tasks = vault::scan_tasks(config)?;
    let plan = build_plan(&tasks, today);
    let path = vault::daily_note_path(config, today);
    vault::replace_section(&path, "plan", &render_plan(&plan, today))?;
    Ok(plan)
}

pub fn render_plan(plan: &Plan, today: NaiveDate) -> String {
    let mut out = format!("## Focus Plan - {}\n", today);
    render_group(&mut out, "Overdue", &plan.overdue);
    render_group(&mut out, "Due Today", &plan.due_today);
    render_group(&mut out, "Scheduled Today", &plan.scheduled_today);
    render_group(&mut out, "Waiting", &plan.waiting);
    render_project_groups(&mut out, &plan.projects);
    render_group(&mut out, "Inbox", &plan.inbox);
    out.trim_end().to_string()
}

pub fn render_summary(plan: &Plan) -> String {
    format!(
        "Overdue: {}\nDue today: {}\nScheduled today: {}\nWaiting: {}\nProjects: {}\nInbox: {}\nTotal: {}",
        plan.overdue.len(),
        plan.due_today.len(),
        plan.scheduled_today.len(),
        plan.waiting.len(),
        plan.projects.len(),
        plan.inbox.len(),
        plan.total()
    )
}

fn render_project_groups(out: &mut String, projects: &BTreeMap<String, Vec<Task>>) {
    out.push_str("\n### Projects\n");
    if projects.is_empty() {
        out.push_str("- None\n");
    } else {
        for (project, tasks) in projects {
            out.push_str(&format!("\n#### {project}\n"));
            for task in tasks {
                out.push_str(&format!(
                    "- [ ] {} <!-- {} -->\n",
                    task.text,
                    task.source_ref()
                ));
            }
        }
    }
}

fn render_group(out: &mut String, title: &str, tasks: &[Task]) {
    out.push_str(&format!("\n### {title}\n"));
    if tasks.is_empty() {
        out.push_str("- None\n");
    } else {
        for task in tasks {
            out.push_str(&format!(
                "- [ ] {} <!-- {} -->\n",
                task.text,
                task.source_ref()
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn classifies_overdue_due_and_waiting() {
        let today = NaiveDate::from_ymd_opt(2026, 4, 25).unwrap();
        let tasks = vec![
            crate::task::parse_task_line(PathBuf::from("a.md"), 1, "- [ ] Late 📅 2026-04-24")
                .unwrap(),
            crate::task::parse_task_line(PathBuf::from("b.md"), 1, "- [ ] Today 📅 2026-04-25")
                .unwrap(),
            crate::task::parse_task_line(
                PathBuf::from("c.md"),
                1,
                "- [ ] Waiting on Bob: reply #waiting",
            )
            .unwrap(),
        ];
        let plan = build_plan(&tasks, today);
        assert_eq!(plan.overdue.len(), 1);
        assert_eq!(plan.due_today.len(), 1);
        assert_eq!(plan.waiting.len(), 1);
    }

    #[test]
    fn groups_project_tasks() {
        let today = NaiveDate::from_ymd_opt(2026, 4, 25).unwrap();
        let tasks = vec![
            crate::task::parse_task_line(
                PathBuf::from("a.md"),
                1,
                "- [ ] Build thing #project/mpa",
            )
            .unwrap(),
        ];
        let plan = build_plan(&tasks, today);
        assert_eq!(plan.projects.get("mpa").unwrap().len(), 1);
    }
}
