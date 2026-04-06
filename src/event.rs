use std::io;
use std::path::Path;
use std::time::Duration;

use crossterm::event::{
    self, Event, KeyCode, KeyEvent, KeyEventKind, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::layout::{Position, Rect};

use crate::app::{App, ContextMenu, Focus, RenderMode};
use crate::config::Config;
use crate::markdown::highlight::Highlighter;
use crate::ui;
use crate::watcher;
use crate::Tui;

const POLL_INTERVAL_MS: u64 = 100;
const PAGE_SCROLL_LINES: u16 = 10;
const MOUSE_SCROLL_LINES: u16 = 3;
const MIN_SIDEBAR_WIDTH: u16 = 10;
const MIN_CONTENT_WIDTH: u16 = 20;
const CONTEXT_MENU_ITEMS: usize = 2;

pub fn run(terminal: &mut Tui, path: &Path, config: &Config) -> io::Result<()> {
    let files = watcher::discover_md_files(path, &config.exclude_dirs);
    let mut app = App::new(path.to_path_buf(), files);
    app.theme_variant = config.theme;
    app.sidebar_width = config.sidebar_width;
    app.render_mode = config.render_mode;
    app.exclude_dirs = config.exclude_dirs.clone();
    app.show_help = config.first_run;
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
                    let files = watcher::discover_md_files(path, &app.exclude_dirs);
                    app.refresh_file_list(files);
                }
            }
        }

        if app.should_quit {
            return Ok(());
        }

        if !event::poll(Duration::from_millis(POLL_INTERVAL_MS))? {
            continue;
        }

        match event::read()? {
            Event::Key(KeyEvent {
                code,
                kind: KeyEventKind::Press,
                ..
            }) => {
                if app.show_excludes {
                    handle_exclude_input(&mut app, path, code);
                } else if app.searching {
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
    if app.context_menu.is_some() {
        handle_context_menu_key(app, code);
        return;
    }

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
                app.scroll_down(PAGE_SCROLL_LINES);
            }
        }
        KeyCode::Char('u') => {
            if app.focus == Focus::Content {
                app.scroll_up(PAGE_SCROLL_LINES);
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
                app.clear_selection();
            }
        }
        KeyCode::BackTab => {
            if !app.tabs.is_empty() {
                app.active_tab = if app.active_tab == 0 {
                    app.tabs.len() - 1
                } else {
                    app.active_tab - 1
                };
                app.clear_selection();
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
        KeyCode::Char('e') => {
            app.show_excludes = true;
            app.exclude_input.clear();
            app.exclude_selected = 0;
        }
        _ => {}
    }
}

fn handle_context_menu_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc => {
            app.context_menu = None;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if let Some(m) = &mut app.context_menu {
                m.selected = m.selected.saturating_sub(1);
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if let Some(m) = &mut app.context_menu {
                m.selected = (m.selected + 1).min(CONTEXT_MENU_ITEMS - 1);
            }
        }
        KeyCode::Enter => {
            if let Some(menu) = app.context_menu.take() {
                match menu.selected {
                    0 => {
                        let _ = open::that(&menu.url);
                    }
                    1 => {
                        copy_to_clipboard(&menu.url);
                    }
                    _ => {}
                }
            }
        }
        _ => {
            app.context_menu = None;
        }
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

fn handle_exclude_input(app: &mut App, root: &Path, code: KeyCode) {
    match code {
        KeyCode::Esc => {
            app.show_excludes = false;
            app.exclude_input.clear();
        }
        KeyCode::Enter => {
            let input = app.exclude_input.trim().to_string();
            if !input.is_empty() && !app.exclude_dirs.contains(&input) {
                app.exclude_dirs.push(input);
                crate::config::save_exclude_dirs(&app.exclude_dirs);
                let files = watcher::discover_md_files(root, &app.exclude_dirs);
                app.refresh_file_list(files);
            }
            app.exclude_input.clear();
        }
        KeyCode::Backspace => {
            app.exclude_input.pop();
        }
        KeyCode::Char(c) => {
            app.exclude_input.push(c);
        }
        KeyCode::Delete => {
            if !app.exclude_dirs.is_empty() {
                let idx = app.exclude_selected.min(app.exclude_dirs.len() - 1);
                app.exclude_dirs.remove(idx);
                if app.exclude_selected > 0 && app.exclude_selected >= app.exclude_dirs.len() {
                    app.exclude_selected = app.exclude_dirs.len().saturating_sub(1);
                }
                crate::config::save_exclude_dirs(&app.exclude_dirs);
                let files = watcher::discover_md_files(root, &app.exclude_dirs);
                app.refresh_file_list(files);
            }
        }
        KeyCode::Up => {
            app.exclude_selected = app.exclude_selected.saturating_sub(1);
        }
        KeyCode::Down => {
            if !app.exclude_dirs.is_empty() {
                app.exclude_selected = (app.exclude_selected + 1).min(app.exclude_dirs.len() - 1);
            }
        }
        _ => {}
    }
}

fn handle_mouse(terminal: &mut Tui, app: &mut App, mouse: MouseEvent) -> io::Result<()> {
    let area = terminal.get_frame().area();
    let (sidebar_area, content_area, _) = ui::compute_layout(area, app.sidebar_width);
    let border_col = sidebar_area.x + sidebar_area.width;
    let mouse_pos = Position::new(mouse.column, mouse.row);

    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            if app.context_menu.is_some() {
                app.context_menu = None;
                return Ok(());
            }

            let col = mouse.column;
            let row = mouse.row;

            app.clear_selection();

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
                        app.clear_selection();
                    }
                } else if is_content_body(row, col, content_area) {
                    if let Some(tab) = app.tabs.get(app.active_tab) {
                        let (line, c) =
                            screen_to_content(row, col, content_area, tab.scroll_offset);
                        app.selection_start = Some((line, c));
                        app.selection_end = Some((line, c));
                        app.selecting = true;
                    }
                }
            }
        }
        MouseEventKind::Up(MouseButton::Left) => {
            if app.resizing_sidebar {
                app.resizing_sidebar = false;
            } else if app.selecting {
                app.selecting = false;
                if let Some(text) = app.selected_text() {
                    copy_to_clipboard(&text);
                }
            }
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            if app.resizing_sidebar {
                let new_width = mouse
                    .column
                    .max(MIN_SIDEBAR_WIDTH)
                    .min(area.width.saturating_sub(MIN_CONTENT_WIDTH));
                app.sidebar_width = new_width;
            } else if app.selecting {
                if let Some(tab) = app.tabs.get(app.active_tab) {
                    let row = mouse.row;
                    let col = mouse.column;
                    if row > content_area.y && col > content_area.x {
                        let (line, c) =
                            screen_to_content(row, col, content_area, tab.scroll_offset);
                        app.selection_end = Some((line, c));
                    }
                }
            }
        }
        MouseEventKind::Down(MouseButton::Right) => {
            if content_area.contains(mouse_pos)
                && is_content_body(mouse.row, mouse.column, content_area)
            {
                if let Some(tab) = app.tabs.get(app.active_tab) {
                    let (line, col) =
                        screen_to_content(mouse.row, mouse.column, content_area, tab.scroll_offset);
                    if let Some(link) = app.find_link_at(line, col) {
                        let url = link.url.clone();
                        app.context_menu = Some(ContextMenu {
                            x: mouse.column,
                            y: mouse.row,
                            url,
                            selected: 0,
                        });
                    }
                }
            }
        }
        MouseEventKind::ScrollUp => {
            if content_area.contains(mouse_pos) {
                app.scroll_up(MOUSE_SCROLL_LINES);
            } else if sidebar_area.contains(mouse_pos) {
                app.sidebar_up();
            }
        }
        MouseEventKind::ScrollDown => {
            if content_area.contains(mouse_pos) {
                app.scroll_down(MOUSE_SCROLL_LINES);
            } else if sidebar_area.contains(mouse_pos) {
                app.sidebar_down();
            }
        }
        _ => {}
    }

    Ok(())
}

fn screen_to_content(row: u16, col: u16, content_area: Rect, scroll_offset: u16) -> (u16, u16) {
    let line = (row - content_area.y - 1) + scroll_offset;
    let c = col - content_area.x - 1;
    (line, c)
}

fn is_content_body(row: u16, col: u16, area: Rect) -> bool {
    row > area.y && row < area.y + area.height - 1 && col > area.x && col < area.x + area.width - 1
}

fn copy_to_clipboard(text: &str) {
    if let Ok(mut clipboard) = arboard::Clipboard::new() {
        let _ = clipboard.set_text(text);
    }
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
