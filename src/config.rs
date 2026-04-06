use std::path::PathBuf;

use serde::Deserialize;

use crate::theme::ThemeVariant;

#[derive(Deserialize, Default)]
pub struct RawConfig {
    pub theme: Option<String>,
    pub sidebar_width: Option<u16>,
    pub render_mode: Option<String>,
}

pub struct Config {
    pub theme: ThemeVariant,
    pub sidebar_width: u16,
    pub render_mode: crate::app::RenderMode,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: ThemeVariant::Mocha,
            sidebar_width: 30,
            render_mode: crate::app::RenderMode::Formatted,
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let path = config_path();
        if !path.exists() {
            return Self::default();
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
        }
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
