use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use syntect::easy::HighlightLines;
use syntect::highlighting::{FontStyle, Style as SyntectStyle, Theme, ThemeSet};
use syntect::parsing::{SyntaxReference, SyntaxSet};

const THEME_NAME: &str = "base16-ocean.dark";
const MD_EXTENSION: &str = "md";

pub struct Highlighter {
    syntax_set: SyntaxSet,
    theme: Theme,
    syntax_idx: usize,
}

impl Highlighter {
    pub fn new() -> Self {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let theme_set = ThemeSet::load_defaults();
        let theme = theme_set
            .themes
            .get(THEME_NAME)
            .cloned()
            .unwrap_or_else(|| theme_set.themes.values().next().unwrap().clone());
        let syntax_idx = syntax_set
            .find_syntax_by_extension(MD_EXTENSION)
            .map(|s| {
                syntax_set
                    .syntaxes()
                    .iter()
                    .position(|x| x.name == s.name)
                    .unwrap_or(0)
            })
            .unwrap_or(0);
        Self {
            syntax_set,
            theme,
            syntax_idx,
        }
    }

    fn syntax(&self) -> &SyntaxReference {
        &self.syntax_set.syntaxes()[self.syntax_idx]
    }

    pub fn highlight_markdown(&self, source: &str) -> Vec<Line<'static>> {
        let mut hl = HighlightLines::new(self.syntax(), &self.theme);
        let mut lines = Vec::new();

        for line_str in source.lines() {
            let line_with_nl = format!("{}\n", line_str);
            let ranges = hl
                .highlight_line(&line_with_nl, &self.syntax_set)
                .unwrap_or_default();

            let spans: Vec<Span<'static>> = ranges
                .iter()
                .map(|(style, text)| {
                    Span::styled(
                        text.trim_end_matches('\n').to_string(),
                        syntect_to_ratatui(style),
                    )
                })
                .collect();

            lines.push(Line::from(spans));
        }

        lines
    }
}

fn syntect_to_ratatui(style: &SyntectStyle) -> Style {
    let fg = Color::Rgb(style.foreground.r, style.foreground.g, style.foreground.b);
    let mut modifier = Modifier::empty();
    if style.font_style.contains(FontStyle::BOLD) {
        modifier |= Modifier::BOLD;
    }
    if style.font_style.contains(FontStyle::ITALIC) {
        modifier |= Modifier::ITALIC;
    }
    if style.font_style.contains(FontStyle::UNDERLINE) {
        modifier |= Modifier::UNDERLINED;
    }
    Style::default().fg(fg).add_modifier(modifier)
}
