use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::theme::{self, Palette, ThemeVariant};

struct Colors {
    heading: Color,
    code: Color,
    code_block_bg: Color,
    link: Color,
    blockquote: Color,
    list_marker: Color,
    rule: Color,
}

impl Colors {
    fn from_palette(p: &Palette) -> Self {
        Self {
            heading: p.blue,
            code: p.green,
            code_block_bg: p.surface0,
            link: p.sapphire,
            blockquote: p.yellow,
            list_marker: p.mauve,
            rule: p.surface1,
        }
    }
}

pub fn render_markdown(
    source: &str,
    variant: ThemeVariant,
) -> (Vec<Line<'static>>, Vec<crate::app::LinkInfo>) {
    let p = theme::palette(variant);
    let c = Colors::from_palette(p);
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(source, options);
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut current_spans: Vec<Span<'static>> = Vec::new();
    let mut style_stack: Vec<Style> = vec![Style::default()];
    let mut list_depth: usize = 0;
    let mut ordered_counters: Vec<Option<u64>> = Vec::new();
    let mut in_code_block = false;
    let mut blockquote_depth: usize = 0;
    let mut current_col: u16 = 0;
    let mut current_line_idx: u16 = 0;
    let mut link_start_col: Option<u16> = None;
    let mut current_link_url: Option<String> = None;
    let mut links: Vec<crate::app::LinkInfo> = Vec::new();

    for event in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                let base = Style::default().fg(c.heading).add_modifier(Modifier::BOLD);
                let style = match level {
                    HeadingLevel::H1 => base.add_modifier(Modifier::UNDERLINED),
                    HeadingLevel::H2 => base,
                    _ => Style::default().fg(c.heading),
                };
                style_stack.push(style);
            }
            Event::End(TagEnd::Heading(_)) => {
                flush_line(&mut lines, &mut current_spans);
                current_col = 0;
                current_line_idx += 1;
                lines.push(Line::default());
                current_line_idx += 1;
                style_stack.pop();
            }
            Event::Start(Tag::Paragraph) => {}
            Event::End(TagEnd::Paragraph) => {
                flush_line(&mut lines, &mut current_spans);
                current_col = 0;
                current_line_idx += 1;
                lines.push(Line::default());
                current_line_idx += 1;
            }
            Event::Start(Tag::Emphasis) => {
                let base = current_style(&style_stack);
                style_stack.push(base.add_modifier(Modifier::ITALIC));
            }
            Event::End(TagEnd::Emphasis) => {
                style_stack.pop();
            }
            Event::Start(Tag::Strong) => {
                let base = current_style(&style_stack);
                style_stack.push(base.add_modifier(Modifier::BOLD));
            }
            Event::End(TagEnd::Strong) => {
                style_stack.pop();
            }
            Event::Start(Tag::Strikethrough) => {
                let base = current_style(&style_stack);
                style_stack.push(base.add_modifier(Modifier::CROSSED_OUT));
            }
            Event::End(TagEnd::Strikethrough) => {
                style_stack.pop();
            }
            Event::Start(Tag::CodeBlock(_)) => {
                in_code_block = true;
                flush_line(&mut lines, &mut current_spans);
                current_col = 0;
                current_line_idx += 1;
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code_block = false;
                flush_line(&mut lines, &mut current_spans);
                current_col = 0;
                current_line_idx += 1;
                lines.push(Line::default());
                current_line_idx += 1;
            }
            Event::Start(Tag::BlockQuote(_)) => {
                blockquote_depth += 1;
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                blockquote_depth = blockquote_depth.saturating_sub(1);
                flush_line(&mut lines, &mut current_spans);
                current_col = 0;
                current_line_idx += 1;
            }
            Event::Start(Tag::List(ordered)) => {
                list_depth += 1;
                ordered_counters.push(ordered);
            }
            Event::End(TagEnd::List(_)) => {
                list_depth = list_depth.saturating_sub(1);
                ordered_counters.pop();
                if list_depth == 0 {
                    lines.push(Line::default());
                }
            }
            Event::Start(Tag::Item) => {
                let indent = "  ".repeat(list_depth.saturating_sub(1));
                let marker = match ordered_counters.last().copied() {
                    Some(Some(n)) => {
                        let m = format!("{}{}. ", indent, n);
                        if let Some(counter) = ordered_counters.last_mut() {
                            *counter = Some(n + 1);
                        }
                        m
                    }
                    _ => format!("{}- ", indent),
                };
                if blockquote_depth > 0 {
                    let prefix = "> ".repeat(blockquote_depth);
                    current_spans.push(Span::styled(prefix, Style::default().fg(c.blockquote)));
                }
                current_spans.push(Span::styled(marker, Style::default().fg(c.list_marker)));
            }
            Event::End(TagEnd::Item) => {
                flush_line(&mut lines, &mut current_spans);
                current_col = 0;
                current_line_idx += 1;
            }
            Event::Start(Tag::Link { dest_url, .. }) => {
                style_stack.push(
                    Style::default()
                        .fg(c.link)
                        .add_modifier(Modifier::UNDERLINED),
                );
                link_start_col = Some(current_col);
                current_link_url = Some(dest_url.to_string());
            }
            Event::End(TagEnd::Link) => {
                style_stack.pop();
                if let (Some(start_col), Some(url)) =
                    (link_start_col.take(), current_link_url.take())
                {
                    links.push(crate::app::LinkInfo {
                        line_idx: current_line_idx,
                        col_start: start_col,
                        col_end: current_col,
                        url,
                    });
                }
            }
            Event::Text(text) => {
                if in_code_block {
                    for code_line in text.lines() {
                        current_spans.push(Span::styled(
                            format!("  {}", code_line),
                            Style::default().fg(c.code).bg(c.code_block_bg),
                        ));
                        flush_line(&mut lines, &mut current_spans);
                        current_col = 0;
                        current_line_idx += 1;
                    }
                } else {
                    if blockquote_depth > 0 && current_spans.is_empty() {
                        let prefix = "> ".repeat(blockquote_depth);
                        current_col += prefix.len() as u16;
                        current_spans.push(Span::styled(prefix, Style::default().fg(c.blockquote)));
                    }
                    current_col += text.len() as u16;
                    current_spans.push(Span::styled(text.to_string(), current_style(&style_stack)));
                }
            }
            Event::Code(code) => {
                current_col += (code.len() + 2) as u16;
                current_spans.push(Span::styled(
                    format!("`{}`", code),
                    Style::default().fg(c.code).bg(c.code_block_bg),
                ));
            }
            Event::SoftBreak | Event::HardBreak => {
                flush_line(&mut lines, &mut current_spans);
                current_col = 0;
                current_line_idx += 1;
            }
            Event::Rule => {
                flush_line(&mut lines, &mut current_spans);
                current_col = 0;
                current_line_idx += 1;
                lines.push(Line::from(Span::styled(
                    "-".repeat(40),
                    Style::default().fg(c.rule),
                )));
                current_line_idx += 1;
                lines.push(Line::default());
                current_line_idx += 1;
            }
            Event::TaskListMarker(checked) => {
                let marker_str = if checked { "[x] " } else { "[ ] " };
                current_spans.push(Span::styled(
                    marker_str.to_string(),
                    Style::default().fg(c.list_marker),
                ));
            }
            _ => {}
        }
    }
    flush_line(&mut lines, &mut current_spans);
    (lines, links)
}

fn flush_line(lines: &mut Vec<Line<'static>>, spans: &mut Vec<Span<'static>>) {
    if !spans.is_empty() {
        lines.push(Line::from(std::mem::take(spans)));
    }
}

fn current_style(stack: &[Style]) -> Style {
    stack.last().copied().unwrap_or_default()
}
