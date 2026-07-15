pub mod habits;
pub mod home;
pub mod todos;

use ratatui::Frame;

use crate::app::{App, Screen};

pub fn render(f: &mut Frame, app: &mut App) {
    match app.screen {
        Screen::Home => home::render(f, app),
        Screen::Habits => habits::render_zoomed(f, app),
        Screen::Todos => todos::render_zoomed(f, app),
        // Remaining zoomed module screens land in Tasks 7–10; until then everything is Home.
        _ => home::render(f, app),
    }
}
