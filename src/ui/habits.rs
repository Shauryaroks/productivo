use chrono::NaiveDate;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, ListState, Paragraph};
use ratatui::Frame;

use crate::app::{App, InputMode};
use crate::db;
use crate::models::Habit;

#[derive(Default)]
pub struct HabitsState {
    pub items: Vec<Habit>,
    pub checked: Vec<i64>,
    pub streaks: Vec<u32>,
    pub selected: usize,
    pub day: Option<NaiveDate>, // None = today; Some(d) = viewing yesterday
    pub input: Option<String>,  // Some = add-mode text buffer
}

impl HabitsState {
    pub fn load(&mut self, conn: &rusqlite::Connection, today: NaiveDate) {
        let day = self.day.unwrap_or(today);
        self.items = db::habits_list(conn).unwrap_or_default();
        self.checked = db::habit_checked_on(conn, day).unwrap_or_default();
        self.streaks = self
            .items
            .iter()
            .map(|h| db::habit_streak(conn, h.id, today).unwrap_or(0))
            .collect();
        if self.selected >= self.items.len() && !self.items.is_empty() {
            self.selected = self.items.len() - 1;
        }
    }
}

pub fn handle_key(app: &mut App, key: KeyEvent) {
    let today = app.today;
    // add-mode text entry
    if let Some(buf) = app.habits.input.as_mut() {
        match key.code {
            KeyCode::Enter => {
                let name = buf.trim().to_string();
                if !name.is_empty() {
                    let _ = db::habit_add(&app.conn, &name);
                }
                app.habits.input = None;
                app.mode = InputMode::Normal;
                app.habits.load(&app.conn, today);
            }
            KeyCode::Esc => {
                app.habits.input = None;
                app.mode = InputMode::Normal;
            }
            KeyCode::Backspace => {
                buf.pop();
            }
            KeyCode::Char(c) => buf.push(c),
            _ => {}
        }
        return;
    }

    let n = app.habits.items.len();
    match key.code {
        KeyCode::Up | KeyCode::Char('k') if n > 0 => {
            app.habits.selected = app.habits.selected.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') if n > 0 => {
            app.habits.selected = (app.habits.selected + 1).min(n - 1);
        }
        KeyCode::Char(' ') if n > 0 => {
            let id = app.habits.items[app.habits.selected].id;
            let day = app.habits.day.unwrap_or(today);
            let _ = db::habit_toggle(&app.conn, id, day);
            app.habits.load(&app.conn, today);
        }
        KeyCode::Char('a') => {
            app.habits.input = Some(String::new());
            app.mode = InputMode::Editing;
        }
        KeyCode::Char('d') if n > 0 => {
            let _ = db::habit_archive(&app.conn, app.habits.items[app.habits.selected].id);
            app.habits.load(&app.conn, today);
        }
        KeyCode::Char('K') if n > 0 => {
            let id = app.habits.items[app.habits.selected].id;
            if db::habit_move(&app.conn, id, -1).unwrap_or(false) {
                app.habits.selected = app.habits.selected.saturating_sub(1);
            }
            app.habits.load(&app.conn, today);
        }
        KeyCode::Char('J') if n > 0 => {
            let id = app.habits.items[app.habits.selected].id;
            if db::habit_move(&app.conn, id, 1).unwrap_or(false) {
                app.habits.selected = (app.habits.selected + 1).min(n - 1);
            }
            app.habits.load(&app.conn, today);
        }
        // y toggles viewing yesterday (spec: yesterday editable, nothing older)
        KeyCode::Char('y') => {
            app.habits.day = match app.habits.day {
                None => Some(today.pred_opt().unwrap()),
                Some(_) => None,
            };
            app.habits.load(&app.conn, today);
        }
        _ => {}
    }
}

fn habit_lines(app: &App, show_streak: bool) -> Vec<ListItem<'static>> {
    let t = app.theme;
    app.habits
        .items
        .iter()
        .enumerate()
        .map(|(i, h)| {
            let checked = app.habits.checked.contains(&h.id);
            let mark = if checked { "✔" } else { "○" };
            let mark_style = if checked {
                Style::default().fg(t.green)
            } else {
                Style::default().fg(t.muted)
            };
            let mut name_style = Style::default().fg(t.text);
            if i == app.habits.selected {
                name_style = name_style.add_modifier(Modifier::BOLD).fg(t.accent);
            }
            let mut spans = vec![
                Span::styled(format!(" {mark} "), mark_style),
                Span::styled(h.name.clone(), name_style),
            ];
            if show_streak && app.habits.streaks.get(i).copied().unwrap_or(0) > 0 {
                spans.push(Span::styled(
                    format!("  ⚡{}", app.habits.streaks[i]),
                    Style::default().fg(t.peach),
                ));
            }
            ListItem::new(Line::from(spans))
        })
        .collect()
}

/// Text-entry hint for the bottom bar — shown on both the zoomed screen and
/// Home (panels are directly editable from Home).
pub fn input_hint(app: &App) -> Option<String> {
    app.habits
        .input
        .as_ref()
        .map(|buf| format!(" new habit: {buf}▏  (enter save · esc cancel)"))
}

pub fn render_panel(f: &mut Frame, app: &mut App, area: Rect, focused: bool) {
    let done = app.habits.checked.len();
    let total = app.habits.items.len();
    let title = if app.habits.day.is_some() {
        format!("HABITS {done}/{total} · yday")
    } else {
        format!("HABITS {done}/{total}")
    };
    let block = app.theme.panel_block(&title, focused);
    // Stateful render scrolls the list to keep the selection visible on overflow.
    let mut st = ListState::default();
    st.select(Some(app.habits.selected));
    f.render_stateful_widget(
        List::new(habit_lines(app, false)).block(block),
        area,
        &mut st,
    );
}

pub fn render_zoomed(f: &mut Frame, app: &mut App) {
    let area = f.area();
    let rows = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(area);
    let day_label = match app.habits.day {
        None => "today".to_string(),
        Some(d) => format!("yesterday · {d}"),
    };
    let block = app
        .theme
        .panel_block(&format!("HABITS — {day_label}"), true);
    let mut st = ListState::default();
    st.select(Some(app.habits.selected));
    f.render_stateful_widget(
        List::new(habit_lines(app, true)).block(block),
        rows[0],
        &mut st,
    );

    let hint = app
        .status
        .clone()
        .or_else(|| input_hint(app))
        .unwrap_or_else(|| {
            " space check · a add · d archive · J/K reorder · y yesterday · esc home ".into()
        });
    f.render_widget(Paragraph::new(hint).style(app.theme.hint()), rows[1]);
}
