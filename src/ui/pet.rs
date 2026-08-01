use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::App;

/// Completed focus sessions needed per level.
const PER_LEVEL: u32 = 5;

fn stage(level: u32) -> &'static str {
    match level {
        0..=2 => "kitten",
        3..=5 => "cat",
        6..=9 => "chonker",
        _ => "legend",
    }
}

/// The productivity pet: eats when a focus session completes (hungry until the
/// first one of the day), levels up every PER_LEVEL sessions all-time. State is
/// fully derived from the pomodoros table — no storage of its own.
pub fn render(f: &mut Frame, app: &mut App, area: Rect, focused: bool) {
    let t = app.theme;
    let total = crate::db::pomodoro_completed_total(&app.conn).unwrap_or(0);
    let level = total / PER_LEVEL + 1;
    let progress = total % PER_LEVEL;
    let fed_today = app.pomodoro.today_count > 0;

    let block = t.panel_block(&format!("PET · lv{level} {}", stage(level)), focused);
    let inner = block.inner(area);
    f.render_widget(block, area);
    if inner.width < 16 || inner.height < 7 {
        return;
    }

    // Blink every ~3s; happy eyes once fed today.
    let blink = app.frame % 30 < 3;
    let eyes = if blink {
        "-.-"
    } else if fed_today {
        "^.^"
    } else {
        "o.o"
    };
    let tail = if (app.frame / 8) % 2 == 0 { "^" } else { "~" };
    // Pace back and forth across the panel (triangle wave over the frame counter).
    let span_w = inner.width.saturating_sub(10) as usize;
    let period = (span_w.max(1)) * 2;
    let ph = (app.frame / 4) % period;
    let x = if ph < span_w { ph } else { period - ph };
    let pad = " ".repeat(x);

    let bar: String = (0..PER_LEVEL)
        .map(|i| if i < progress { '▰' } else { '▱' })
        .collect();
    let lines = vec![
        Line::styled(format!("{pad}   /\\_/\\"), Style::default().fg(t.peach)),
        Line::styled(format!("{pad}  ( {eyes} )"), Style::default().fg(t.peach)),
        Line::styled(format!("{pad}   > {tail} <"), Style::default().fg(t.peach)),
        Line::raw(""),
        Line::from(vec![
            Span::styled(
                format!(" lv{level} "),
                Style::default().fg(t.text).add_modifier(Modifier::BOLD),
            ),
            Span::styled(bar, Style::default().fg(t.green)),
            Span::styled(
                format!(" {progress}/{PER_LEVEL} to lv{}", level + 1),
                Style::default().fg(t.muted),
            ),
        ]),
        Line::styled(
            if fed_today {
                format!(" fed today · {total} sessions all-time")
            } else {
                " hungry — finish a focus session to feed".to_string()
            },
            Style::default().fg(if fed_today { t.green } else { t.yellow }),
        ),
    ];
    // Center the little scene vertically in whatever the slot gives us.
    let h = lines.len() as u16;
    let content = Rect {
        x: inner.x,
        y: inner.y + inner.height.saturating_sub(h) / 2,
        width: inner.width,
        height: h.min(inner.height),
    };
    f.render_widget(Paragraph::new(lines), content);
}
