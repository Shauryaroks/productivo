pub mod ambient;
pub mod calendar;
pub mod habits;
pub mod home;
pub mod ideas;
pub mod pomodoro;
pub mod stats;
pub mod todos;

use ratatui::Frame;

use crate::app::{App, Screen};

pub fn render(f: &mut Frame, app: &mut App) {
    match app.screen {
        Screen::Home => home::render(f, app),
        Screen::Habits => habits::render_zoomed(f, app),
        Screen::Todos => todos::render_zoomed(f, app),
        Screen::Calendar => calendar::render_zoomed(f, app),
        Screen::Ideas => ideas::render_zoomed(f, app),
        Screen::Pomodoro => pomodoro::render_zoomed(f, app),
        Screen::Stats => stats::render_zoomed(f, app),
    }

    // Active pomodoro floats on top of every screen except its own zoom
    // (which already shows the big clock).
    if app.screen != Screen::Pomodoro {
        pomodoro::render_floating(f, app);
    }
}
