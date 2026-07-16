use chrono::NaiveDate;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::app::{App, InputMode};
use crate::db::{self, NewTodo};
use crate::models::Todo;
use crate::recur;

pub struct Row {
    pub todo: Todo,
    pub is_subtask: bool,
    pub sub_counts: Option<(i64, i64)>,
}

pub struct TodoForm {
    pub fields: [String; 7], // title, notes, priority(0-2), due, project, tags, recur
    pub focus: usize,
    pub editing_id: Option<i64>,
    pub parent_id: Option<i64>,
}

pub const FIELD_LABELS: [&str; 7] = [
    "title",
    "notes",
    "priority",
    "due (YYYY-MM-DD)",
    "project",
    "tags",
    "repeat",
];

#[derive(Default)]
pub struct TodosState {
    pub items: Vec<Row>,
    pub selected: usize,
    pub group_by_project: bool,
    pub filter: Option<String>,
    pub filter_editing: bool,
    pub form: Option<TodoForm>,
    pub expanded: Option<i64>,
    /// (completed todo id, spawned next-occurrence id if recurring) — lets `u` undo both.
    pub last_completed: Option<(i64, Option<i64>)>,
}

impl TodosState {
    pub fn load(&mut self, conn: &rusqlite::Connection) {
        let mut tops = db::todos_open(conn).unwrap_or_default();
        if let Some(f) = self.filter.as_deref() {
            let f = f.to_lowercase();
            tops.retain(|t| {
                t.title.to_lowercase().contains(&f)
                    || t.project
                        .as_deref()
                        .unwrap_or("")
                        .to_lowercase()
                        .contains(&f)
                    || t.tags.to_lowercase().contains(&f)
            });
        }
        if self.group_by_project {
            tops.sort_by_key(|t| t.project.clone().unwrap_or_default());
        }
        self.items = Vec::new();
        for t in tops {
            let id = t.id;
            self.items.push(Row {
                sub_counts: db::open_subtask_count(conn, id).ok().filter(|c| c.1 > 0),
                todo: t,
                is_subtask: false,
            });
            if self.expanded == Some(id) {
                for s in db::subtasks_of(conn, id).unwrap_or_default() {
                    self.items.push(Row {
                        todo: s,
                        is_subtask: true,
                        sub_counts: None,
                    });
                }
            }
        }
        if self.selected >= self.items.len() && !self.items.is_empty() {
            self.selected = self.items.len() - 1;
        }
    }
}

pub fn handle_key(app: &mut App, key: KeyEvent) {
    // ---- form entry ----
    if app.todos.form.is_some() {
        form_key(app, key);
        return;
    }
    // ---- live filter entry ----
    if app.todos.filter_editing {
        match key.code {
            KeyCode::Enter | KeyCode::Esc => {
                if key.code == KeyCode::Esc {
                    app.todos.filter = None;
                }
                app.todos.filter_editing = false;
                app.mode = InputMode::Normal;
            }
            KeyCode::Backspace => {
                app.todos.filter.get_or_insert_default().pop();
            }
            KeyCode::Char(c) => app.todos.filter.get_or_insert_default().push(c),
            _ => {}
        }
        app.todos.load(&app.conn);
        return;
    }

    let n = app.todos.items.len();
    match key.code {
        KeyCode::Up | KeyCode::Char('k') if n > 0 => {
            app.todos.selected = app.todos.selected.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') if n > 0 => {
            app.todos.selected = (app.todos.selected + 1).min(n - 1);
        }
        KeyCode::Enter if n > 0 => {
            let row = &app.todos.items[app.todos.selected];
            if !row.is_subtask {
                let id = row.todo.id;
                app.todos.expanded = if app.todos.expanded == Some(id) {
                    None
                } else {
                    Some(id)
                };
                app.todos.load(&app.conn);
            }
        }
        KeyCode::Char(' ') | KeyCode::Char('x') if n > 0 => {
            let row = &app.todos.items[app.todos.selected];
            if row.todo.done_at.is_some() {
                // already completed (only reachable for an expanded, done subtask) — no-op
                return;
            }
            let (id, recurring, parent) = (
                row.todo.id,
                row.todo.recur_rule.is_some(),
                row.todo.parent_id,
            );
            // completing a top-level todo with open subtasks would orphan them — refuse
            if parent.is_none() {
                if let Ok((open, _)) = db::open_subtask_count(&app.conn, id) {
                    if open > 0 {
                        app.status =
                            Some(format!("{open} subtasks still open — finish them first"));
                        return;
                    }
                }
            }
            let spawned = db::todo_complete(&app.conn, id, app.today).unwrap_or(None);
            app.todos.last_completed = Some((id, spawned));
            if recurring {
                app.status = Some("↻ next occurrence scheduled".into());
            }
            // completing the last open subtask offers the parent via status line
            if let Some(pid) = parent {
                if let Ok((0, _)) = db::open_subtask_count(&app.conn, pid) {
                    app.status = Some("all subtasks done — space on parent to close it".into());
                }
            }
            app.todos.load(&app.conn);
            app.calendar.load(&app.conn);
        }
        KeyCode::Char('u') => {
            if let Some((id, spawned)) = app.todos.last_completed.take() {
                let _ = db::todo_uncomplete(&app.conn, id);
                if let Some(sid) = spawned {
                    let _ = db::todo_delete(&app.conn, sid);
                }
                app.todos.load(&app.conn);
                app.calendar.load(&app.conn);
            }
        }
        KeyCode::Char('d') if n > 0 => {
            let _ = db::todo_delete(&app.conn, app.todos.items[app.todos.selected].todo.id);
            app.todos.load(&app.conn);
            app.calendar.load(&app.conn);
        }
        KeyCode::Char('a') => open_form(app, None, None),
        KeyCode::Char('A') if n > 0 => {
            let row = &app.todos.items[app.todos.selected];
            let pid = row.todo.parent_id.unwrap_or(row.todo.id); // one level max
            open_form(app, None, Some(pid));
        }
        KeyCode::Char('e') if n > 0 => {
            let t = app.todos.items[app.todos.selected].todo.clone();
            open_form(app, Some(t), None);
        }
        KeyCode::Char('/') => {
            app.todos.filter = Some(String::new());
            app.todos.filter_editing = true;
            app.mode = InputMode::Editing;
        }
        KeyCode::Char('g') => {
            app.todos.group_by_project = !app.todos.group_by_project;
            app.todos.load(&app.conn);
        }
        KeyCode::Char('p') if n > 0 => {
            let (id, title) = {
                let row = &app.todos.items[app.todos.selected];
                (row.todo.id, row.todo.title.clone())
            };
            crate::ui::pomodoro::start(app, Some(id), Some(title));
        }
        _ => {}
    }
}

fn open_form(app: &mut App, edit: Option<Todo>, parent_id: Option<i64>) {
    let form = match edit {
        Some(t) => TodoForm {
            fields: [
                t.title.clone(),
                t.notes.clone(),
                t.priority.to_string(),
                t.due_date.map(|d| d.to_string()).unwrap_or_default(),
                t.project.clone().unwrap_or_default(),
                t.tags.clone(),
                t.recur_rule.clone().unwrap_or_default(),
            ],
            focus: 0,
            editing_id: Some(t.id),
            parent_id: t.parent_id,
        },
        None => TodoForm {
            fields: Default::default(),
            focus: 0,
            editing_id: None,
            parent_id,
        },
    };
    app.todos.form = Some(form);
    app.mode = InputMode::Editing;
}

fn form_key(app: &mut App, key: KeyEvent) {
    let form = app.todos.form.as_mut().unwrap();
    match key.code {
        KeyCode::Esc => {
            app.todos.form = None;
            app.mode = InputMode::Normal;
        }
        KeyCode::Tab | KeyCode::Down => form.focus = (form.focus + 1) % 7,
        KeyCode::BackTab | KeyCode::Up => form.focus = (form.focus + 6) % 7,
        KeyCode::Left | KeyCode::Right if form.focus == 2 => {
            // priority cycles 0→1→2
            let p: u8 = form.fields[2].parse().unwrap_or(0);
            let p = if key.code == KeyCode::Right {
                (p + 1) % 3
            } else {
                (p + 2) % 3
            };
            form.fields[2] = p.to_string();
        }
        KeyCode::Backspace => {
            form.fields[form.focus].pop();
        }
        KeyCode::Enter => {
            // validate + save
            let title = form.fields[0].trim().to_string();
            if title.is_empty() {
                app.status = Some("title is required".into());
                return;
            }
            let due = match form.fields[3].trim() {
                "" => None,
                s => match s.parse::<NaiveDate>() {
                    Ok(d) => Some(d),
                    Err(_) => {
                        app.status = Some("due must be YYYY-MM-DD".into());
                        return;
                    }
                },
            };
            let rule = match form.fields[6].trim() {
                "" => None,
                s => {
                    if recur::parse(s).is_none() {
                        app.status = Some("repeat: daily | weekly:mon,thu | every:3d".into());
                        return;
                    }
                    Some(s.to_string())
                }
            };
            let nt = NewTodo {
                title,
                notes: form.fields[1].clone(),
                priority: form.fields[2].parse().unwrap_or(0),
                due_date: due,
                project: match form.fields[4].trim() {
                    "" => None,
                    s => Some(s.into()),
                },
                tags: form.fields[5].trim().to_string(),
                parent_id: form.parent_id,
                recur_rule: rule,
            };
            let res = match form.editing_id {
                Some(id) => db::todo_update(&app.conn, id, &nt),
                None => db::todo_add(&app.conn, &nt).map(|_| ()),
            };
            if let Err(e) = res {
                app.status = Some(format!("save failed: {e}"));
            }
            app.todos.form = None;
            app.mode = InputMode::Normal;
            app.todos.load(&app.conn);
            app.calendar.load(&app.conn);
        }
        KeyCode::Char(c) if form.focus != 2 => form.fields[form.focus].push(c),
        _ => {}
    }
}

fn todo_line(app: &App, row: &Row, selected: bool) -> ListItem<'static> {
    let t = app.theme;
    let td = &row.todo;
    let done = td.done_at.is_some();
    let overdue = !done && td.due_date.map(|d| d < app.today).unwrap_or(false);

    let mut spans: Vec<Span> = Vec::new();
    if row.is_subtask {
        spans.push(Span::raw("    "));
    }
    let mark = if done {
        "✔"
    } else if td.priority == 2 {
        "◉"
    } else {
        "○"
    };
    let mark_color = if done {
        t.green
    } else if overdue || td.priority == 2 {
        t.red
    } else if td.priority == 1 {
        t.yellow
    } else {
        t.muted
    };
    spans.push(Span::styled(
        format!(" {mark} "),
        Style::default().fg(mark_color),
    ));

    let mut title_style = Style::default().fg(if overdue { t.red } else { t.text });
    if done {
        title_style = title_style.fg(t.muted).add_modifier(Modifier::CROSSED_OUT);
    }
    if selected {
        title_style = title_style.add_modifier(Modifier::BOLD | Modifier::REVERSED);
    }
    spans.push(Span::styled(td.title.clone(), title_style));

    if td.recur_rule.is_some() {
        spans.push(Span::styled(" ↻", Style::default().fg(t.blue)));
    }
    if let Some((open, total)) = row.sub_counts {
        spans.push(Span::styled(
            format!(" [{}/{}]", total - open, total),
            Style::default().fg(t.muted),
        ));
    }
    if let Some(d) = td.due_date {
        let c = if overdue { t.red } else { t.peach };
        spans.push(Span::styled(format!("  {d}"), Style::default().fg(c)));
    }
    if let Some(p) = &td.project {
        spans.push(Span::styled(format!("  #{p}"), Style::default().fg(t.blue)));
    }
    ListItem::new(Line::from(spans))
}

pub fn render_panel(f: &mut Frame, app: &mut App, area: Rect, focused: bool) {
    let block = app
        .theme
        .panel_block(&format!("TODOS ({})", app.todos.items.len()), focused);
    let items: Vec<ListItem> = app
        .todos
        .items
        .iter()
        .enumerate()
        .map(|(i, r)| todo_line(app, r, focused && i == app.todos.selected))
        .collect();
    f.render_widget(List::new(items).block(block), area);
}

pub fn render_zoomed(f: &mut Frame, app: &mut App) {
    let area = f.area();
    let rows = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(area);

    let mut title = String::from("TODOS");
    if let Some(fil) = &app.todos.filter {
        title = format!("TODOS · filter: {fil}");
    }
    if app.todos.group_by_project {
        title.push_str(" · by project");
    }
    let block = app.theme.panel_block(&title, true);
    let items: Vec<ListItem> = app
        .todos
        .items
        .iter()
        .enumerate()
        .map(|(i, r)| todo_line(app, r, i == app.todos.selected))
        .collect();
    f.render_widget(List::new(items).block(block), rows[0]);

    let hint = app.status.clone().unwrap_or_else(|| {
        if app.todos.filter_editing {
            format!(" filter: {}▏  (enter apply · esc clear)", app.todos.filter.as_deref().unwrap_or(""))
        } else {
            " a add · A subtask · e edit · space done · u undo · d delete · enter expand · / filter · g group · p pomodoro · esc home ".into()
        }
    });
    f.render_widget(Paragraph::new(hint).style(app.theme.hint()), rows[1]);

    if app.todos.form.is_some() {
        render_form(f, app, area);
    }
}

fn render_form(f: &mut Frame, app: &mut App, screen: Rect) {
    let form = app.todos.form.as_ref().unwrap();
    let t = app.theme;
    let w = 60.min(screen.width.saturating_sub(4));
    let h = 11u16;
    let popup = Rect {
        x: screen.x + (screen.width.saturating_sub(w)) / 2,
        y: screen.y + (screen.height.saturating_sub(h)) / 2,
        width: w,
        height: h,
    };
    f.render_widget(Clear, popup);
    let title = if form.editing_id.is_some() {
        "EDIT TODO"
    } else if form.parent_id.is_some() {
        "NEW SUBTASK"
    } else {
        "NEW TODO"
    };
    let block = t
        .panel_block(title, true)
        .style(Style::default().bg(t.surface));
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let lines: Vec<Line> = FIELD_LABELS
        .iter()
        .enumerate()
        .map(|(i, label)| {
            let focused = i == form.focus;
            let val = if i == 2 {
                ["low", "med", "high"][form.fields[2].parse::<usize>().unwrap_or(0).min(2)]
                    .to_string()
            } else {
                form.fields[i].clone()
            };
            let cursor = if focused && i != 2 { "▏" } else { "" };
            Line::from(vec![
                Span::styled(
                    format!(" {label:<18}"),
                    Style::default().fg(if focused { t.accent } else { t.muted }),
                ),
                Span::styled(format!("{val}{cursor}"), Style::default().fg(t.text)),
            ])
        })
        .collect();
    let mut all = lines;
    all.push(Line::from(Span::styled(
        "  tab next · ←/→ priority · enter save · esc cancel",
        Style::default().fg(t.muted),
    )));
    f.render_widget(Paragraph::new(all), inner);
}
