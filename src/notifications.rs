use crate::task::Task;
use anyhow::{Context, Result};
use chrono::NaiveDate;
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationCandidate {
    pub title: String,
    pub body: String,
}

pub fn candidates(tasks: &[Task], today: NaiveDate) -> Vec<NotificationCandidate> {
    let mut out = Vec::new();
    for task in tasks.iter().filter(|task| task.is_open()) {
        if task.due.is_some_and(|due| due < today) {
            out.push(NotificationCandidate {
                title: "Overdue task".to_string(),
                body: task.text.clone(),
            });
        } else if task.due == Some(today) {
            out.push(NotificationCandidate {
                title: "Task due today".to_string(),
                body: task.text.clone(),
            });
        } else if task.scheduled == Some(today) {
            out.push(NotificationCandidate {
                title: "Task scheduled today".to_string(),
                body: task.text.clone(),
            });
        } else if task.is_waiting() && task.due.is_some_and(|due| due <= today) {
            out.push(NotificationCandidate {
                title: "Waiting followup".to_string(),
                body: task.text.clone(),
            });
        }
    }
    out
}

pub fn send(candidate: &NotificationCandidate) -> Result<()> {
    let script = format!(
        "display notification {:?} with title {:?}",
        candidate.body, candidate.title
    );
    let status = Command::new("osascript")
        .arg("-e")
        .arg(script)
        .status()
        .context("could not invoke osascript")?;
    if !status.success() {
        anyhow::bail!("osascript failed with status {status}");
    }
    Ok(())
}

pub fn launch_agent_plist(binary: &str, config_path: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>com.focus.notify</string>
  <key>ProgramArguments</key>
  <array>
    <string>{binary}</string>
    <string>notify</string>
    <string>run</string>
  </array>
  <key>EnvironmentVariables</key>
  <dict>
    <key>FOCUS_CONFIG</key>
    <string>{config_path}</string>
  </dict>
  <key>StartInterval</key>
  <integer>1800</integer>
</dict>
</plist>
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn selects_due_and_scheduled_notifications() {
        let today = NaiveDate::from_ymd_opt(2026, 4, 25).unwrap();
        let tasks = vec![
            crate::task::parse_task_line(PathBuf::from("a.md"), 1, "- [ ] Late 📅 2026-04-24")
                .unwrap(),
            crate::task::parse_task_line(PathBuf::from("b.md"), 1, "- [ ] Scheduled ⏳ 2026-04-25")
                .unwrap(),
            crate::task::parse_task_line(PathBuf::from("c.md"), 1, "- [x] Done 📅 2026-04-25")
                .unwrap(),
        ];
        let selected = candidates(&tasks, today);
        assert_eq!(selected.len(), 2);
    }
}
