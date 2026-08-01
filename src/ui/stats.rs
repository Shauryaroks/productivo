use chrono::{Datelike, Duration, NaiveDate};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Sparkline};
use ratatui::Frame;

use crate::app::App;
use crate::db;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Range {
    #[default]
    Week,
    Month,
    Year,
}

impl Range {
    fn label(self) -> &'static str {
        match self {
            Range::Week => "week",
            Range::Month => "month",
            Range::Year => "year",
        }
    }

    fn days(self) -> i64 {
        match self {
            Range::Week => 7,
            Range::Month => 30,
            Range::Year => 365,
        }
    }

    fn next(self) -> Range {
        match self {
            Range::Week => Range::Month,
            Range::Month => Range::Year,
            Range::Year => Range::Week,
        }
    }
}

#[derive(Default)]
pub struct StatsState {
    pub range: Range,
}

pub fn handle_key(app: &mut App, key: KeyEvent) {
    if key.code == KeyCode::Char('r') {
        app.stats.range = app.stats.range.next();
    }
}

fn since_for(range: Range, today: NaiveDate) -> NaiveDate {
    today - Duration::days(range.days())
}

fn fmt_hm(min: u32) -> String {
    format!("{}h {:02}m", min / 60, min % 60)
}

const SPARK_CHARS: [char; 5] = ['▁', '▂', '▃', '▅', '▇'];

fn spark_chars(counts: &[u32], max: u32) -> String {
    let max = max.max(1);
    counts
        .iter()
        .map(|&c| {
            let ratio = c as f32 / max as f32;
            let idx = (ratio * (SPARK_CHARS.len() - 1) as f32).round() as usize;
            SPARK_CHARS[idx.min(SPARK_CHARS.len() - 1)]
        })
        .collect()
}

/// Compact home panel: best current-streak habit, mini 7-day habit sparkline, today's focus time.
pub fn render_panel(f: &mut Frame, app: &mut App, area: Rect, focused: bool) {
    let conn = &app.conn;
    let today = app.today;
    let habits = db::habits_list(conn).unwrap_or_default();
    let habit_count = habits.len() as u32;

    let best = habits
        .iter()
        .map(|h| {
            (
                h.name.clone(),
                db::habit_streak(conn, h.id, today).unwrap_or(0),
            )
        })
        .max_by_key(|(_, s)| *s);
    let streak_line = match best {
        Some((name, streak)) if streak > 0 => format!("{name}  ⚡{streak} now"),
        _ => "no active streaks".to_string(),
    };

    let since = today - Duration::days(6);
    let day_counts = db::stat_habit_days(conn, since).unwrap_or_default();
    let counts: Vec<u32> = (0..7)
        .map(|i| {
            let d = since + Duration::days(i);
            day_counts
                .iter()
                .find(|(dt, _)| *dt == d)
                .map(|(_, n)| *n)
                .unwrap_or(0)
        })
        .collect();
    let spark = spark_chars(&counts, habit_count);

    let focus_today = db::stat_focus_minutes(conn, today)
        .unwrap_or_default()
        .into_iter()
        .find(|(d, _)| *d == today)
        .map(|(_, m)| m)
        .unwrap_or(0);
    let focus_line = format!("focus {} today", fmt_hm(focus_today));

    let t = app.theme;

    // Tall panel (bento right-bottom): the productivity pet lives here.
    if area.height >= 18 {
        crate::ui::pet::render(f, app, area, focused);
        return;
    }

    let block = t.panel_block("STATS", focused);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let lines = vec![
        Line::styled(streak_line, Style::default().fg(t.peach)),
        Line::styled(spark, Style::default().fg(t.green)),
        Line::styled(focus_line, Style::default().fg(t.text)),
    ];
    f.render_widget(Paragraph::new(lines), inner);
}

pub fn render_zoomed(f: &mut Frame, app: &mut App) {
    let area = f.area();
    let rows = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(area);

    let title = format!("STATS — {}", app.stats.range.label());
    let outer = app.theme.panel_block(&title, true);
    let inner = outer.inner(rows[0]);
    f.render_widget(outer, rows[0]);

    let grid_rows =
        Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)]).split(inner);
    let top = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(grid_rows[0]);

    crate::ui::pet::render(f, app, top[0], false);
    render_velocity(f, app, top[1]);
    render_week_review(f, app, grid_rows[1]);

    let hint = app
        .status
        .clone()
        .unwrap_or_else(|| " r range · esc home ".into());
    f.render_widget(Paragraph::new(hint).style(app.theme.hint()), rows[1]);
}

fn render_velocity(f: &mut Frame, app: &mut App, area: Rect) {
    let t = app.theme;
    let block = t.panel_block("TODO VELOCITY", false);
    let inner = block.inner(area);
    f.render_widget(block, area);
    if inner.width == 0 || inner.height < 5 {
        return;
    }

    let today = app.today;
    let since = since_for(app.stats.range, today);
    let data = db::stat_todo_velocity(&app.conn, since).unwrap_or_default();
    let ndays = ((today - since).num_days().max(0) + 1) as usize;
    let mut created = vec![0u64; ndays];
    let mut completed = vec![0u64; ndays];
    for (d, c, done) in &data {
        let idx = (*d - since).num_days();
        if idx >= 0 && (idx as usize) < ndays {
            created[idx as usize] = *c as u64;
            completed[idx as usize] = *done as u64;
        }
    }
    let total_created: u64 = created.iter().sum();
    let total_done: u64 = completed.iter().sum();

    let rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .split(inner);

    // Sparkline only shows the *first* `width` samples, so keep just the most recent window.
    let width = rows[1].width as usize;
    let recent = |v: &[u64]| -> Vec<u64> {
        if v.len() > width && width > 0 {
            v[v.len() - width..].to_vec()
        } else {
            v.to_vec()
        }
    };
    let created_recent = recent(&created);
    let completed_recent = recent(&completed);

    f.render_widget(
        Paragraph::new(Line::styled("created", Style::default().fg(t.blue))),
        rows[0],
    );
    f.render_widget(
        Sparkline::default()
            .data(&created_recent)
            .style(Style::default().fg(t.blue)),
        rows[1],
    );
    f.render_widget(
        Paragraph::new(Line::styled("completed", Style::default().fg(t.green))),
        rows[2],
    );
    f.render_widget(
        Sparkline::default()
            .data(&completed_recent)
            .style(Style::default().fg(t.green)),
        rows[3],
    );
    f.render_widget(
        Paragraph::new(Line::styled(
            format!("▲ {total_created} created · ✔ {total_done} done"),
            Style::default().fg(t.text),
        )),
        rows[4],
    );
}

fn render_week_review(f: &mut Frame, app: &mut App, area: Rect) {
    let t = app.theme;
    let block = t.panel_block("WEEK REVIEW", false);
    let inner = block.inner(area);
    f.render_widget(block, area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let today = app.today;
    let this_monday = today - Duration::days(today.weekday().num_days_from_monday() as i64);
    let last_monday = this_monday - Duration::days(7);
    let empty = db::WeekStats {
        habit_pct: 0,
        todos_done: 0,
        focus_min: 0,
    };
    let cur = db::stat_week(&app.conn, this_monday).unwrap_or(empty);
    let empty2 = db::WeekStats {
        habit_pct: 0,
        todos_done: 0,
        focus_min: 0,
    };
    let prev = db::stat_week(&app.conn, last_monday).unwrap_or(empty2);

    let habit_delta = cur.habit_pct as i64 - prev.habit_pct as i64;
    let todos_delta = cur.todos_done as i64 - prev.todos_done as i64;
    let focus_delta = cur.focus_min as i64 - prev.focus_min as i64;

    let arrow = |delta: i64| -> (&'static str, Color) {
        if delta >= 0 {
            ("▲", t.green)
        } else {
            ("▼", t.red)
        }
    };
    let (ha, hc) = arrow(habit_delta);
    let (ta, tc) = arrow(todos_delta);
    let (fa, fc) = arrow(focus_delta);

    let lines = vec![
        Line::from(vec![
            Span::styled(format!("{:<15}", "habits"), Style::default().fg(t.text)),
            Span::styled(format!("{}% ", cur.habit_pct), Style::default().fg(t.text)),
            Span::styled(format!("{ha} {:+}", habit_delta), Style::default().fg(hc)),
        ]),
        Line::from(vec![
            Span::styled(
                format!("{:<15}", "todos closed"),
                Style::default().fg(t.text),
            ),
            Span::styled(format!("{} ", cur.todos_done), Style::default().fg(t.text)),
            Span::styled(format!("{ta} {:+}", todos_delta), Style::default().fg(tc)),
        ]),
        Line::from(vec![
            Span::styled(format!("{:<15}", "focus"), Style::default().fg(t.text)),
            Span::styled(
                format!("{} ", fmt_hm(cur.focus_min)),
                Style::default().fg(t.text),
            ),
            Span::styled(
                format!(
                    "{fa} {}{}",
                    if focus_delta >= 0 { "+" } else { "-" },
                    fmt_hm(focus_delta.unsigned_abs() as u32)
                ),
                Style::default().fg(fc),
            ),
        ]),
    ];
    f.render_widget(Paragraph::new(lines), inner);
}
