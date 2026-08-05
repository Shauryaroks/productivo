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

    let mut ambient_area: Option<Rect> = None;
    let subs_area: Rect;
    let slots: Vec<Rect> = if rows[0].width >= 110 {
        let cols = Layout::horizontal([
            Constraint::Percentage(24),
            Constraint::Percentage(42),
            Constraint::Percentage(34),
        ])
        .split(rows[0]);
        // Rail-top fits its content (habit count) instead of claiming half the rail.
        let rail_top = (app.habits.items.len() as u16 + 2).clamp(5, rows[0].height / 3);
        // Rail middle gets 8 rows so the pomodoro panel fits its big clock.
        let rail = Layout::vertical([
            Constraint::Length(rail_top),
            Constraint::Length(8),
            Constraint::Fill(1),
        ])
        .split(cols[0]);
        // Center: ambient aurora strip on top, then todos and subs split equally.
        let center = Layout::vertical([
            Constraint::Length(8),
            Constraint::Fill(1),
            Constraint::Fill(1),
        ])
        .split(cols[1]);
        ambient_area = Some(center[0]);
        subs_area = center[2];
        // Right-top sized for the month grid WITH event-dot rows (calendar
        // renders dots whenever it gets this much height).
        let right = Layout::vertical([Constraint::Length(16), Constraint::Fill(1)]).split(cols[2]);
        vec![rail[0], rail[1], rail[2], center[1], right[0], right[1]]
    } else {
        // Narrow fallback: grid on top, subs strip full-width along the bottom,
        // as tall as one grid cell (grid is 3 rows of cells → strip gets 1/4).
        let split =
            Layout::vertical([Constraint::Fill(3), Constraint::Fill(1)]).split(rows[0]);
        subs_area = split[1];
        let cols = Layout::horizontal([Constraint::Percentage(42), Constraint::Percentage(58)])
            .split(split[0]);
        // ponytail: middle-left is pomodoro with the default panel order — give
        // it the 8 rows its big clock needs; revisit if panels get reordered.
        let left = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(8),
            Constraint::Fill(1),
        ])
        .split(cols[0]);
        // ponytail: middle-right is calendar with the default panel order — 9
        // rows fits a full compact month; revisit if panels get reordered.
        let right = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(9),
            Constraint::Fill(1),
        ])
        .split(cols[1]);
        left.iter().chain(right.iter()).copied().collect()
    };

    for (i, panel) in app.config.panels.clone().iter().enumerate() {
        let Some(&slot) = slots.get(i) else { break };
        render_panel(f, app, panel, slot, i == app.focus);
    }
    crate::ui::subs::render_panel(f, app, subs_area, app.focus == app.config.panels.len());

    if let Some(aa) = ambient_area {
        crate::ui::ambient::render(f, app, aa);
    }

    let hint = app
        .status
        .clone()
        .or_else(|| crate::ui::habits::input_hint(app))
        .or_else(|| crate::ui::todos::input_hint(app))
        .or_else(|| crate::ui::ideas::input_hint(app))
        .or_else(|| crate::ui::subs::input_hint(app))
        .unwrap_or_else(|| panel_hint(app));
    f.render_widget(
        Paragraph::new(Line::from(hint)).style(app.theme.hint()),
        rows[1],
    );

    // Panels are editable from Home: forms opened here render as popups over the grid.
    if app.todos.form.is_some() {
        crate::ui::todos::render_form(f, app, area);
    }
    if app.calendar.form.is_some() {
        crate::ui::calendar::render_event_form(f, app, area);
    }
}

/// Bottom-bar hint for the focused panel's own keys (the generic navigation
/// tail stays constant). Keys mirror each module's zoomed-screen hint.
fn panel_hint(app: &App) -> String {
    use crate::app::Screen;
    let keys = match app.active_module() {
        Screen::Habits => " space check · a add · d archive · J/K reorder · y yesterday".into(),
        Screen::Todos => " a add · e edit · space done · u undo · d delete · / filter · p pomodoro".into(),
        Screen::Calendar => " ←↓↑→ move · [/] month · t today · a add event · d delete".into(),
        Screen::Ideas => " a capture · s cycle status · d delete".into(),
        Screen::Pomodoro => format!(
            " s start · space pause · x abandon · +/- focus {}m · [/] break {}m",
            app.config.pomodoro.focus_min, app.config.pomodoro.break_min
        ),
        Screen::Stats => " p pet · b boop · c skin · r range".into(),
        Screen::Subs => " a add sub · t add tool · d delete".into(),
        Screen::Home => String::new(),
    };
    format!("{keys} · tab next · enter zoom · q quit ")
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
