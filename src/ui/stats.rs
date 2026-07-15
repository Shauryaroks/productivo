use chrono::{Datelike, Duration, NaiveDate};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{BarChart, Paragraph, Sparkline};
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
        .map(|h| (h.name.clone(), db::habit_streak(conn, h.id, today).unwrap_or(0)))
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
            day_counts.iter().find(|(dt, _)| *dt == d).map(|(_, n)| *n).unwrap_or(0)
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
    let lines = vec![
        Line::styled(streak_line, Style::default().fg(t.peach)),
        Line::styled(spark, Style::default().fg(t.green)),
        Line::styled(focus_line, Style::default().fg(t.text)),
    ];
    let block = t.panel_block("STATS", focused);
    f.render_widget(Paragraph::new(lines).block(block), area);
}

pub fn render_zoomed(f: &mut Frame, app: &mut App) {
    let area = f.area();
    f.render_widget(
        ratatui::widgets::Block::default().style(Style::default().bg(app.theme.bg)),
        area,
    );
    let rows = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(area);

    let title = format!("STATS — {}", app.stats.range.label());
    let outer = app.theme.panel_block(&title, true);
    let inner = outer.inner(rows[0]);
    f.render_widget(outer, rows[0]);

    let grid_rows = Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)]).split(inner);
    let top = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).split(grid_rows[0]);
    let bottom = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).split(grid_rows[1]);

    render_heatmap(f, app, top[0]);
    render_velocity(f, app, top[1]);
    render_focus(f, app, bottom[0]);
    render_week_review(f, app, bottom[1]);

    let hint = app.status.clone().unwrap_or_else(|| " r range · esc home ".into());
    f.render_widget(Paragraph::new(hint).style(app.theme.hint()), rows[1]);
}

fn render_heatmap(f: &mut Frame, app: &mut App, area: Rect) {
    let t = app.theme;
    let block = t.panel_block("HABIT HEATMAP", false);
    let inner = block.inner(area);
    f.render_widget(block, area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let today = app.today;
    let habits = db::habits_list(&app.conn).unwrap_or_default();
    let habit_count = habits.len() as u32;
    let since = since_for(app.stats.range, today);
    let day_counts = db::stat_habit_days(&app.conn, since).unwrap_or_default();
    let lookup = |d: NaiveDate| day_counts.iter().find(|(dt, _)| *dt == d).map(|(_, n)| *n).unwrap_or(0);

    let this_monday = today - Duration::days(today.weekday().num_days_from_monday() as i64);
    let earliest_monday = since - Duration::days(since.weekday().num_days_from_monday() as i64);
    let mut total_weeks = ((this_monday - earliest_monday).num_days() / 7 + 1).max(1) as usize;
    if app.stats.range == Range::Year {
        total_weeks = total_weeks.min(52);
    }
    // each cell renders as "■ " — 2 columns wide.
    let max_cols = ((inner.width / 2).max(1)) as usize;
    let weeks_shown = total_weeks.min(max_cols).max(1);
    let start_monday = this_monday - Duration::days(7 * (weeks_shown as i64 - 1));

    let mut lines: Vec<Line> = Vec::with_capacity(7 + habits.len());
    for row in 0..7i64 {
        let mut spans = Vec::with_capacity(weeks_shown);
        for w in 0..weeks_shown {
            let d = start_monday + Duration::days(7 * w as i64 + row);
            let color = if d > today || habit_count == 0 {
                t.heat[0]
            } else {
                let n = lookup(d);
                let ratio = n as f32 / habit_count as f32;
                let idx = if n == 0 {
                    0
                } else if ratio <= 1.0 / 3.0 {
                    1
                } else if ratio <= 2.0 / 3.0 {
                    2
                } else {
                    3
                };
                t.heat[idx]
            };
            spans.push(Span::styled("■ ", Style::default().fg(color)));
        }
        lines.push(Line::from(spans));
    }

    for h in &habits {
        let cur = db::habit_streak(&app.conn, h.id, today).unwrap_or(0);
        let best = db::habit_best_streak(&app.conn, h.id).unwrap_or(0);
        lines.push(Line::from(vec![
            Span::styled(format!("{:<12}", h.name), Style::default().fg(t.text)),
            Span::styled(format!("⚡{cur} now · {best} best"), Style::default().fg(t.peach)),
        ]));
    }

    f.render_widget(Paragraph::new(lines), inner);
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

    f.render_widget(Paragraph::new(Line::styled("created", Style::default().fg(t.blue))), rows[0]);
    f.render_widget(
        Sparkline::default().data(&created_recent).style(Style::default().fg(t.blue)),
        rows[1],
    );
    f.render_widget(Paragraph::new(Line::styled("completed", Style::default().fg(t.green))), rows[2]);
    f.render_widget(
        Sparkline::default().data(&completed_recent).style(Style::default().fg(t.green)),
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

fn render_focus(f: &mut Frame, app: &mut App, area: Rect) {
    let t = app.theme;
    let block = t.panel_block("FOCUS", false);
    let inner = block.inner(area);
    f.render_widget(block, area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let today = app.today;
    let since = since_for(app.stats.range, today);
    let mins = db::stat_focus_minutes(&app.conn, since).unwrap_or_default();
    let n = (((today - since).num_days() + 1).clamp(1, 14)) as i64;
    let start = today - Duration::days(n - 1);
    let bars: Vec<(String, u64)> = (0..n)
        .map(|i| {
            let d = start + Duration::days(i);
            let m = mins.iter().find(|(dt, _)| *dt == d).map(|(_, v)| *v).unwrap_or(0);
            (d.day().to_string(), m as u64)
        })
        .collect();
    let bar_data: Vec<(&str, u64)> = bars.iter().map(|(l, v)| (l.as_str(), *v)).collect();

    let rows = Layout::vertical([Constraint::Length((inner.height * 3 / 5).max(4)), Constraint::Min(0)]).split(inner);

    let chart = BarChart::default()
        .data(&bar_data)
        .bar_width(3)
        .bar_gap(1)
        .bar_style(Style::default().fg(t.peach))
        .value_style(Style::default().fg(t.bg).bg(t.peach))
        .label_style(Style::default().fg(t.muted));
    f.render_widget(chart, rows[0]);

    let projects = db::stat_focus_by_project(&app.conn, since).unwrap_or_default();
    let max_proj = projects.iter().map(|(_, m)| *m).max().unwrap_or(0);
    let bar_budget = (rows[1].width as usize).saturating_sub(24).clamp(4, 20);
    let mut plines: Vec<Line> = Vec::new();
    for (name, m) in projects.iter().take(rows[1].height as usize) {
        let bar_len = if max_proj == 0 { 0 } else { (*m as usize * bar_budget) / max_proj as usize };
        let bar_len = if *m > 0 { bar_len.max(1) } else { 0 };
        let bar = "█".repeat(bar_len);
        plines.push(Line::from(vec![
            Span::styled(format!("#{name} "), Style::default().fg(t.blue)),
            Span::styled(bar, Style::default().fg(t.peach)),
            Span::styled(format!(" {}", fmt_hm(*m)), Style::default().fg(t.text)),
        ]));
    }
    if plines.is_empty() {
        plines.push(Line::styled(" no focus sessions yet", Style::default().fg(t.muted)));
    }
    f.render_widget(Paragraph::new(plines), rows[1]);
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
    let empty = db::WeekStats { habit_pct: 0, todos_done: 0, focus_min: 0 };
    let cur = db::stat_week(&app.conn, this_monday).unwrap_or(empty);
    let empty2 = db::WeekStats { habit_pct: 0, todos_done: 0, focus_min: 0 };
    let prev = db::stat_week(&app.conn, last_monday).unwrap_or(empty2);

    let habit_delta = cur.habit_pct as i64 - prev.habit_pct as i64;
    let todos_delta = cur.todos_done as i64 - prev.todos_done as i64;
    let focus_delta = cur.focus_min as i64 - prev.focus_min as i64;

    let arrow = |delta: i64| -> (&'static str, Color) {
        if delta >= 0 { ("▲", t.green) } else { ("▼", t.red) }
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
            Span::styled(format!("{:<15}", "todos closed"), Style::default().fg(t.text)),
            Span::styled(format!("{} ", cur.todos_done), Style::default().fg(t.text)),
            Span::styled(format!("{ta} {:+}", todos_delta), Style::default().fg(tc)),
        ]),
        Line::from(vec![
            Span::styled(format!("{:<15}", "focus"), Style::default().fg(t.text)),
            Span::styled(format!("{} ", fmt_hm(cur.focus_min)), Style::default().fg(t.text)),
            Span::styled(
                format!("{fa} {}{}", if focus_delta >= 0 { "+" } else { "-" }, fmt_hm(focus_delta.unsigned_abs() as u32)),
                Style::default().fg(fc),
            ),
        ]),
    ];
    f.render_widget(Paragraph::new(lines), inner);
}
