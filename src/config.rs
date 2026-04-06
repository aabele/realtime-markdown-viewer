use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::theme::ThemeVariant;

#[derive(Deserialize, Serialize, Default)]
pub struct RawConfig {
    pub theme: Option<String>,
    pub sidebar_width: Option<u16>,
    pub render_mode: Option<String>,
    pub exclude_dirs: Option<Vec<String>>,
}

pub struct Config {
    pub theme: ThemeVariant,
    pub sidebar_width: u16,
    pub render_mode: crate::app::RenderMode,
    pub exclude_dirs: Vec<String>,
    pub first_run: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: ThemeVariant::Mocha,
            sidebar_width: 30,
            render_mode: crate::app::RenderMode::Formatted,
            exclude_dirs: vec![
                "node_modules".to_string(),
                ".git".to_string(),
                "target".to_string(),
            ],
            first_run: true,
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let path = config_path();
        if !path.exists() {
            let config = Self::default();
            config.save();
            return config;
        }

        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return Self::default(),
        };

        let raw: RawConfig = match toml::from_str(&content) {
            Ok(r) => r,
            Err(_) => return Self::default(),
        };

        Self {
            theme: match raw.theme.as_deref() {
                Some("latte") => ThemeVariant::Latte,
                _ => ThemeVariant::Mocha,
            },
            sidebar_width: raw.sidebar_width.unwrap_or(30).clamp(10, 80),
            render_mode: match raw.render_mode.as_deref() {
                Some("syntax") => crate::app::RenderMode::SyntaxHighlight,
                _ => crate::app::RenderMode::Formatted,
            },
            exclude_dirs: raw.exclude_dirs.unwrap_or_default(),
            first_run: false,
        }
    }

    fn to_raw(&self) -> RawConfig {
        RawConfig {
            theme: Some(match self.theme {
                ThemeVariant::Mocha => "mocha".to_string(),
                ThemeVariant::Latte => "latte".to_string(),
            }),
            sidebar_width: Some(self.sidebar_width),
            render_mode: Some(match self.render_mode {
                crate::app::RenderMode::Formatted => "formatted".to_string(),
                crate::app::RenderMode::SyntaxHighlight => "syntax".to_string(),
            }),
            exclude_dirs: if self.exclude_dirs.is_empty() {
                None
            } else {
                Some(self.exclude_dirs.clone())
            },
        }
    }

    fn save(&self) {
        let path = config_path();
        if let Ok(content) = toml::to_string_pretty(&self.to_raw()) {
            let _ = std::fs::write(&path, content);
        }
    }
}

pub fn save_exclude_dirs(exclude_dirs: &[String]) {
    let path = config_path();
    let mut raw: RawConfig = if path.exists() {
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|c| toml::from_str(&c).ok())
            .unwrap_or_default()
    } else {
        RawConfig::default()
    };

    raw.exclude_dirs = if exclude_dirs.is_empty() {
        None
    } else {
        Some(exclude_dirs.to_vec())
    };

    if let Ok(content) = toml::to_string_pretty(&raw) {
        let _ = std::fs::write(&path, content);
    }
}

fn config_path() -> PathBuf {
    dirs_or_home().join(".rtmrc")
}

fn dirs_or_home() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}
