use chrono::{Duration, Utc};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use std::io::Write;

use crate::app::App;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Focus,
    Break,
}

impl Kind {
    fn as_str(self) -> &'static str {
        match self {
            Kind::Focus => "focus",
            Kind::Break => "break",
        }
    }
}

pub struct ActiveSession {
    pub db_id: i64,
    pub kind: Kind,
    pub todo_title: Option<String>,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub duration: chrono::Duration,
    pub paused_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl ActiveSession {
    pub fn remaining(&self, now: chrono::DateTime<chrono::Utc>) -> chrono::Duration {
        let effective_now = self.paused_at.unwrap_or(now);
        self.duration - (effective_now - self.started_at)
    }

    pub fn resume(&mut self, now: chrono::DateTime<chrono::Utc>) {
        if let Some(p) = self.paused_at.take() {
            self.started_at += now - p;
        }
    }
}

#[derive(Default)]
pub struct PomodoroState {
    pub active: Option<ActiveSession>,
    pub today_count: u32,
    pub suggest_break: bool,
}

impl PomodoroState {
    pub fn load(&mut self, conn: &rusqlite::Connection, today: chrono::NaiveDate) {
        self.today_count = crate::db::pomo_count_today(conn, today).unwrap_or(0);
    }
}

fn start_session(app: &mut App, kind: Kind, minutes: u64, todo_id: Option<i64>, todo_title: Option<String>) {
    let db_id = match crate::db::pomo_start(&app.conn, todo_id, kind.as_str()) {
        Ok(id) => id,
        Err(_) => return,
    };
    app.pomodoro.active = Some(ActiveSession {
        db_id,
        kind,
        todo_title,
        started_at: Utc::now(),
        duration: Duration::minutes(minutes as i64),
        paused_at: None,
    });
    app.pomodoro.suggest_break = false;
}

/// Starts a Focus session, optionally linked to a todo. Callable from todos.rs.
pub fn start(app: &mut App, todo_id: Option<i64>, todo_title: Option<String>) {
    let minutes = app.config.pomodoro.focus_min;
    start_session(app, Kind::Focus, minutes, todo_id, todo_title);
}

pub fn handle_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('s') => {
            if app.pomodoro.active.is_none() {
                if app.pomodoro.suggest_break {
                    let minutes = app.config.pomodoro.break_min;
                    start_session(app, Kind::Break, minutes, None, None);
                } else {
                    start(app, None, None);
                }
            }
        }
        KeyCode::Char(' ') => {
            if let Some(active) = app.pomodoro.active.as_mut() {
                if active.paused_at.is_some() {
                    active.resume(Utc::now());
                } else {
                    active.paused_at = Some(Utc::now());
                }
            }
        }
        KeyCode::Char('x') => {
            if let Some(active) = app.pomodoro.active.take() {
                let _ = crate::db::pomo_finish(&app.conn, active.db_id, false);
            }
        }
        _ => {}
    }
}

/// Called from App::tick — checks for completion, rings the bell, and updates state.
pub fn on_tick(app: &mut App) {
    let now = Utc::now();
    let Some(active) = &app.pomodoro.active else { return };
    if active.remaining(now) > Duration::zero() {
        return;
    }
    let db_id = active.db_id;
    let kind = active.kind;
    let _ = crate::db::pomo_finish(&app.conn, db_id, true);
    print!("\x07");
    let _ = std::io::stdout().flush();
    match kind {
        Kind::Focus => {
            app.pomodoro.today_count = crate::db::pomo_count_today(&app.conn, app.today).unwrap_or(0);
            app.pomodoro.suggest_break = true;
            app.status = Some("focus done — s starts a 5m break".into());
        }
        Kind::Break => {
            app.status = Some("break over — s starts focus".into());
        }
    }
    app.pomodoro.active = None;
}

// 3x5 block-character digit font for 0-9 and ':' (index 10).
const DIGITS: [[&str; 5]; 11] = [
    ["███", "█ █", "█ █", "█ █", "███"], // 0
    ["  █", "  █", "  █", "  █", "  █"], // 1
    ["███", "  █", "███", "█  ", "███"], // 2
    ["███", "  █", "███", "  █", "███"], // 3
    ["█ █", "█ █", "███", "  █", "  █"], // 4
    ["███", "█  ", "███", "  █", "███"], // 5
    ["███", "█  ", "███", "█ █", "███"], // 6
    ["███", "  █", "  █", "  █", "  █"], // 7
    ["███", "█ █", "███", "█ █", "███"], // 8
    ["███", "█ █", "███", "  █", "███"], // 9
    [" ", "█", " ", "█", " "],           // :
];

fn digit_index(c: char) -> usize {
    match c {
        '0'..='9' => c as usize - '0' as usize,
        _ => 10, // ':'
    }
}

fn big_clock_lines(text: &str, color: Color) -> Vec<Line<'static>> {
    let glyphs: Vec<usize> = text.chars().map(digit_index).collect();
    (0..5)
        .map(|row| {
            let s = glyphs
                .iter()
                .map(|&g| DIGITS[g][row])
                .collect::<Vec<_>>()
                .join(" ");
            Line::styled(s, Style::default().fg(color))
        })
        .collect()
}

fn mmss(d: Duration) -> (i64, i64) {
    let secs = d.num_seconds().max(0);
    (secs / 60, secs % 60)
}

fn active_color(app: &App) -> Color {
    let Some(active) = &app.pomodoro.active else { return app.theme.muted };
    if active.paused_at.is_some() {
        app.theme.yellow
    } else {
        match active.kind {
            Kind::Focus => app.theme.green,
            Kind::Break => app.theme.peach,
        }
    }
}

pub fn render_panel(f: &mut Frame, app: &mut App, area: Rect, focused: bool) {
    let mut line = if let Some(active) = &app.pomodoro.active {
        let (mm, ss) = mmss(active.remaining(Utc::now()));
        let kind_str = active.kind.as_str();
        match &active.todo_title {
            Some(title) => format!("▶ {mm:02}:{ss:02} {kind_str} · {title}"),
            None => format!("▶ {mm:02}:{ss:02} {kind_str}"),
        }
    } else {
        format!("{} done today · s to start", app.pomodoro.today_count)
    };
    if app.pomodoro.suggest_break && app.pomodoro.active.is_none() {
        line.push_str("  ·  break time?");
    }
    let block = app.theme.panel_block("POMODORO", focused);
    f.render_widget(
        Paragraph::new(Line::from(line)).style(Style::default().fg(app.theme.text)).block(block),
        area,
    );
}

pub fn render_zoomed(f: &mut Frame, app: &mut App) {
    let area = f.area();
    f.render_widget(
        ratatui::widgets::Block::default().style(Style::default().bg(app.theme.bg)),
        area,
    );
    let rows = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(area);

    let block = app.theme.panel_block("POMODORO", true);
    let inner = block.inner(rows[0]);
    f.render_widget(block, rows[0]);

    let now = Utc::now();
    let remaining = match &app.pomodoro.active {
        Some(active) => active.remaining(now),
        None => Duration::minutes(app.config.pomodoro.focus_min as i64),
    };
    let (mm, ss) = mmss(remaining);
    let clock_text = format!("{mm:02}:{ss:02}");
    let color = active_color(app);
    let mut lines = big_clock_lines(&clock_text, color);

    lines.push(Line::default());
    let title_line = match app.pomodoro.active.as_ref().and_then(|a| a.todo_title.as_ref()) {
        Some(title) => Line::styled(title.clone(), Style::default().fg(app.theme.text)),
        None => Line::default(),
    };
    lines.push(title_line);

    let filled = app.pomodoro.today_count.min(4);
    let dots: String = (0..4)
        .map(|i| if i < filled { "● " } else { "○ " })
        .collect();
    lines.push(Line::styled(dots.trim_end().to_string(), Style::default().fg(app.theme.peach)));

    let content = Layout::vertical([Constraint::Fill(1), Constraint::Length(lines.len() as u16), Constraint::Fill(1)])
        .split(inner);
    f.render_widget(
        Paragraph::new(lines).alignment(Alignment::Center),
        content[1],
    );

    let hint = app.status.clone().unwrap_or_else(|| {
        " s start · space pause · x abandon · esc home ".into()
    });
    f.render_widget(Paragraph::new(hint).style(app.theme.hint()), rows[1]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, TimeZone, Utc};

    #[test]
    fn remaining_counts_down_and_freezes_while_paused() {
        let t0 = Utc.with_ymd_and_hms(2026, 7, 15, 10, 0, 0).unwrap();
        let mut s = ActiveSession {
            db_id: 1, kind: Kind::Focus, todo_title: None,
            started_at: t0, duration: Duration::minutes(25), paused_at: None,
        };
        assert_eq!(s.remaining(t0 + Duration::minutes(10)), Duration::minutes(15));
        s.paused_at = Some(t0 + Duration::minutes(10));
        // clock advances, remaining doesn't
        assert_eq!(s.remaining(t0 + Duration::minutes(20)), Duration::minutes(15));
    }

    #[test]
    fn resume_shifts_started_at_so_no_time_is_lost() {
        let t0 = Utc.with_ymd_and_hms(2026, 7, 15, 10, 0, 0).unwrap();
        let mut s = ActiveSession {
            db_id: 1, kind: Kind::Focus, todo_title: None,
            started_at: t0, duration: Duration::minutes(25),
            paused_at: Some(t0 + Duration::minutes(10)),
        };
        s.resume(t0 + Duration::minutes(30)); // paused 20 min
        assert_eq!(s.remaining(t0 + Duration::minutes(30)), Duration::minutes(15));
        assert_eq!(s.paused_at, None);
    }
}
