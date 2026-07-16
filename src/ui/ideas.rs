use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{App, InputMode};
use crate::db;
use crate::models::Idea;
use crate::theme::Theme;

#[derive(Default)]
pub struct IdeasState {
    pub items: Vec<Idea>,
    pub selected: usize,
    pub input: Option<String>,     // Some = instant-capture title buffer
    pub body_edit: Option<String>, // Some = body-edit buffer for selected idea
}

impl IdeasState {
    pub fn load(&mut self, conn: &rusqlite::Connection) {
        self.items = db::ideas_list(conn).unwrap_or_default();
        if self.selected >= self.items.len() && !self.items.is_empty() {
            self.selected = self.items.len() - 1;
        }
    }
}

fn badge(t: &Theme, status: &str) -> (&'static str, ratatui::style::Color) {
    match status {
        "spark" => ("✦", t.yellow),
        "brewing" => ("◌", t.peach),
        "active" => ("▶", t.blue),
        "shipped" => ("✔", t.green),
        "dropped" => ("✕", t.muted),
        _ => ("?", t.muted),
    }
}

pub fn handle_key(app: &mut App, key: KeyEvent) {
    // instant title capture
    if let Some(buf) = app.ideas.input.as_mut() {
        match key.code {
            KeyCode::Enter => {
                let title = buf.trim().to_string();
                if !title.is_empty() {
                    let _ = db::idea_add(&app.conn, &title);
                }
                app.ideas.input = None;
                app.mode = InputMode::Normal;
                app.ideas.load(&app.conn);
            }
            KeyCode::Esc => {
                app.ideas.input = None;
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

    // body edit
    if let Some(buf) = app.ideas.body_edit.as_mut() {
        match key.code {
            KeyCode::Enter => {
                if let Some(idea) = app.ideas.items.get(app.ideas.selected) {
                    let id = idea.id;
                    let body = buf.clone();
                    let _ = db::idea_set_body(&app.conn, id, &body);
                }
                app.ideas.body_edit = None;
                app.mode = InputMode::Normal;
                app.ideas.load(&app.conn);
            }
            KeyCode::Esc => {
                app.ideas.body_edit = None;
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

    let n = app.ideas.items.len();
    match key.code {
        KeyCode::Up | KeyCode::Char('k') if n > 0 => {
            app.ideas.selected = app.ideas.selected.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') if n > 0 => {
            app.ideas.selected = (app.ideas.selected + 1).min(n - 1);
        }
        KeyCode::Char('a') => {
            app.ideas.input = Some(String::new());
            app.mode = InputMode::Editing;
        }
        KeyCode::Enter if n > 0 => {
            let current = app.ideas.items[app.ideas.selected].body.clone();
            app.ideas.body_edit = Some(current);
            app.mode = InputMode::Editing;
        }
        KeyCode::Char('s') if n > 0 => {
            let id = app.ideas.items[app.ideas.selected].id;
            let _ = db::idea_cycle_status(&app.conn, id);
            app.ideas.load(&app.conn);
        }
        KeyCode::Char('d') if n > 0 => {
            let _ = db::idea_delete(&app.conn, app.ideas.items[app.ideas.selected].id);
            app.ideas.load(&app.conn);
        }
        _ => {}
    }
}

fn idea_lines(app: &App) -> Vec<ListItem<'static>> {
    let t = app.theme;
    app.ideas
        .items
        .iter()
        .enumerate()
        .map(|(i, idea)| {
            let (mark, mark_color) = badge(&t, &idea.status);
            let mut title_style = Style::default().fg(t.text);
            if i == app.ideas.selected {
                title_style = title_style.add_modifier(Modifier::BOLD).fg(t.accent);
            }
            let spans = vec![
                Span::styled(format!(" {mark} "), Style::default().fg(mark_color)),
                Span::styled(idea.title.clone(), title_style),
            ];
            ListItem::new(Line::from(spans))
        })
        .collect()
}

pub fn render_panel(f: &mut Frame, app: &mut App, area: Rect, focused: bool) {
    let title = format!("IDEAS ({})", app.ideas.items.len());
    let block = app.theme.panel_block(&title, focused);
    let t = app.theme;
    let lines: Vec<Line> = app
        .ideas
        .items
        .iter()
        .take(5)
        .map(|idea| {
            let (mark, mark_color) = badge(&t, &idea.status);
            let date = idea.created_at.get(0..10).unwrap_or(&idea.created_at);
            Line::from(vec![
                Span::styled(format!(" {mark} "), Style::default().fg(mark_color)),
                Span::styled(idea.title.clone(), Style::default().fg(t.text)),
                Span::styled(format!("  {date}"), Style::default().fg(t.muted)),
            ])
        })
        .collect();
    f.render_widget(Paragraph::new(lines).block(block), area);
}

pub fn render_zoomed(f: &mut Frame, app: &mut App) {
    let area = f.area();
    let rows = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(area);
    let cols =
        Layout::horizontal([Constraint::Percentage(60), Constraint::Percentage(40)]).split(rows[0]);

    let title = format!("IDEAS ({})", app.ideas.items.len());
    let block = app.theme.panel_block(&title, true);
    f.render_widget(List::new(idea_lines(app)).block(block), cols[0]);

    let body_text = app
        .ideas
        .items
        .get(app.ideas.selected)
        .map(|idea| idea.body.clone())
        .unwrap_or_default();
    let detail_block = app.theme.panel_block("DETAIL", false);
    f.render_widget(
        Paragraph::new(body_text)
            .style(Style::default().fg(app.theme.text))
            .wrap(Wrap { trim: false })
            .block(detail_block),
        cols[1],
    );

    let hint = app.status.clone().unwrap_or_else(|| {
        if let Some(buf) = &app.ideas.input {
            format!(" new idea: {buf}▏  (enter save · esc cancel)")
        } else if let Some(buf) = &app.ideas.body_edit {
            format!(" body: {buf}▏  (enter save · esc cancel)")
        } else {
            " a capture · enter edit body · s cycle status · d delete · esc home ".into()
        }
    });
    f.render_widget(Paragraph::new(hint).style(app.theme.hint()), rows[1]);
}
