use std::path::{Path, PathBuf};

use crate::theme::ThemeVariant;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Focus {
    Sidebar,
    Content,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RenderMode {
    Formatted,
    SyntaxHighlight,
}

pub struct Tab {
    pub path: PathBuf,
    pub scroll_offset: u16,
    pub content: String,
    pub rendered_line_count: u16,
    pub viewport_height: u16,
    pub cached_lines: Vec<ratatui::text::Line<'static>>,
}

pub struct App {
    pub root: PathBuf,
    pub files: Vec<PathBuf>,
    pub sidebar_selected: usize,
    pub tabs: Vec<Tab>,
    pub active_tab: usize,
    pub focus: Focus,
    pub render_mode: RenderMode,
    pub search_query: String,
    pub searching: bool,
    pub should_quit: bool,
    pub show_help: bool,
    pub sidebar_width: u16,
    pub resizing_sidebar: bool,
    pub theme_variant: ThemeVariant,
}

impl App {
    pub fn new(root: PathBuf, files: Vec<PathBuf>) -> Self {
        Self {
            root,
            files,
            sidebar_selected: 0,
            tabs: Vec::new(),
            active_tab: 0,
            focus: Focus::Sidebar,
            render_mode: RenderMode::Formatted,
            search_query: String::new(),
            searching: false,
            should_quit: false,
            show_help: true,
            sidebar_width: 30,
            resizing_sidebar: false,
            theme_variant: ThemeVariant::Mocha,
        }
    }

    pub fn relative_path(&self, path: &Path) -> String {
        path.strip_prefix(&self.root)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string()
    }

    pub fn open_tab(&mut self, path: PathBuf) {
        if let Some(idx) = self.tabs.iter().position(|t| t.path == path) {
            self.active_tab = idx;
            self.focus = Focus::Content;
            return;
        }
        let content =
            std::fs::read_to_string(&path).unwrap_or_else(|e| format!("Error reading file: {}", e));
        self.tabs.push(Tab {
            path,
            scroll_offset: 0,
            content,
            rendered_line_count: 0,
            viewport_height: 0,
            cached_lines: Vec::new(),
        });
        self.active_tab = self.tabs.len() - 1;
        self.focus = Focus::Content;
    }

    pub fn close_tab(&mut self, idx: usize) {
        if idx >= self.tabs.len() {
            return;
        }
        self.tabs.remove(idx);
        if self.tabs.is_empty() {
            self.active_tab = 0;
            self.focus = Focus::Sidebar;
        } else if self.active_tab >= self.tabs.len() {
            self.active_tab = self.tabs.len() - 1;
        }
    }

    pub fn active_tab_mut(&mut self) -> Option<&mut Tab> {
        self.tabs.get_mut(self.active_tab)
    }

    pub fn scroll_down(&mut self, amount: u16) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab) {
            let max_scroll = tab.rendered_line_count.saturating_sub(tab.viewport_height);
            tab.scroll_offset = (tab.scroll_offset + amount).min(max_scroll);
        }
    }

    pub fn scroll_up(&mut self, amount: u16) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab) {
            tab.scroll_offset = tab.scroll_offset.saturating_sub(amount);
        }
    }

    pub fn sidebar_down(&mut self) {
        let filtered = self.filtered_files();
        if !filtered.is_empty() {
            self.sidebar_selected = (self.sidebar_selected + 1).min(filtered.len() - 1);
        }
    }

    pub fn sidebar_up(&mut self) {
        self.sidebar_selected = self.sidebar_selected.saturating_sub(1);
    }

    pub fn filtered_files(&self) -> Vec<&PathBuf> {
        if self.search_query.is_empty() {
            self.files.iter().collect()
        } else {
            let query = self.search_query.to_lowercase();
            self.files
                .iter()
                .filter(|f| self.relative_path(f).to_lowercase().contains(&query))
                .collect()
        }
    }

    pub fn reload_file(&mut self, path: &Path) {
        if let Ok(content) = std::fs::read_to_string(path) {
            for tab in &mut self.tabs {
                if tab.path == path {
                    tab.content = content;
                    tab.cached_lines.clear();
                    return;
                }
            }
        }
    }

    pub fn invalidate_all_caches(&mut self) {
        for tab in &mut self.tabs {
            tab.cached_lines.clear();
        }
    }

    pub fn refresh_file_list(&mut self, files: Vec<PathBuf>) {
        self.files = files;
        self.tabs.retain(|tab| tab.path.exists());
        if self.active_tab >= self.tabs.len() {
            self.active_tab = self.tabs.len().saturating_sub(1);
        }
        if self.sidebar_selected >= self.files.len() {
            self.sidebar_selected = self.files.len().saturating_sub(1);
        }
    }
}
