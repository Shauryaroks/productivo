use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::App;

/// Home = asymmetric bento grid on a transparent background. Config order
/// fills slots by importance, not equally:
///   [0] left rail top      [1] left rail middle (slim strip)
///   [2] left rail bottom   [3] center hero (largest)
///   [4] right top (month-grid sized)   [5] right bottom
/// Below 110 columns it degrades to the equal 2×3 grid (column-major).
pub fn render(f: &mut Frame, app: &mut App) {
    let area = f.area();
    let rows = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(area);

    let slots: Vec<Rect> = if rows[0].width >= 110 {
        let cols = Layout::horizontal([
            Constraint::Percentage(24),
            Constraint::Percentage(42),
            Constraint::Percentage(34),
        ])
        .split(rows[0]);
        let rail = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(4),
            Constraint::Fill(1),
        ])
        .split(cols[0]);
        let right = Layout::vertical([Constraint::Length(10), Constraint::Fill(1)]).split(cols[2]);
        vec![rail[0], rail[1], rail[2], cols[1], right[0], right[1]]
    } else {
        let cols = Layout::horizontal([Constraint::Percentage(42), Constraint::Percentage(58)])
            .split(rows[0]);
        let left = Layout::vertical([Constraint::Ratio(1, 3); 3]).split(cols[0]);
        let right = Layout::vertical([Constraint::Ratio(1, 3); 3]).split(cols[1]);
        left.iter().chain(right.iter()).copied().collect()
    };

    for (i, panel) in app.config.panels.clone().iter().enumerate() {
        let Some(&slot) = slots.get(i) else { break };
        render_panel(f, app, panel, slot, i == app.focus);
    }

    let hint = app
        .status
        .clone()
        .unwrap_or_else(|| " tab focus · enter zoom · 1-6 jump · q quit ".into());
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
    if panel == "pomodoro" {
        crate::ui::pomodoro::render_panel(f, app, area, focused);
        return;
    }
    // "stats" and any unrecognized panel name land here — matches screen_for's fallback to
    // Screen::Stats, so an unknown config entry still renders something instead of a dead
    // placeholder.
    crate::ui::stats::render_panel(f, app, area, focused);
}
