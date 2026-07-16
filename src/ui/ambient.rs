use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::App;

// Intensity ramp for the glow field, dim → bright.
const RAMP: [char; 8] = [' ', '·', ':', '~', '≈', '▒', '▓', '█'];

/// Ambient aurora: three phase-shifted sine ribbons drift across the full
/// strip width, each glowing in a theme color. Fills whatever area it gets —
/// no fixed geometry, no negative space. Driven by `App::frame`.
pub fn render(f: &mut Frame, app: &App, area: Rect) {
    if area.width < 10 || area.height < 3 {
        return;
    }
    let w = area.width as usize;
    let h = area.height as usize;
    let t = app.frame as f32 * 0.12;
    let theme = app.theme;

    // (amplitude, frequency, speed, phase, color)
    let waves: [(f32, f32, f32, f32, Color); 3] = [
        (0.32, 0.09, 1.0, 0.0, theme.accent),
        (0.24, 0.05, -0.55, 2.1, theme.blue),
        (0.18, 0.13, 0.35, 4.2, theme.muted),
    ];
    let sigma = (h as f32 * 0.16).max(0.6);
    let mid = h as f32 / 2.0;

    let lines: Vec<Line> = (0..h)
        .map(|row| {
            let spans: Vec<Span> = (0..w)
                .map(|col| {
                    let x = col as f32;
                    let mut total = 0.0f32;
                    let mut best = 0.0f32;
                    let mut color = theme.muted;
                    for (amp, freq, speed, phase, c) in waves {
                        let wy = mid + amp * h as f32 * (x * freq + t * speed + phase).sin();
                        let dy = row as f32 - wy;
                        let contrib = (-dy * dy / (2.0 * sigma * sigma)).exp();
                        total += contrib;
                        if contrib > best {
                            best = contrib;
                            color = c;
                        }
                    }
                    let level =
                        ((total.min(1.0) * (RAMP.len() - 1) as f32) as usize).min(RAMP.len() - 1);
                    Span::styled(RAMP[level].to_string(), Style::default().fg(color))
                })
                .collect();
            Line::from(spans)
        })
        .collect();
    f.render_widget(Paragraph::new(lines), area);
}
