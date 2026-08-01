use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, ListState, Paragraph};
use ratatui::Frame;

use crate::app::{App, InputMode};
use crate::db;

#[derive(Default)]
pub struct SubsState {
    pub items: Vec<crate::models::Sub>,
    pub selected: usize,
    /// Some((kind, buffer)) = add-mode text entry; kind is "sub" or "tool".
    pub input: Option<(String, String)>,
}

impl SubsState {
    pub fn load(&mut self, conn: &rusqlite::Connection) {
        self.items = db::subs_list(conn).unwrap_or_default();
        if self.selected >= self.items.len() && !self.items.is_empty() {
            self.selected = self.items.len() - 1;
        }
    }

    /// Sum of subscription prices, yearly ones counted as price/12.
    pub fn monthly_total(&self) -> f64 {
        self.items
            .iter()
            .filter(|s| s.kind == "sub")
            .filter_map(|s| {
                s.price
                    .map(|p| if s.cycle == "yearly" { p / 12.0 } else { p })
            })
            .sum()
    }
}

/// "name [price] [renew day] [y|yearly]" — up to two trailing numbers are
/// price then day; a y/yr/yearly token anywhere marks a yearly cycle.
fn parse_sub(input: &str) -> (String, Option<f64>, Option<u32>, &'static str) {
    let mut toks: Vec<&str> = input.split_whitespace().collect();
    let mut cycle = "monthly";
    toks.retain(|t| {
        let yearly = matches!(t.to_ascii_lowercase().as_str(), "y" | "yr" | "yearly");
        if yearly {
            cycle = "yearly";
        }
        !yearly
    });
    let mut nums: Vec<f64> = Vec::new();
    while nums.len() < 2 && toks.len() > 1 {
        match toks.last().and_then(|t| t.parse::<f64>().ok()) {
            Some(n) => {
                nums.insert(0, n);
                toks.pop();
            }
            None => break,
        }
    }
    let (price, day) = match nums.len() {
        2 => (Some(nums[0]), Some(nums[1] as u32)),
        1 => (Some(nums[0]), None),
        _ => (None, None),
    };
    (
        toks.join(" "),
        price,
        day.filter(|d| (1..=31).contains(d)),
        cycle,
    )
}

pub fn handle_key(app: &mut App, key: KeyEvent) {
    // add-mode text entry
    if let Some((kind, buf)) = app.subs.input.as_mut() {
        match key.code {
            KeyCode::Enter => {
                let kind = kind.clone();
                let text = buf.trim().to_string();
                if !text.is_empty() {
                    let (name, price, day, cycle) = if kind == "sub" {
                        parse_sub(&text)
                    } else {
                        (text, None, None, "monthly")
                    };
                    if !name.is_empty() {
                        let _ = db::sub_add(&app.conn, &name, &kind, price, cycle, day);
                    }
                }
                app.subs.input = None;
                app.mode = InputMode::Normal;
                app.subs.load(&app.conn);
            }
            KeyCode::Esc => {
                app.subs.input = None;
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

    let n = app.subs.items.len();
    match key.code {
        KeyCode::Up | KeyCode::Char('k') if n > 0 => {
            app.subs.selected = app.subs.selected.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') if n > 0 => {
            app.subs.selected = (app.subs.selected + 1).min(n - 1);
        }
        KeyCode::Char('a') => {
            app.subs.input = Some(("sub".into(), String::new()));
            app.mode = InputMode::Editing;
        }
        KeyCode::Char('t') => {
            app.subs.input = Some(("tool".into(), String::new()));
            app.mode = InputMode::Editing;
        }
        KeyCode::Char('d') if n > 0 => {
            let _ = db::sub_delete(&app.conn, app.subs.items[app.subs.selected].id);
            app.subs.load(&app.conn);
        }
        _ => {}
    }
}

fn sub_lines(app: &App) -> Vec<ListItem<'static>> {
    let t = app.theme;
    app.subs
        .items
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let (mark, mark_color) = if s.kind == "sub" {
                ("◆", t.blue)
            } else {
                ("⚒", t.muted)
            };
            let mut name_style = Style::default().fg(t.text);
            if i == app.subs.selected {
                name_style = name_style.add_modifier(Modifier::BOLD).fg(t.accent);
            }
            let mut spans = vec![
                Span::styled(format!(" {mark} "), Style::default().fg(mark_color)),
                Span::styled(s.name.clone(), name_style),
            ];
            if let Some(p) = s.price {
                let per = if s.cycle == "yearly" { "/y" } else { "" };
                spans.push(Span::styled(
                    format!("  {p:.0}{per}"),
                    Style::default().fg(t.peach),
                ));
            }
            if let Some(d) = s.renew_day {
                spans.push(Span::styled(
                    format!(" · d{d}"),
                    Style::default().fg(t.muted),
                ));
            }
            ListItem::new(Line::from(spans))
        })
        .collect()
}

fn title(app: &App) -> String {
    let total = app.subs.monthly_total();
    if total > 0.0 {
        format!("SUBS & TOOLS · {total:.0}/mo")
    } else {
        "SUBS & TOOLS".into()
    }
}

/// Text-entry hint for the bottom bar — shown on both the zoomed screen and Home.
pub fn input_hint(app: &App) -> Option<String> {
    app.subs.input.as_ref().map(|(kind, buf)| {
        if kind == "sub" {
            format!(" new sub (name price day, add y if yearly): {buf}▏  (enter save · esc cancel)")
        } else {
            format!(" new tool: {buf}▏  (enter save · esc cancel)")
        }
    })
}

pub fn render_panel(f: &mut Frame, app: &mut App, area: Rect, focused: bool) {
    let block = app.theme.panel_block(&title(app), focused);
    let mut st = ListState::default();
    st.select(Some(app.subs.selected));
    f.render_stateful_widget(List::new(sub_lines(app)).block(block), area, &mut st);
}

pub fn render_zoomed(f: &mut Frame, app: &mut App) {
    let area = f.area();
    let rows = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(area);
    let block = app.theme.panel_block(&title(app), true);
    let mut st = ListState::default();
    st.select(Some(app.subs.selected));
    f.render_stateful_widget(List::new(sub_lines(app)).block(block), rows[0], &mut st);

    let hint = app
        .status
        .clone()
        .or_else(|| input_hint(app))
        .unwrap_or_else(|| " a add sub · t add tool · d delete · j/k move · esc home ".into());
    f.render_widget(Paragraph::new(hint).style(app.theme.hint()), rows[1]);
}

#[cfg(test)]
mod tests {
    use super::parse_sub;

    #[test]
    fn parses_trailing_numbers() {
        assert_eq!(
            parse_sub("Google One 130 12"),
            ("Google One".into(), Some(130.0), Some(12), "monthly")
        );
        assert_eq!(
            parse_sub("Spotify 119"),
            ("Spotify".into(), Some(119.0), None, "monthly")
        );
        assert_eq!(parse_sub("yazi"), ("yazi".into(), None, None, "monthly"));
        // numeric-looking name alone stays a name, not a price
        assert_eq!(parse_sub("365"), ("365".into(), None, None, "monthly"));
        // day out of range is dropped
        assert_eq!(
            parse_sub("Netflix 649 99"),
            ("Netflix".into(), Some(649.0), None, "monthly")
        );
        // y token anywhere marks yearly billing
        assert_eq!(
            parse_sub("Google One 1300 y 12"),
            ("Google One".into(), Some(1300.0), Some(12), "yearly")
        );
    }
}
