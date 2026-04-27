mod cli;
mod config;
mod notifications;
mod planner;
mod projects;
mod task;
mod tui;
mod vault;
mod waiting;

use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    cli::Args::parse().run()
}
