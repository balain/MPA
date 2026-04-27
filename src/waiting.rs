use crate::config::Config;
use crate::task::Task;
use crate::vault;
use anyhow::Result;
use chrono::NaiveDate;
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct WaitingLedger {
    pub groups: BTreeMap<String, Vec<Task>>,
}

impl WaitingLedger {
    pub fn count(&self) -> usize {
        self.groups.values().map(Vec::len).sum()
    }
}

pub fn build_ledger(tasks: &[Task]) -> WaitingLedger {
    let mut groups: BTreeMap<String, Vec<Task>> = BTreeMap::new();
    for task in tasks
        .iter()
        .filter(|task| task.is_open() && task.is_waiting())
    {
        let person = task
            .person
            .clone()
            .unwrap_or_else(|| "Unassigned".to_string());
        groups.entry(person).or_default().push(task.clone());
    }
    WaitingLedger { groups }
}

pub fn write_ledger(config: &Config, today: NaiveDate) -> Result<WaitingLedger> {
    let tasks = vault::scan_tasks(config)?;
    let ledger = build_ledger(&tasks);
    vault::replace_section(
        &config.ledger_file(),
        "waiting-ledger",
        &render_ledger(&ledger, today),
    )?;
    Ok(ledger)
}

pub fn render_ledger(ledger: &WaitingLedger, today: NaiveDate) -> String {
    let mut out = format!("## Waiting Ledger - {}\n", today);
    if ledger.groups.is_empty() {
        out.push_str("\nNo waiting-on items.\n");
        return out.trim_end().to_string();
    }
    for (person, tasks) in &ledger.groups {
        out.push_str(&format!("\n### {person}\n"));
        for task in tasks {
            out.push_str(&format!(
                "- [ ] {} <!-- {} -->\n",
                task.text,
                task.source_ref()
            ));
        }
    }
    out.trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn groups_waiting_items_by_person() {
        let tasks = vec![
            crate::task::parse_task_line(
                PathBuf::from("a.md"),
                1,
                "- [ ] Waiting on Alice: feedback #waiting",
            )
            .unwrap(),
            crate::task::parse_task_line(
                PathBuf::from("b.md"),
                1,
                "- [ ] Waiting on Bob: invoice #waiting",
            )
            .unwrap(),
        ];
        let ledger = build_ledger(&tasks);
        assert_eq!(ledger.count(), 2);
        assert!(ledger.groups.contains_key("Alice"));
        assert!(ledger.groups.contains_key("Bob"));
    }
}
