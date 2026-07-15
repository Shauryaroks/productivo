pub mod habits;
pub mod home;

use ratatui::Frame;

use crate::app::{App, Screen};

pub fn render(f: &mut Frame, app: &mut App) {
    match app.screen {
        Screen::Home => home::render(f, app),
        Screen::Habits => habits::render_zoomed(f, app),
        // Remaining zoomed module screens land in Tasks 4–10; until then everything is Home.
        _ => home::render(f, app),
    }
}
