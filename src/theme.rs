#![allow(dead_code)]
use ratatui::style::{Color, Modifier, Style};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ThemeVariant {
    Mocha,
    Latte,
}

impl ThemeVariant {
    pub fn toggle(self) -> Self {
        match self {
            Self::Mocha => Self::Latte,
            Self::Latte => Self::Mocha,
        }
    }
}

pub struct Palette {
    pub base: Color,
    pub mantle: Color,
    pub crust: Color,
    pub surface0: Color,
    pub surface1: Color,
    pub surface2: Color,
    pub overlay0: Color,
    pub overlay1: Color,
    pub text: Color,
    pub subtext0: Color,
    pub subtext1: Color,
    pub rosewater: Color,
    pub flamingo: Color,
    pub pink: Color,
    pub mauve: Color,
    pub red: Color,
    pub maroon: Color,
    pub peach: Color,
    pub yellow: Color,
    pub green: Color,
    pub teal: Color,
    pub sky: Color,
    pub sapphire: Color,
    pub blue: Color,
    pub lavender: Color,
}

pub const MOCHA: Palette = Palette {
    base: Color::Rgb(30, 30, 46),
    mantle: Color::Rgb(24, 24, 37),
    crust: Color::Rgb(17, 17, 27),
    surface0: Color::Rgb(49, 50, 68),
    surface1: Color::Rgb(69, 71, 90),
    surface2: Color::Rgb(88, 91, 112),
    overlay0: Color::Rgb(108, 112, 134),
    overlay1: Color::Rgb(127, 132, 156),
    text: Color::Rgb(205, 214, 244),
    subtext0: Color::Rgb(166, 173, 200),
    subtext1: Color::Rgb(186, 194, 222),
    rosewater: Color::Rgb(245, 224, 220),
    flamingo: Color::Rgb(242, 205, 205),
    pink: Color::Rgb(245, 194, 231),
    mauve: Color::Rgb(203, 166, 247),
    red: Color::Rgb(243, 139, 168),
    maroon: Color::Rgb(235, 160, 172),
    peach: Color::Rgb(250, 179, 135),
    yellow: Color::Rgb(249, 226, 175),
    green: Color::Rgb(166, 227, 161),
    teal: Color::Rgb(148, 226, 213),
    sky: Color::Rgb(137, 220, 235),
    sapphire: Color::Rgb(116, 199, 236),
    blue: Color::Rgb(137, 180, 250),
    lavender: Color::Rgb(180, 190, 254),
};

pub const LATTE: Palette = Palette {
    base: Color::Rgb(239, 241, 245),
    mantle: Color::Rgb(230, 233, 239),
    crust: Color::Rgb(220, 224, 232),
    surface0: Color::Rgb(204, 208, 218),
    surface1: Color::Rgb(188, 192, 204),
    surface2: Color::Rgb(172, 176, 190),
    overlay0: Color::Rgb(156, 160, 176),
    overlay1: Color::Rgb(140, 143, 161),
    text: Color::Rgb(76, 79, 105),
    subtext0: Color::Rgb(108, 111, 133),
    subtext1: Color::Rgb(92, 95, 119),
    rosewater: Color::Rgb(220, 138, 120),
    flamingo: Color::Rgb(221, 120, 120),
    pink: Color::Rgb(234, 118, 203),
    mauve: Color::Rgb(136, 57, 239),
    red: Color::Rgb(210, 15, 57),
    maroon: Color::Rgb(230, 69, 83),
    peach: Color::Rgb(254, 100, 11),
    yellow: Color::Rgb(223, 142, 29),
    green: Color::Rgb(64, 160, 43),
    teal: Color::Rgb(23, 146, 153),
    sky: Color::Rgb(4, 165, 229),
    sapphire: Color::Rgb(32, 159, 181),
    blue: Color::Rgb(30, 102, 245),
    lavender: Color::Rgb(114, 135, 253),
};

pub fn palette(variant: ThemeVariant) -> &'static Palette {
    match variant {
        ThemeVariant::Mocha => &MOCHA,
        ThemeVariant::Latte => &LATTE,
    }
}

pub fn border_active(p: &Palette) -> Style {
    Style::default().fg(p.lavender)
}

pub fn border_inactive(p: &Palette) -> Style {
    Style::default().fg(p.surface1)
}

pub fn highlight(p: &Palette) -> Style {
    Style::default().fg(p.mauve).add_modifier(Modifier::BOLD)
}

pub fn status_mode(p: &Palette) -> Style {
    Style::default()
        .fg(p.crust)
        .bg(p.mauve)
        .add_modifier(Modifier::BOLD)
}

pub fn status_file(p: &Palette) -> Style {
    Style::default().fg(p.text)
}

pub fn status_dim(p: &Palette) -> Style {
    Style::default().fg(p.overlay0)
}
