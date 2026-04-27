use crate::config::Config;
use crate::notifications;
use crate::planner;
use crate::projects;
use crate::task::{Priority, build_task_line};
use crate::vault;
use crate::waiting;
use anyhow::{Context, Result, anyhow};
use chrono::{Local, NaiveDate};
use clap::{Args as ClapArgs, Parser, Subcommand};
use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "focus",
    version,
    about = "Obsidian-backed personal productivity CLI"
)]
pub struct Args {
    #[command(subcommand)]
    command: Command,
}

impl Args {
    pub fn run(self) -> Result<()> {
        match self.command {
            Command::Init(cmd) => cmd.run(),
            Command::Capture(cmd) => cmd.run(),
            Command::Plan => {
                let config = Config::load()?;
                let plan = planner::write_daily_plan(&config, today())?;
                println!("{}", planner::render_summary(&plan));
                Ok(())
            }
            Command::Today => {
                let config = Config::load()?;
                let tasks = vault::scan_tasks(&config)?;
                let plan = planner::build_plan(&tasks, today());
                crate::tui::run(&config, &plan, today())
            }
            Command::Ids(cmd) => cmd.run(),
            Command::Projects => {
                let config = Config::load()?;
                let summaries = projects::load_summaries(&config, today())?;
                println!("{}", projects::render_projects(&summaries));
                Ok(())
            }
            Command::Project(cmd) => cmd.run(),
            Command::Waiting => {
                let config = Config::load()?;
                let ledger = waiting::write_ledger(&config, today())?;
                println!("Waiting items: {}", ledger.count());
                for (person, tasks) in ledger.groups {
                    println!("{person}: {}", tasks.len());
                }
                Ok(())
            }
            Command::Done(cmd) => cmd.run(),
            Command::Review => {
                let config = Config::load()?;
                let tasks = vault::scan_tasks(&config)?;
                let today = today();
                let completed_today = tasks
                    .iter()
                    .filter(|task| task.is_done() && task.done == Some(today))
                    .count();
                let due_remaining = tasks
                    .iter()
                    .filter(|task| task.is_open() && task.due.is_some_and(|due| due <= today))
                    .count();
                let waiting = tasks
                    .iter()
                    .filter(|task| task.is_open() && task.is_waiting())
                    .count();
                println!("Completed today: {completed_today}");
                println!("Due or overdue remaining: {due_remaining}");
                println!("Waiting items: {waiting}");
                Ok(())
            }
            Command::Notify(cmd) => cmd.run(),
        }
    }
}

#[derive(Debug, Subcommand)]
enum Command {
    Init(InitCommand),
    Capture(CaptureCommand),
    Plan,
    Today,
    Ids(IdsCommand),
    Projects,
    Project(ProjectCommand),
    Waiting,
    Done(DoneCommand),
    Review,
    Notify(NotifyCommand),
}

#[derive(Debug, ClapArgs)]
struct InitCommand {
    #[arg(long)]
    vault: PathBuf,
    #[arg(long, default_value = "Daily Notes")]
    daily_folder: PathBuf,
    #[arg(long, default_value = "People")]
    people_folder: PathBuf,
    #[arg(long, default_value = "Projects")]
    projects_folder: PathBuf,
    #[arg(long, default_value = "Waiting.md")]
    ledger_path: PathBuf,
    #[arg(long)]
    notifications: bool,
}

impl InitCommand {
    fn run(self) -> Result<()> {
        let config = Config {
            vault_path: self.vault,
            daily_folder: self.daily_folder,
            people_folder: self.people_folder,
            projects_folder: self.projects_folder,
            ledger_path: self.ledger_path,
            notifications_enabled: self.notifications,
            ..Config::default()
        };
        fs::create_dir_all(config.daily_dir())?;
        fs::create_dir_all(config.people_dir())?;
        fs::create_dir_all(config.projects_dir())?;
        if let Some(parent) = config.ledger_file().parent() {
            fs::create_dir_all(parent)?;
        }
        config.save()?;
        println!("Wrote config to {}", Config::path()?.display());
        Ok(())
    }
}

#[derive(Debug, ClapArgs)]
struct CaptureCommand {
    text: String,
    #[arg(long, value_parser = parse_date)]
    due: Option<NaiveDate>,
    #[arg(long, value_parser = parse_date)]
    scheduled: Option<NaiveDate>,
    #[arg(long, value_parser = parse_date)]
    start: Option<NaiveDate>,
    #[arg(long, value_parser = parse_priority)]
    priority: Option<Priority>,
    #[arg(long)]
    person: Option<String>,
    #[arg(long)]
    project: Option<String>,
    #[arg(long)]
    waiting: bool,
}

impl CaptureCommand {
    fn run(self) -> Result<()> {
        let config = Config::load()?;
        let line = build_task_line(
            &self.text,
            self.due,
            self.scheduled,
            self.start,
            self.priority,
            self.person.as_deref(),
            self.project.as_deref(),
            self.waiting,
        );
        let path = vault::daily_note_path(&config, today());
        vault::append_to_section(&path, "inbox", "Inbox", &line)?;
        println!("Captured in {}", path.display());
        Ok(())
    }
}

#[derive(Debug, ClapArgs)]
struct DoneCommand {
    #[arg(long)]
    id: Option<String>,
    #[arg(long)]
    file: Option<PathBuf>,
    #[arg(long)]
    line: Option<usize>,
}

impl DoneCommand {
    fn run(self) -> Result<()> {
        if let Some(id) = self.id {
            let config = Config::load()?;
            let task = vault::complete_task_by_id(&config, &id, today())?;
            println!("Completed {} ({})", id, task.source_ref());
            return Ok(());
        }
        let file = self
            .file
            .ok_or_else(|| anyhow!("pass --id or both --file and --line"))?;
        let line = self
            .line
            .ok_or_else(|| anyhow!("pass --id or both --file and --line"))?;
        vault::complete_task_at(&file, line, today())?;
        println!("Completed {}:{}", file.display(), line);
        Ok(())
    }
}

#[derive(Debug, Subcommand)]
enum IdsSubcommand {
    Backfill,
}

#[derive(Debug, ClapArgs)]
struct IdsCommand {
    #[command(subcommand)]
    command: IdsSubcommand,
}

impl IdsCommand {
    fn run(self) -> Result<()> {
        match self.command {
            IdsSubcommand::Backfill => {
                let config = Config::load()?;
                let changed = vault::backfill_task_ids(&config)?;
                println!("Added IDs to {changed} tasks.");
                Ok(())
            }
        }
    }
}

#[derive(Debug, ClapArgs)]
struct ProjectCommand {
    name: String,
}

impl ProjectCommand {
    fn run(self) -> Result<()> {
        let config = Config::load()?;
        let summaries = projects::load_summaries(&config, today())?;
        let normalized = self.name.to_lowercase().replace('-', " ");
        let summary = summaries
            .get(&normalized)
            .or_else(|| summaries.get(&self.name))
            .ok_or_else(|| anyhow!("no active project named {}", self.name))?;
        println!("{}", projects::render_project(summary));
        Ok(())
    }
}

#[derive(Debug, Subcommand)]
enum NotifySubcommand {
    Run {
        #[arg(long)]
        dry_run: bool,
    },
    Install,
}

#[derive(Debug, ClapArgs)]
struct NotifyCommand {
    #[command(subcommand)]
    command: NotifySubcommand,
}

impl NotifyCommand {
    fn run(self) -> Result<()> {
        match self.command {
            NotifySubcommand::Run { dry_run } => {
                let config = Config::load()?;
                let tasks = vault::scan_tasks(&config)?;
                let candidates = notifications::candidates(&tasks, today());
                for candidate in &candidates {
                    if dry_run {
                        println!("{}: {}", candidate.title, candidate.body);
                    } else {
                        notifications::send(candidate)?;
                    }
                }
                if candidates.is_empty() {
                    println!("No notifications.");
                }
                Ok(())
            }
            NotifySubcommand::Install => {
                let config_path = Config::path()?;
                let binary = env::current_exe().context("could not resolve current executable")?;
                let plist = notifications::launch_agent_plist(
                    &binary.to_string_lossy(),
                    &config_path.to_string_lossy(),
                );
                let dir = dirs::home_dir()
                    .ok_or_else(|| anyhow!("could not resolve home directory"))?
                    .join("Library/LaunchAgents");
                fs::create_dir_all(&dir)?;
                let path = dir.join("com.focus.notify.plist");
                fs::write(&path, plist)?;
                println!("Wrote {}", path.display());
                Ok(())
            }
        }
    }
}

fn parse_date(value: &str) -> Result<NaiveDate, String> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|err| err.to_string())
}

fn parse_priority(value: &str) -> Result<Priority, String> {
    Priority::from_name(value)
        .ok_or_else(|| "expected highest, high, medium, low, or lowest".to_string())
}

fn today() -> NaiveDate {
    Local::now().date_naive()
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use std::fs;
    use std::sync::{Mutex, OnceLock};
    use tempfile::tempdir;

    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
    }

    #[test]
    fn cli_runs_core_fake_vault_workflow() {
        let _guard = env_lock();
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.toml");
        let vault = dir.path().join("vault");
        unsafe {
            env::set_var("FOCUS_CONFIG", &config_path);
        }

        Args::parse_from([
            "focus",
            "init",
            "--vault",
            vault.to_str().unwrap(),
            "--notifications",
        ])
        .run()
        .unwrap();

        Args::parse_from([
            "focus",
            "capture",
            "Ask Alice for the report",
            "--waiting",
            "--person",
            "Alice",
            "--due",
            &today().to_string(),
        ])
        .run()
        .unwrap();

        Args::parse_from(["focus", "plan"]).run().unwrap();
        Args::parse_from(["focus", "waiting"]).run().unwrap();

        let daily = vault::daily_note_path(&Config::load().unwrap(), today());
        let daily_raw = fs::read_to_string(&daily).unwrap();
        assert!(daily_raw.contains("Waiting on Alice: Ask Alice for the report"));
        assert!(daily_raw.contains("<!-- focus:plan:start -->"));

        let ledger = vault.join("Waiting.md");
        let ledger_raw = fs::read_to_string(ledger).unwrap();
        assert!(ledger_raw.contains("### Alice"));

        let task_line = daily_raw
            .lines()
            .position(|line| line.contains("Waiting on Alice: Ask Alice for the report"))
            .unwrap()
            + 1;
        Args::parse_from([
            "focus",
            "done",
            "--id",
            crate::task::parse_task_line(
                daily.clone(),
                task_line,
                daily_raw.lines().nth(task_line - 1).unwrap(),
            )
            .unwrap()
            .id
            .as_deref()
            .unwrap(),
        ])
        .run()
        .unwrap();

        let completed = fs::read_to_string(&daily).unwrap();
        assert!(completed.contains("- [x] Waiting on Alice: Ask Alice for the report"));

        unsafe {
            env::remove_var("FOCUS_CONFIG");
        }
    }

    #[test]
    fn cli_backfills_ids_and_reports_projects() {
        let _guard = env_lock();
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.toml");
        let vault = dir.path().join("vault");
        unsafe {
            env::set_var("FOCUS_CONFIG", &config_path);
        }

        Args::parse_from(["focus", "init", "--vault", vault.to_str().unwrap()])
            .run()
            .unwrap();

        let project_file = vault.join("Projects").join("MPA.md");
        fs::write(
            &project_file,
            "- [ ] Ship milestone #project/mpa 📅 2026-04-24\n",
        )
        .unwrap();

        Args::parse_from(["focus", "ids", "backfill"])
            .run()
            .unwrap();
        let raw = fs::read_to_string(project_file).unwrap();
        assert!(raw.contains("🆔"));

        Args::parse_from(["focus", "projects"]).run().unwrap();
        Args::parse_from(["focus", "project", "mpa"]).run().unwrap();

        unsafe {
            env::remove_var("FOCUS_CONFIG");
        }
    }

    #[test]
    fn legacy_done_by_file_line_still_works() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("note.md");
        fs::write(&path, "- [ ] Legacy task\n").unwrap();
        Args::parse_from([
            "focus",
            "done",
            "--file",
            path.to_str().unwrap(),
            "--line",
            "1",
        ])
        .run()
        .unwrap();
        assert!(
            fs::read_to_string(path)
                .unwrap()
                .contains("- [x] Legacy task")
        );
    }
}
