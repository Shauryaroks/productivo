use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::App;

const LUM: &[u8] = b".,-~:;=!*#$@";

/// Classic spinning-donut torus projection, sized to whatever area it gets.
/// Angles advance with `App::frame` (one step per event-loop tick), so it
/// spins on its own and never blocks input.
pub fn render(f: &mut Frame, app: &App, area: Rect) {
    if area.width < 16 || area.height < 5 {
        return;
    }
    let w = area.width as usize;
    let h = area.height as usize;
    let a = app.frame as f32 * 0.08;
    let b = app.frame as f32 * 0.04;

    let mut chars = vec![' '; w * h];
    let mut lum = vec![0usize; w * h];
    let mut zbuf = vec![0f32; w * h];

    let (r1, r2, k2) = (1.0f32, 2.0f32, 5.0f32);
    // Fit the projection to the smaller screen dimension (terminal cells are
    // ~2:1, so a row is worth two columns).
    let k1 = f32::min(w as f32 * 0.375, h as f32 * 1.5);

    let (sin_a, cos_a) = a.sin_cos();
    let (sin_b, cos_b) = b.sin_cos();
    let mut theta = 0.0f32;
    while theta < std::f32::consts::TAU {
        let (sin_t, cos_t) = theta.sin_cos();
        let mut phi = 0.0f32;
        while phi < std::f32::consts::TAU {
            let (sin_p, cos_p) = phi.sin_cos();
            let circle_x = r2 + r1 * cos_t;
            let circle_y = r1 * sin_t;
            let x = circle_x * (cos_b * cos_p + sin_a * sin_b * sin_p) - circle_y * cos_a * sin_b;
            let y = circle_x * (sin_b * cos_p - sin_a * cos_b * sin_p) + circle_y * cos_a * cos_b;
            let z = k2 + cos_a * circle_x * sin_p + circle_y * sin_a;
            let ooz = 1.0 / z;
            let xp = (w as f32 / 2.0 + k1 * ooz * x) as isize;
            let yp = (h as f32 / 2.0 - k1 * 0.5 * ooz * y) as isize;
            let l = cos_p * cos_t * sin_b - cos_a * cos_t * sin_p - sin_a * sin_t
                + cos_b * (cos_a * sin_t - cos_t * sin_a * sin_p);
            if l > 0.0 && xp >= 0 && (xp as usize) < w && yp >= 0 && (yp as usize) < h {
                let idx = yp as usize * w + xp as usize;
                if ooz > zbuf[idx] {
                    zbuf[idx] = ooz;
                    let li = ((l * 8.0) as usize).min(LUM.len() - 1);
                    chars[idx] = LUM[li] as char;
                    lum[idx] = li;
                }
            }
            phi += 0.02;
        }
        theta += 0.07;
    }

    let t = app.theme;
    let lines: Vec<Line> = (0..h)
        .map(|row| {
            let spans: Vec<Span> = (0..w)
                .map(|col| {
                    let i = row * w + col;
                    let color = if lum[i] >= 8 {
                        t.accent
                    } else if lum[i] >= 4 {
                        t.blue
                    } else {
                        t.muted
                    };
                    Span::styled(chars[i].to_string(), Style::default().fg(color))
                })
                .collect();
            Line::from(spans)
        })
        .collect();
    f.render_widget(Paragraph::new(lines), area);
}
