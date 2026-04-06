use std::io;
use std::path::Path;
use std::time::Duration;

use crossterm::event::{
    self, Event, KeyCode, KeyEvent, KeyEventKind, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::layout::Position;

use crate::app::{App, Focus, RenderMode};
use crate::config::Config;
use crate::markdown::highlight::Highlighter;
use crate::ui;
use crate::watcher;
use crate::Tui;

pub fn run(terminal: &mut Tui, path: &Path, config: &Config) -> io::Result<()> {
    let files = watcher::discover_md_files(path);
    let mut app = App::new(path.to_path_buf(), files);
    app.theme_variant = config.theme;
    app.sidebar_width = config.sidebar_width;
    app.render_mode = config.render_mode;
    let highlighter = Highlighter::new();

    let (watch_rx, _debouncer) = watcher::start_watcher(path)
        .map_err(|e| io::Error::other(format!("Watcher error: {}", e)))?;

    loop {
        terminal.draw(|frame| ui::draw(frame, &mut app, &highlighter))?;

        while let Ok(evt) = watch_rx.try_recv() {
            match evt {
                watcher::WatchEvent::FileChanged(changed_path) => {
                    app.reload_file(&changed_path);
                }
                watcher::WatchEvent::Rescan => {
                    let files = watcher::discover_md_files(path);
                    app.refresh_file_list(files);
                }
            }
        }

        if app.should_quit {
            return Ok(());
        }

        if !event::poll(Duration::from_millis(100))? {
            continue;
        }

        match event::read()? {
            Event::Key(KeyEvent {
                code,
                kind: KeyEventKind::Press,
                ..
            }) => {
                if app.searching {
                    handle_search_input(&mut app, code);
                } else {
                    handle_key(&mut app, code);
                }
            }
            Event::Mouse(mouse) => {
                handle_mouse(terminal, &mut app, mouse)?;
            }
            Event::Resize(_, _) => {}
            _ => {}
        }
    }
}

fn handle_key(app: &mut App, code: KeyCode) {
    if app.show_help {
        app.show_help = false;
        return;
    }

    match code {
        KeyCode::Char('q') | KeyCode::Esc => {
            app.should_quit = true;
        }
        KeyCode::Char('?') => {
            app.show_help = true;
        }
        KeyCode::Char('h') | KeyCode::Left => {
            app.focus = Focus::Sidebar;
        }
        KeyCode::Char('l') | KeyCode::Right => {
            if !app.tabs.is_empty() {
                app.focus = Focus::Content;
            }
        }
        KeyCode::Char('j') | KeyCode::Down => match app.focus {
            Focus::Sidebar => app.sidebar_down(),
            Focus::Content => app.scroll_down(1),
        },
        KeyCode::Char('k') | KeyCode::Up => match app.focus {
            Focus::Sidebar => app.sidebar_up(),
            Focus::Content => app.scroll_up(1),
        },
        KeyCode::Char('d') => {
            if app.focus == Focus::Content {
                app.scroll_down(10);
            }
        }
        KeyCode::Char('u') => {
            if app.focus == Focus::Content {
                app.scroll_up(10);
            }
        }
        KeyCode::Char('G') => {
            if app.focus == Focus::Content {
                app.scroll_down(u16::MAX);
            }
        }
        KeyCode::Char('g') => {
            if app.focus == Focus::Content {
                if let Some(tab) = app.active_tab_mut() {
                    tab.scroll_offset = 0;
                }
            }
        }
        KeyCode::Enter => {
            if app.focus == Focus::Sidebar {
                let filtered = app.filtered_files();
                if let Some(path) = filtered.get(app.sidebar_selected) {
                    let path = (*path).clone();
                    app.open_tab(path);
                }
            }
        }
        KeyCode::Char('x') => {
            if !app.tabs.is_empty() {
                let idx = app.active_tab;
                app.close_tab(idx);
            }
        }
        KeyCode::Tab => {
            if !app.tabs.is_empty() {
                app.active_tab = (app.active_tab + 1) % app.tabs.len();
            }
        }
        KeyCode::BackTab => {
            if !app.tabs.is_empty() {
                app.active_tab = if app.active_tab == 0 {
                    app.tabs.len() - 1
                } else {
                    app.active_tab - 1
                };
            }
        }
        KeyCode::Char('m') => {
            app.render_mode = match app.render_mode {
                RenderMode::Formatted => RenderMode::SyntaxHighlight,
                RenderMode::SyntaxHighlight => RenderMode::Formatted,
            };
            app.invalidate_all_caches();
        }
        KeyCode::Char('t') => {
            app.theme_variant = app.theme_variant.toggle();
            app.invalidate_all_caches();
        }
        KeyCode::Char('/') => {
            app.searching = true;
            app.search_query.clear();
        }
        _ => {}
    }
}

fn handle_search_input(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc => {
            app.searching = false;
            app.search_query.clear();
        }
        KeyCode::Enter => {
            app.searching = false;
        }
        KeyCode::Backspace => {
            app.search_query.pop();
        }
        KeyCode::Char(c) => {
            app.search_query.push(c);
        }
        _ => {}
    }
    app.sidebar_selected = 0;
}

fn handle_mouse(terminal: &mut Tui, app: &mut App, mouse: MouseEvent) -> io::Result<()> {
    let area = terminal.get_frame().area();
    let (sidebar_area, content_area, _) = ui::compute_layout(area, app.sidebar_width);
    let border_col = sidebar_area.x + sidebar_area.width;
    let mouse_pos = Position::new(mouse.column, mouse.row);

    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            let col = mouse.column;
            let row = mouse.row;

            if col == border_col || col == border_col.saturating_sub(1) {
                app.resizing_sidebar = true;
            } else if sidebar_area.contains(mouse_pos) {
                app.focus = Focus::Sidebar;
                let relative_row = (row - sidebar_area.y).saturating_sub(1) as usize;
                let path = app.filtered_files().get(relative_row).map(|p| (*p).clone());
                if let Some(path) = path {
                    app.sidebar_selected = relative_row;
                    app.open_tab(path);
                }
            } else if content_area.contains(mouse_pos) {
                app.focus = Focus::Content;
                if !app.tabs.is_empty() && row == content_area.y {
                    let tab_click = estimate_tab_index(app, col - content_area.x);
                    if tab_click < app.tabs.len() {
                        app.active_tab = tab_click;
                    }
                }
            }
        }
        MouseEventKind::Up(MouseButton::Left) => {
            app.resizing_sidebar = false;
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            if app.resizing_sidebar {
                let new_width = mouse.column.max(10).min(area.width.saturating_sub(20));
                app.sidebar_width = new_width;
            }
        }
        MouseEventKind::ScrollUp => {
            if content_area.contains(mouse_pos) {
                app.scroll_up(3);
            } else if sidebar_area.contains(mouse_pos) {
                app.sidebar_up();
            }
        }
        MouseEventKind::ScrollDown => {
            if content_area.contains(mouse_pos) {
                app.scroll_down(3);
            } else if sidebar_area.contains(mouse_pos) {
                app.sidebar_down();
            }
        }
        _ => {}
    }

    Ok(())
}

fn estimate_tab_index(app: &App, col_offset: u16) -> usize {
    let mut pos: u16 = 0;
    for (i, tab) in app.tabs.iter().enumerate() {
        let name_len = tab
            .path
            .file_name()
            .map(|n| n.to_string_lossy().len())
            .unwrap_or(3) as u16;
        let tab_width = name_len + 3;
        if col_offset < pos + tab_width {
            return i;
        }
        pos += tab_width;
    }
    app.tabs.len().saturating_sub(1)
}
