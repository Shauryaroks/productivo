use chrono::{Datelike, Duration, NaiveDate};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph};
use ratatui::Frame;

use crate::app::{App, InputMode};
use crate::db;
use crate::models::{Event, Todo};

pub struct EventForm {
    pub fields: [String; 3], // title, time HH:MM (optional), category
    pub focus: usize,
}

pub struct CalendarState {
    pub cursor: NaiveDate,
    pub events: Vec<Event>,
    pub due: Vec<Todo>,
    pub form: Option<EventForm>,
}

impl Default for CalendarState {
    fn default() -> Self {
        Self {
            cursor: chrono::Local::now().date_naive(),
            events: Vec::new(),
            due: Vec::new(),
            form: None,
        }
    }
}

fn month_start(d: NaiveDate) -> NaiveDate {
    d.with_day(1).unwrap()
}
fn month_end(d: NaiveDate) -> NaiveDate {
    let next = if d.month() == 12 {
        NaiveDate::from_ymd_opt(d.year() + 1, 1, 1).unwrap()
    } else {
        NaiveDate::from_ymd_opt(d.year(), d.month() + 1, 1).unwrap()
    };
    next.pred_opt().unwrap()
}

pub fn category_color(t: &crate::theme::Theme, category: &str) -> Color {
    match category {
        "work" => t.blue,
        "personal" => t.green,
        "health" => t.peach,
        "deadline" => t.red,
        _ => t.accent,
    }
}

impl CalendarState {
    pub fn load(&mut self, conn: &rusqlite::Connection) {
        let start = month_start(self.cursor);
        let end = month_end(self.cursor) + Duration::days(7);
        self.events = db::events_between(conn, start, end).unwrap_or_default();
        self.due = db::todos_due_between(conn, start, end).unwrap_or_default();
    }
    fn events_on(&self, d: NaiveDate) -> Vec<&Event> {
        self.events.iter().filter(|e| e.date == d).collect()
    }
    fn due_on(&self, d: NaiveDate) -> Vec<&Todo> {
        self.due.iter().filter(|t| t.due_date == Some(d)).collect()
    }
}

pub fn handle_key(app: &mut App, key: KeyEvent) {
    if app.calendar.form.is_some() {
        form_key(app, key);
        return;
    }
    let c = app.calendar.cursor;
    let mut moved = None;
    match key.code {
        KeyCode::Left | KeyCode::Char('h') => moved = Some(c - Duration::days(1)),
        KeyCode::Right | KeyCode::Char('l') => moved = Some(c + Duration::days(1)),
        KeyCode::Up | KeyCode::Char('k') => moved = Some(c - Duration::days(7)),
        KeyCode::Down | KeyCode::Char('j') => moved = Some(c + Duration::days(7)),
        KeyCode::Char('[') => moved = Some(month_start(c).pred_opt().unwrap()),
        KeyCode::Char(']') => moved = Some(month_end(c) + Duration::days(1)),
        KeyCode::Char('t') => moved = Some(app.today),
        KeyCode::Char('a') => {
            app.calendar.form = Some(EventForm {
                fields: Default::default(),
                focus: 0,
            });
            app.mode = InputMode::Editing;
        }
        KeyCode::Char('d') => {
            if let Some(e) = app.calendar.events_on(c).first() {
                let _ = db::event_delete(&app.conn, e.id);
                app.calendar.load(&app.conn);
            }
        }
        _ => {}
    }
    if let Some(d) = moved {
        let month_changed = d.month() != c.month() || d.year() != c.year();
        app.calendar.cursor = d;
        if month_changed {
            app.calendar.load(&app.conn);
        }
    }
}

fn form_key(app: &mut App, key: KeyEvent) {
    let form = app.calendar.form.as_mut().unwrap();
    match key.code {
        KeyCode::Esc => {
            app.calendar.form = None;
            app.mode = InputMode::Normal;
        }
        KeyCode::Tab | KeyCode::Down => form.focus = (form.focus + 1) % 3,
        KeyCode::BackTab | KeyCode::Up => form.focus = (form.focus + 2) % 3,
        KeyCode::Backspace => {
            form.fields[form.focus].pop();
        }
        KeyCode::Enter => {
            let title = form.fields[0].trim().to_string();
            if title.is_empty() {
                app.status = Some("title is required".into());
                return;
            }
            let time = form.fields[1].trim();
            if !time.is_empty() && chrono::NaiveTime::parse_from_str(time, "%H:%M").is_err() {
                app.status = Some("time must be HH:MM".into());
                return;
            }
            let cat = match form.fields[2].trim() {
                "" => "general",
                s => s,
            };
            let _ = db::event_add(
                &app.conn,
                &title,
                app.calendar.cursor,
                if time.is_empty() { None } else { Some(time) },
                cat,
                "themed",
            );
            app.calendar.form = None;
            app.mode = InputMode::Normal;
            app.calendar.load(&app.conn);
        }
        KeyCode::Char(c) => form.fields[form.focus].push(c),
        _ => {}
    }
}

fn month_grid_lines(app: &App, compact: bool, cell_w: usize) -> Vec<Line<'static>> {
    let t = app.theme;
    let cur = app.calendar.cursor;
    let start = month_start(cur);
    let header: String = ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"]
        .iter()
        .map(|d| format!("{d:^cell_w$}"))
        .collect();
    let mut lines = vec![Line::from(Span::styled(
        header,
        Style::default().fg(t.muted).add_modifier(Modifier::BOLD),
    ))];
    let lead = start.weekday().num_days_from_monday() as i64;
    let mut day = start - Duration::days(lead);
    let end = month_end(cur);
    while day <= end {
        let mut num_spans: Vec<Span> = Vec::new();
        let mut dot_spans: Vec<Span> = Vec::new();
        for _ in 0..7 {
            let in_month = day.month() == cur.month();
            let mut style = Style::default().fg(if in_month { t.text } else { t.muted });
            if day == app.today {
                style = style.fg(t.accent).add_modifier(Modifier::BOLD);
            }
            if day == cur {
                style = style.add_modifier(Modifier::REVERSED);
            }
            num_spans.push(Span::styled(
                format!("{:^cell_w$}", format!("{:>2}", day.day())),
                style,
            ));

            let evs = app.calendar.events_on(day);
            let due_n = app.calendar.due_on(day).len();
            let mut dot_line: Vec<Span> = Vec::new();
            let mut count = 0usize;
            for e in evs.iter().take(3) {
                dot_line.push(Span::styled(
                    "•",
                    Style::default().fg(category_color(&t, &e.category)),
                ));
                count += 1;
            }
            if due_n > 0 && count < 3 {
                dot_line.push(Span::styled("▪", Style::default().fg(t.yellow)));
                count += 1;
            }
            // Center the dots within the cell.
            let lead_pad = cell_w.saturating_sub(count) / 2;
            let mut cell: Vec<Span> = vec![Span::raw(" ".repeat(lead_pad))];
            cell.extend(dot_line);
            cell.push(Span::raw(
                " ".repeat(cell_w.saturating_sub(lead_pad + count)),
            ));
            dot_spans.extend(cell);
            day += Duration::days(1);
        }
        lines.push(Line::from(num_spans));
        if !compact {
            lines.push(Line::from(dot_spans));
        }
    }
    lines
}

fn agenda_lines(app: &App, day: NaiveDate, header: &str) -> Vec<Line<'static>> {
    let t = app.theme;
    let mut out = vec![Line::from(Span::styled(
        header.to_string(),
        Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
    ))];
    for e in app.calendar.events_on(day) {
        let time = e.time.clone().map(|s| format!("{s} ")).unwrap_or_default();
        out.push(Line::from(vec![
            Span::styled(" • ", Style::default().fg(category_color(&t, &e.category))),
            Span::styled(format!("{time}{}", e.title), Style::default().fg(t.text)),
        ]));
    }
    for td in app.calendar.due_on(day) {
        out.push(Line::from(vec![
            Span::styled(" ▪ due: ", Style::default().fg(t.yellow)),
            Span::styled(td.title.clone(), Style::default().fg(t.text)),
        ]));
    }
    if out.len() == 1 {
        out.push(Line::from(Span::styled(
            "   —",
            Style::default().fg(t.muted),
        )));
    }
    out
}

pub fn render_panel(f: &mut Frame, app: &mut App, area: Rect, focused: bool) {
    let cur = app.calendar.cursor;
    let title = format!(
        "{} {}",
        ["JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC"]
            [cur.month0() as usize],
        cur.year()
    );
    let block = app.theme.panel_block(&title, focused);
    let inner = block.inner(area);
    f.render_widget(block, area);
    // Show event-dot rows whenever the slot is tall enough (13 lines covers a
    // 6-week month with dots); center the grid either way.
    let compact = inner.height < 13;
    let cell_w = if inner.width >= 42 { 5 } else { 4 };
    render_centered_grid(f, app, inner, compact, cell_w);
}

/// Render the month grid centered (both axes) inside `inner`.
fn render_centered_grid(f: &mut Frame, app: &mut App, inner: Rect, compact: bool, cell_w: usize) {
    let lines = month_grid_lines(app, compact, cell_w);
    let grid_w = (7 * cell_w) as u16;
    let grid_h = lines.len() as u16;
    let area = Rect {
        x: inner.x + inner.width.saturating_sub(grid_w) / 2,
        y: inner.y + inner.height.saturating_sub(grid_h) / 2,
        width: grid_w.min(inner.width),
        height: grid_h.min(inner.height),
    };
    f.render_widget(Paragraph::new(lines), area);
}

pub fn render_zoomed(f: &mut Frame, app: &mut App) {
    let area = f.area();
    let rows = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(area);
    let cols =
        Layout::horizontal([Constraint::Percentage(55), Constraint::Percentage(45)]).split(rows[0]);

    let cur = app.calendar.cursor;
    let title = format!(
        "CALENDAR — {} {}",
        [
            "January",
            "February",
            "March",
            "April",
            "May",
            "June",
            "July",
            "August",
            "September",
            "October",
            "November",
            "December"
        ][cur.month0() as usize],
        cur.year()
    );
    let block = app.theme.panel_block(&title, true);
    let inner = block.inner(cols[0]);
    f.render_widget(block, cols[0]);
    // Adaptive cell width, grid centered in the pane instead of hugging the corner.
    let cell_w = (inner.width as usize / 7).clamp(4, 7);
    render_centered_grid(f, app, inner, false, cell_w);

    let mut agenda = agenda_lines(app, cur, &format!("{} · {}", cur.weekday(), cur));
    agenda.push(Line::raw(""));
    agenda.push(Line::from(Span::styled(
        "NEXT 7 DAYS",
        Style::default()
            .fg(app.theme.accent)
            .add_modifier(Modifier::BOLD),
    )));
    for i in 1..=7i64 {
        let d = cur + Duration::days(i);
        let ev = app.calendar.events_on(d).len();
        let due = app.calendar.due_on(d).len();
        if ev + due > 0 {
            agenda.push(Line::from(Span::styled(
                format!(" {} {:>2} — {ev} events · {due} due", d.weekday(), d.day()),
                Style::default().fg(app.theme.text),
            )));
        }
    }
    f.render_widget(
        Paragraph::new(agenda).block(app.theme.panel_block("AGENDA", false)),
        cols[1],
    );

    let hint = app.status.clone().unwrap_or_else(|| {
        " ←↓↑→ move · [/] month · t today · a add event · d delete · esc home ".into()
    });
    f.render_widget(Paragraph::new(hint).style(app.theme.hint()), rows[1]);

    if app.calendar.form.is_some() {
        render_event_form(f, app, area);
    }
}

pub fn render_event_form(f: &mut Frame, app: &mut App, screen: Rect) {
    let form = app.calendar.form.as_ref().unwrap();
    let t = app.theme;
    let labels = ["title", "time (HH:MM)", "category"];
    let w = 50.min(screen.width.saturating_sub(4));
    let popup = Rect {
        x: screen.x + (screen.width.saturating_sub(w)) / 2,
        y: screen.y + (screen.height.saturating_sub(7)) / 2,
        width: w,
        height: 7,
    };
    f.render_widget(Clear, popup);
    let block = t
        .panel_block(&format!("NEW EVENT — {}", app.calendar.cursor), true)
        .style(Style::default().bg(t.surface));
    let inner = block.inner(popup);
    f.render_widget(block, popup);
    let mut lines: Vec<Line> = labels
        .iter()
        .enumerate()
        .map(|(i, label)| {
            let focused = i == form.focus;
            let cursor = if focused { "▏" } else { "" };
            Line::from(vec![
                Span::styled(
                    format!(" {label:<14}"),
                    Style::default().fg(if focused { t.accent } else { t.muted }),
                ),
                Span::styled(
                    format!("{}{cursor}", form.fields[i]),
                    Style::default().fg(t.text),
                ),
            ])
        })
        .collect();
    lines.push(Line::from(Span::styled(
        "  categories: work personal health deadline",
        Style::default().fg(t.muted),
    )));
    f.render_widget(Paragraph::new(lines), inner);
}
