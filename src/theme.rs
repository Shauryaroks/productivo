use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, BorderType, Borders};

use crate::config::ThemeCfg;

#[derive(Clone, Copy)]
// Deliberately no `bg`: cells are left unpainted (terminal default background),
// so a translucent terminal emulator shows through — the "glass" look. Only
// `surface` paints solid fills (form popups) for legibility over any wallpaper.
pub struct Theme {
    pub surface: Color,
    pub text: Color,
    pub muted: Color,
    pub accent: Color,
    pub green: Color,
    pub red: Color,
    pub yellow: Color,
    pub blue: Color,
    pub peach: Color,
    pub heat: [Color; 4], // habit heatmap intensity ramp, 0 → max
}

impl Default for Theme {
    // Catppuccin Mocha — calm, high-contrast, terminal-native
    fn default() -> Self {
        Self {
            surface: Color::Rgb(49, 50, 68),
            text: Color::Rgb(205, 214, 244),
            muted: Color::Rgb(127, 132, 156),
            accent: Color::Rgb(203, 166, 247),
            green: Color::Rgb(166, 227, 161),
            red: Color::Rgb(243, 139, 168),
            yellow: Color::Rgb(249, 226, 175),
            blue: Color::Rgb(137, 180, 250),
            peach: Color::Rgb(250, 179, 135),
            heat: [
                Color::Rgb(49, 50, 68),
                Color::Rgb(87, 116, 84),
                Color::Rgb(126, 171, 120),
                Color::Rgb(166, 227, 161),
            ],
        }
    }
}

fn hex(s: &str) -> Option<Color> {
    let s = s.strip_prefix('#')?;
    if s.len() != 6 {
        return None;
    }
    if !s.is_ascii() {
        return None;
    }
    let r = u8::from_str_radix(&s[0..2], 16).ok()?;
    let g = u8::from_str_radix(&s[2..4], 16).ok()?;
    let b = u8::from_str_radix(&s[4..6], 16).ok()?;
    Some(Color::Rgb(r, g, b))
}

impl Theme {
    pub fn from_cfg(cfg: &ThemeCfg) -> Self {
        let mut t = Self::default();
        if let Some(c) = cfg.accent.as_deref().and_then(hex) {
            t.accent = c;
        }
        if let Some(c) = cfg.green.as_deref().and_then(hex) {
            t.green = c;
        }
        if let Some(c) = cfg.red.as_deref().and_then(hex) {
            t.red = c;
        }
        if let Some(c) = cfg.yellow.as_deref().and_then(hex) {
            t.yellow = c;
        }
        if let Some(c) = cfg.blue.as_deref().and_then(hex) {
            t.blue = c;
        }
        if let Some(c) = cfg.peach.as_deref().and_then(hex) {
            t.peach = c;
        }
        t
    }

    /// Every panel in the app is drawn with this block. Rounded, titled, focus-aware.
    pub fn panel_block(&self, title: &str, focused: bool) -> Block<'static> {
        let border = if focused { self.accent } else { self.muted };
        let title_style = if focused {
            Style::default()
                .fg(self.accent)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(self.text)
        };
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border))
            .title(ratatui::text::Span::styled(
                format!(" {title} "),
                title_style,
            ))
    }

    pub fn hint(&self) -> Style {
        Style::default().fg(self.muted)
    }
}
