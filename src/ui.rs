use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Clear, List, ListItem, ListState, Paragraph, Scrollbar, ScrollbarOrientation,
    ScrollbarState, Tabs, Wrap,
};
use ratatui::Frame;

use crate::app::{App, Focus, RenderMode};
use crate::markdown::highlight::Highlighter;
use crate::markdown::render::render_markdown;
use crate::theme::{self, Palette};

pub const STATUS_HEIGHT: u16 = 1;

pub fn compute_layout(area: Rect, sidebar_width: u16) -> (Rect, Rect, Rect) {
    let [main_area, status_area] =
        Layout::vertical([Constraint::Fill(1), Constraint::Length(STATUS_HEIGHT)]).areas(area);
    let [sidebar_area, content_area] =
        Layout::horizontal([Constraint::Length(sidebar_width), Constraint::Fill(1)])
            .areas(main_area);
    (sidebar_area, content_area, status_area)
}

pub fn draw(frame: &mut Frame, app: &mut App, highlighter: &Highlighter) {
    let area = frame.area();
    let p = theme::palette(app.theme_variant);
    let (sidebar_area, content_area, status_area) = compute_layout(area, app.sidebar_width);

    draw_sidebar(frame, app, p, sidebar_area);

    if app.tabs.is_empty() {
        draw_empty_content(frame, p, content_area);
    } else {
        let [tabs_area, file_content_area] =
            Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]).areas(content_area);

        draw_tabs(frame, app, p, tabs_area);
        draw_content(frame, app, p, highlighter, file_content_area);
    }

    draw_status_bar(frame, app, p, status_area);

    if app.show_help {
        draw_help_overlay(frame, p, area);
    }

    if let Some(menu) = &app.context_menu {
        draw_context_menu(frame, p, menu);
    }

    if app.show_excludes {
        draw_excludes_overlay(frame, app, p, area);
    }
}

fn draw_sidebar(frame: &mut Frame, app: &mut App, p: &Palette, area: Rect) {
    let border_style = if app.focus == Focus::Sidebar {
        theme::border_active(p)
    } else {
        theme::border_inactive(p)
    };

    let filtered = app.filtered_files();
    let items: Vec<ListItem> = filtered
        .iter()
        .map(|path| {
            let rel = app.relative_path(path);
            ListItem::new(rel).style(Style::default().fg(p.subtext0))
        })
        .collect();

    let title = if app.searching {
        format!("/{}", app.search_query)
    } else {
        "Files".to_string()
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .style(Style::default().bg(p.mantle))
                .title(Span::styled(title, Style::default().fg(p.text))),
        )
        .highlight_style(theme::highlight(p))
        .highlight_symbol("> ");

    let mut state = ListState::default();
    if !filtered.is_empty() {
        state.select(Some(app.sidebar_selected.min(filtered.len() - 1)));
    }

    frame.render_stateful_widget(list, area, &mut state);
}

fn draw_tabs(frame: &mut Frame, app: &App, p: &Palette, area: Rect) {
    let tab_titles: Vec<String> = app
        .tabs
        .iter()
        .map(|tab| {
            tab.path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "???".to_string())
        })
        .collect();

    let tabs = Tabs::new(tab_titles)
        .style(Style::default().fg(p.overlay0).bg(p.mantle))
        .select(app.active_tab)
        .highlight_style(Style::default().fg(p.mauve).add_modifier(Modifier::BOLD))
        .divider("|");

    frame.render_widget(tabs, area);
}

fn draw_content(
    frame: &mut Frame,
    app: &mut App,
    p: &Palette,
    highlighter: &Highlighter,
    area: Rect,
) {
    let border_style = if app.focus == Focus::Content {
        theme::border_active(p)
    } else {
        theme::border_inactive(p)
    };

    let render_mode = app.render_mode;
    let theme_variant = app.theme_variant;

    let title = match app.tabs.get(app.active_tab) {
        Some(t) => app.relative_path(&t.path),
        None => return,
    };

    let tab = match app.tabs.get_mut(app.active_tab) {
        Some(t) => t,
        None => return,
    };

    if tab.cached_lines.is_empty() && !tab.content.is_empty() {
        match render_mode {
            RenderMode::Formatted => {
                let (rendered, link_info) = render_markdown(&tab.content, theme_variant);
                tab.cached_lines = rendered;
                tab.links = link_info;
            }
            RenderMode::SyntaxHighlight => {
                tab.cached_lines = highlighter.highlight_markdown(&tab.content);
                tab.links.clear();
            }
        };
    }

    let content_height = area.height.saturating_sub(2);
    let total_lines = tab.cached_lines.len() as u16;
    tab.rendered_line_count = total_lines;
    tab.viewport_height = content_height;

    let max_scroll = total_lines.saturating_sub(content_height);
    if tab.scroll_offset > max_scroll {
        tab.scroll_offset = max_scroll;
    }

    let scroll_offset = tab.scroll_offset;

    let paragraph = Paragraph::new(tab.cached_lines.clone())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .style(Style::default().bg(p.base))
                .title(Span::styled(title, Style::default().fg(p.text))),
        )
        .wrap(Wrap { trim: false })
        .scroll((scroll_offset, 0));

    frame.render_widget(paragraph, area);

    if total_lines > content_height {
        let mut scrollbar_state =
            ScrollbarState::new(total_lines as usize).position(scroll_offset as usize);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .thumb_style(Style::default().fg(p.mauve))
            .track_style(Style::default().fg(p.surface0));
        frame.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
    }

    if let Some((sel_start, sel_end)) = app.normalized_selection() {
        let inner_x = area.x + 1;
        let inner_y = area.y + 1;
        let inner_width = area.width.saturating_sub(2);
        let sel_style = Style::default().bg(p.surface2).fg(p.text);

        for screen_row in 0..content_height {
            let line_idx = scroll_offset + screen_row;
            if line_idx < sel_start.0 || line_idx > sel_end.0 {
                continue;
            }
            let (col_start, col_end) =
                selection_col_range(line_idx, sel_start, sel_end, inner_width);

            for col in col_start..col_end.min(inner_width) {
                let cell_x = inner_x + col;
                let cell_y = inner_y + screen_row;
                if cell_x < area.x + area.width - 1 && cell_y < area.y + area.height - 1 {
                    if let Some(cell) = frame
                        .buffer_mut()
                        .cell_mut(ratatui::layout::Position::new(cell_x, cell_y))
                    {
                        cell.set_style(sel_style);
                    }
                }
            }
        }
    }
}

fn draw_empty_content(frame: &mut Frame, p: &Palette, area: Rect) {
    let msg = Paragraph::new(Span::styled(
        "Select a file from the list (Enter or click)",
        Style::default().fg(p.overlay0),
    ))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme::border_inactive(p))
            .style(Style::default().bg(p.base))
            .title("Content"),
    );
    frame.render_widget(msg, area);
}

fn draw_status_bar(frame: &mut Frame, app: &App, p: &Palette, area: Rect) {
    let mode = match app.render_mode {
        RenderMode::Formatted => "FORMATTED",
        RenderMode::SyntaxHighlight => "SYNTAX",
    };

    let file_info = app
        .tabs
        .get(app.active_tab)
        .map(|t| app.relative_path(&t.path))
        .unwrap_or_else(|| "-".to_string());

    let theme_name = match app.theme_variant {
        crate::theme::ThemeVariant::Mocha => "mocha",
        crate::theme::ThemeVariant::Latte => "latte",
    };

    let status = Line::from(vec![
        Span::styled(format!(" {} ", mode), theme::status_mode(p)),
        Span::raw(" "),
        Span::styled(file_info, theme::status_file(p)),
        Span::styled(" | ", Style::default().fg(p.surface2)),
        Span::styled(
            format!("watching: {} ", app.root.display()),
            theme::status_dim(p),
        ),
        Span::styled(format!("[{}] ", theme_name), Style::default().fg(p.peach)),
        Span::styled("? help", Style::default().fg(p.overlay0)),
    ]);

    let bar = Paragraph::new(status).style(Style::default().bg(p.crust));
    frame.render_widget(bar, area);
}

fn draw_help_overlay(frame: &mut Frame, p: &Palette, area: Rect) {
    let help_width: u16 = 52;
    let help_height: u16 = 23;
    let x = area.width.saturating_sub(help_width) / 2;
    let y = area.height.saturating_sub(help_height) / 2;
    let help_area = Rect::new(
        x,
        y,
        help_width.min(area.width),
        help_height.min(area.height),
    );

    let help_lines = vec![
        Line::from(Span::styled(
            " Realtime Markdown Viewer ",
            Style::default().fg(p.mauve).add_modifier(Modifier::BOLD),
        )),
        Line::default(),
        help_line("j / k", "scroll up/down", p),
        help_line("h / l", "sidebar <-> content", p),
        help_line("Enter", "open file in tab", p),
        help_line("Tab / Shift+Tab", "switch tabs", p),
        help_line("x", "close tab", p),
        help_line("m", "formatted / syntax highlight", p),
        help_line("t", "toggle mocha / latte theme", p),
        help_line("/", "search files", p),
        help_line("e", "exclude directories", p),
        help_line("d / u", "scroll 10 lines", p),
        help_line("g / G", "go to top / bottom", p),
        help_line("?", "toggle this help", p),
        help_line("q / Esc", "quit", p),
        Line::default(),
        help_line("drag", "select text (auto-copy)", p),
        help_line("right-click", "link context menu", p),
        Line::default(),
        Line::from(Span::styled(
            " Mouse: click, scroll, drag to resize ",
            Style::default().fg(p.overlay0),
        )),
        Line::default(),
        Line::from(Span::styled(
            " Press any key to close ",
            Style::default().fg(p.peach).add_modifier(Modifier::ITALIC),
        )),
    ];

    frame.render_widget(Clear, help_area);
    let help = Paragraph::new(help_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(p.mauve))
            .style(Style::default().bg(p.mantle))
            .title(Span::styled(" ? Help ", Style::default().fg(p.mauve))),
    );
    frame.render_widget(help, help_area);
}

fn help_line(key: &str, desc: &str, p: &Palette) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("  {:>16}  ", key),
            Style::default().fg(p.sapphire).add_modifier(Modifier::BOLD),
        ),
        Span::styled(desc.to_string(), Style::default().fg(p.text)),
    ])
}

fn selection_col_range(
    line_idx: u16,
    sel_start: (u16, u16),
    sel_end: (u16, u16),
    default_end: u16,
) -> (u16, u16) {
    if sel_start.0 == sel_end.0 {
        (sel_start.1, sel_end.1)
    } else if line_idx == sel_start.0 {
        (sel_start.1, default_end)
    } else if line_idx == sel_end.0 {
        (0, sel_end.1)
    } else {
        (0, default_end)
    }
}

fn draw_excludes_overlay(frame: &mut Frame, app: &App, p: &Palette, area: Rect) {
    let overlay_width: u16 = 50;
    let list_lines = app.exclude_dirs.len().max(1) as u16;
    let overlay_height: u16 = list_lines + 7;
    let x = area.width.saturating_sub(overlay_width) / 2;
    let y = area.height.saturating_sub(overlay_height) / 2;
    let overlay_area = Rect::new(
        x,
        y,
        overlay_width.min(area.width),
        overlay_height.min(area.height),
    );

    frame.render_widget(Clear, overlay_area);

    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(Span::styled(
        " Excluded directories ",
        Style::default().fg(p.mauve).add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::default());

    if app.exclude_dirs.is_empty() {
        lines.push(Line::from(Span::styled(
            "  (none)",
            Style::default().fg(p.overlay0),
        )));
    } else {
        for (i, dir) in app.exclude_dirs.iter().enumerate() {
            let marker = if i == app.exclude_selected {
                "> "
            } else {
                "  "
            };
            let style = if i == app.exclude_selected {
                Style::default().fg(p.peach).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(p.text)
            };
            lines.push(Line::from(Span::styled(
                format!("{}{}", marker, dir),
                style,
            )));
        }
    }

    lines.push(Line::default());
    lines.push(Line::from(vec![
        Span::styled(" Add: ", Style::default().fg(p.subtext0)),
        Span::styled(
            format!("{}_", app.exclude_input),
            Style::default().fg(p.green),
        ),
    ]));
    lines.push(Line::default());
    lines.push(Line::from(Span::styled(
        " Enter=add  Del=remove  Esc=close ",
        Style::default().fg(p.overlay0),
    )));

    let para = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(p.mauve))
            .style(Style::default().bg(p.mantle))
            .title(Span::styled(" Excludes ", Style::default().fg(p.mauve))),
    );
    frame.render_widget(para, overlay_area);
}

fn draw_context_menu(frame: &mut Frame, p: &Palette, menu: &crate::app::ContextMenu) {
    let items = ["Open in browser", "Copy URL"];
    let width: u16 = 20;
    let height: u16 = (items.len() + 2) as u16;
    let menu_area = Rect::new(menu.x, menu.y, width, height);

    frame.render_widget(Clear, menu_area);

    let menu_items: Vec<Line> = items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            if i == menu.selected {
                Line::from(Span::styled(
                    format!(" > {} ", item),
                    Style::default().fg(p.mauve).add_modifier(Modifier::BOLD),
                ))
            } else {
                Line::from(Span::styled(
                    format!("   {} ", item),
                    Style::default().fg(p.text),
                ))
            }
        })
        .collect();

    let para = Paragraph::new(menu_items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(p.mauve))
            .style(Style::default().bg(p.mantle)),
    );
    frame.render_widget(para, menu_area);
}
