use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Config {
    pub vault_path: PathBuf,
    pub daily_folder: PathBuf,
    pub people_folder: PathBuf,
    #[serde(default = "default_projects_folder")]
    pub projects_folder: PathBuf,
    pub ledger_path: PathBuf,
    pub notifications_enabled: bool,
    pub stale_waiting_days: i64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            vault_path: PathBuf::from("."),
            daily_folder: PathBuf::from("Daily Notes"),
            people_folder: PathBuf::from("People"),
            projects_folder: default_projects_folder(),
            ledger_path: PathBuf::from("Waiting.md"),
            notifications_enabled: false,
            stale_waiting_days: 7,
        }
    }
}

impl Config {
    pub fn path() -> Result<PathBuf> {
        if let Ok(path) = env::var("FOCUS_CONFIG") {
            return Ok(PathBuf::from(path));
        }
        let base = dirs::config_dir().context("could not resolve user config directory")?;
        Ok(base.join("focus").join("config.toml"))
    }

    pub fn load() -> Result<Self> {
        let path = Self::path()?;
        let raw = fs::read_to_string(&path)
            .with_context(|| format!("could not read config at {}", path.display()))?;
        let mut config: Self = toml::from_str(&raw)
            .with_context(|| format!("could not parse config at {}", path.display()))?;
        if config.daily_folder == PathBuf::from("Daily") {
            config.daily_folder = PathBuf::from("Daily Notes");
        }
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("could not create config directory {}", parent.display())
            })?;
        }
        let raw = toml::to_string_pretty(self).context("could not serialize config")?;
        fs::write(&path, raw)
            .with_context(|| format!("could not write config at {}", path.display()))
    }

    pub fn resolve_in_vault(&self, path: &Path) -> PathBuf {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.vault_path.join(path)
        }
    }

    pub fn daily_dir(&self) -> PathBuf {
        self.resolve_in_vault(&self.daily_folder)
    }

    pub fn people_dir(&self) -> PathBuf {
        self.resolve_in_vault(&self.people_folder)
    }

    pub fn projects_dir(&self) -> PathBuf {
        self.resolve_in_vault(&self.projects_folder)
    }

    pub fn ledger_file(&self) -> PathBuf {
        self.resolve_in_vault(&self.ledger_path)
    }
}

fn default_projects_folder() -> PathBuf {
    PathBuf::from("Projects")
}
