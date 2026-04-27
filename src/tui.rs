use crate::config::Config;
use crate::planner::{Plan, render_summary};
use crate::task::Task;
use crate::vault;
use anyhow::Result;
use chrono::{Duration as ChronoDuration, NaiveDate};
use crossterm::event::{self, Event, KeyCode};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use std::io::{self, IsTerminal};
use std::time::Duration as StdDuration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Inbox,
    Today,
    Overdue,
    Scheduled,
    Waiting,
    Projects,
}

impl Section {
    fn all() -> [Self; 6] {
        [
            Self::Inbox,
            Self::Today,
            Self::Overdue,
            Self::Scheduled,
            Self::Waiting,
            Self::Projects,
        ]
    }

    fn title(self) -> &'static str {
        match self {
            Self::Inbox => "Inbox",
            Self::Today => "Today",
            Self::Overdue => "Overdue",
            Self::Scheduled => "Scheduled",
            Self::Waiting => "Waiting",
            Self::Projects => "Projects",
        }
    }
}

#[derive(Debug, Clone)]
pub struct TodayState {
    pub section: Section,
    pub selected: usize,
}

impl Default for TodayState {
    fn default() -> Self {
        Self {
            section: Section::Inbox,
            selected: 0,
        }
    }
}

impl TodayState {
    pub fn next_section(&mut self) {
        let sections = Section::all();
        let idx = sections
            .iter()
            .position(|section| *section == self.section)
            .unwrap_or(0);
        self.section = sections[(idx + 1) % sections.len()];
        self.selected = 0;
    }

    pub fn previous_section(&mut self) {
        let sections = Section::all();
        let idx = sections
            .iter()
            .position(|section| *section == self.section)
            .unwrap_or(0);
        self.section = sections[(idx + sections.len() - 1) % sections.len()];
        self.selected = 0;
    }

    pub fn move_down(&mut self, len: usize) {
        if len > 0 {
            self.selected = (self.selected + 1).min(len - 1);
        }
    }

    pub fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }
}

pub fn tasks_for_section(plan: &Plan, section: Section) -> Vec<&Task> {
    match section {
        Section::Inbox => plan.inbox.iter().collect(),
        Section::Today => plan
            .due_today
            .iter()
            .chain(plan.scheduled_today.iter())
            .collect(),
        Section::Overdue => plan.overdue.iter().collect(),
        Section::Scheduled => plan.scheduled_today.iter().collect(),
        Section::Waiting => plan.waiting.iter().collect(),
        Section::Projects => plan.projects.values().flatten().collect(),
    }
}

pub fn run(config: &Config, plan: &Plan, today: NaiveDate) -> Result<()> {
    if !io::stdout().is_terminal() {
        println!("{}", render_summary(plan));
        return Ok(());
    }

    let mut state = TodayState::default();
    let mut plan = plan.clone();
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = loop {
        terminal.draw(|frame| {
            let area = frame.area();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(5),
                    Constraint::Length(2),
                ])
                .split(area);
            let header = Paragraph::new(Line::from("Focus Today"))
                .block(Block::default().borders(Borders::ALL))
                .style(Style::default().fg(Color::Cyan));
            frame.render_widget(header, chunks[0]);

            let body_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Length(24), Constraint::Min(20)])
                .split(chunks[1]);

            let sections = Section::all()
                .into_iter()
                .map(|section| {
                    let style = if section == state.section {
                        Style::default().fg(Color::Cyan)
                    } else {
                        Style::default()
                    };
                    ListItem::new(Line::from(Span::styled(section.title(), style)))
                })
                .collect::<Vec<_>>();
            frame.render_widget(
                List::new(sections).block(Block::default().title("Sections").borders(Borders::ALL)),
                body_chunks[0],
            );

            let tasks = tasks_for_section(&plan, state.section);
            let items = if tasks.is_empty() {
                vec![ListItem::new("No tasks")]
            } else {
                tasks
                    .iter()
                    .map(|task| {
                        let id = task.id.as_deref().unwrap_or("no-id");
                        ListItem::new(format!("{} [{}]", task.text, id))
                    })
                    .collect()
            };
            let mut list_state = ListState::default();
            if !tasks.is_empty() {
                list_state.select(Some(state.selected.min(tasks.len() - 1)));
            }
            frame.render_stateful_widget(
                List::new(items)
                    .block(
                        Block::default()
                            .title(state.section.title())
                            .borders(Borders::ALL),
                    )
                    .highlight_style(Style::default().fg(Color::Yellow)),
                body_chunks[1],
                &mut list_state,
            );

            let footer = Paragraph::new("tab/shift-tab: section  up/down: task  d: done  f: defer tomorrow  s: schedule today  r: refresh  q: quit")
                .block(Block::default().borders(Borders::ALL));
            frame.render_widget(footer, chunks[2]);
        })?;

        if event::poll(StdDuration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break Ok(()),
                    KeyCode::Tab => state.next_section(),
                    KeyCode::BackTab => state.previous_section(),
                    KeyCode::Down => {
                        let len = tasks_for_section(&plan, state.section).len();
                        state.move_down(len);
                    }
                    KeyCode::Up => state.move_up(),
                    KeyCode::Char('r') => {
                        let tasks = vault::scan_tasks(config)?;
                        plan = crate::planner::build_plan(&tasks, today);
                    }
                    KeyCode::Char('d') | KeyCode::Char('f') | KeyCode::Char('s') => {
                        let selected_id = tasks_for_section(&plan, state.section)
                            .get(state.selected)
                            .and_then(|task| task.id.clone());
                        if let Some(id) = selected_id {
                            match key.code {
                                KeyCode::Char('d') => {
                                    vault::complete_task_by_id(config, &id, today)?;
                                }
                                KeyCode::Char('f') => {
                                    vault::defer_task_by_id(
                                        config,
                                        &id,
                                        today + ChronoDuration::days(1),
                                    )?;
                                }
                                KeyCode::Char('s') => {
                                    vault::schedule_task_by_id(config, &id, today)?;
                                }
                                _ => {}
                            }
                            let tasks = vault::scan_tasks(config)?;
                            plan = crate::planner::build_plan(&tasks, today);
                            let len = tasks_for_section(&plan, state.section).len();
                            if len == 0 {
                                state.selected = 0;
                            } else {
                                state.selected = state.selected.min(len - 1);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    };

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn state_navigation_moves_sections_and_selection() {
        let mut state = TodayState::default();
        assert_eq!(state.section, Section::Inbox);
        state.next_section();
        assert_eq!(state.section, Section::Today);
        state.move_down(3);
        state.move_down(3);
        assert_eq!(state.selected, 2);
        state.next_section();
        assert_eq!(state.selected, 0);
        state.previous_section();
        assert_eq!(state.section, Section::Today);
    }

    #[test]
    fn section_tasks_include_project_tasks() {
        let today = NaiveDate::from_ymd_opt(2026, 4, 25).unwrap();
        let tasks = vec![
            crate::task::parse_task_line(
                PathBuf::from("a.md"),
                1,
                "- [ ] Build app #project/mpa 🆔 abc",
            )
            .unwrap(),
        ];
        let plan = crate::planner::build_plan(&tasks, today);
        assert_eq!(tasks_for_section(&plan, Section::Projects).len(), 1);
    }
}
