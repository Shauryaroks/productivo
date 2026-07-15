use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::App;

/// Home = 2 columns × 3 rows, panels assigned column-major from config order.
pub fn render(f: &mut Frame, app: &mut App) {
    let area = f.area();
    f.render_widget(
        ratatui::widgets::Block::default().style(Style::default().bg(app.theme.bg)),
        area,
    );

    let rows = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(area);
    let cols = Layout::horizontal([Constraint::Percentage(42), Constraint::Percentage(58)])
        .split(rows[0]);
    let left = Layout::vertical([Constraint::Ratio(1, 3); 3]).split(cols[0]);
    let right = Layout::vertical([Constraint::Ratio(1, 3); 3]).split(cols[1]);
    let slots: Vec<Rect> = left.iter().chain(right.iter()).copied().collect();

    for (i, panel) in app.config.panels.clone().iter().enumerate() {
        let Some(&slot) = slots.get(i) else { break };
        render_panel(f, app, panel, slot, i == app.focus);
    }

    let hint = app.status.clone().unwrap_or_else(|| {
        " tab focus · enter zoom · 1-6 jump · q quit ".into()
    });
    f.render_widget(
        Paragraph::new(Line::from(hint)).style(app.theme.hint()),
        rows[1],
    );
}

fn render_panel(f: &mut Frame, app: &mut App, panel: &str, area: Rect, focused: bool) {
    if panel == "habits" {
        crate::ui::habits::render_panel(f, app, area, focused);
        return;
    }
    if panel == "todos" {
        crate::ui::todos::render_panel(f, app, area, focused);
        return;
    }
    if panel == "calendar" {
        crate::ui::calendar::render_panel(f, app, area, focused);
        return;
    }
    if panel == "ideas" {
        crate::ui::ideas::render_panel(f, app, area, focused);
        return;
    }
    let title = match panel {
        "pomodoro" => "POMODORO", _ => "STATS",
    };
    let block = app.theme.panel_block(title, focused);
    // Module panel bodies replace this placeholder in Tasks 4–10.
    f.render_widget(
        Paragraph::new("…").style(Style::default().fg(app.theme.muted)).block(block),
        area,
    );
}
