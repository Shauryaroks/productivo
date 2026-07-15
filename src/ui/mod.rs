pub mod home;

use ratatui::Frame;

use crate::app::{App, Screen};

pub fn render(f: &mut Frame, app: &mut App) {
    match app.screen {
        Screen::Home => home::render(f, app),
        // Zoomed module screens land in Tasks 3–10; until then everything is Home.
        _ => home::render(f, app),
    }
}
