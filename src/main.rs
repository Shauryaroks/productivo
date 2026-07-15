mod app;
mod config;
mod theme;
mod ui;

use std::time::Duration;

use crossterm::event::{self, Event, KeyEventKind};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (config, warn) = config::load();
    // Temporary in-memory DB until Task 2 adds db::open().
    let conn = rusqlite::Connection::open_in_memory()?;
    let mut app = app::App::new(conn, config, warn);

    let mut terminal = ratatui::init(); // installs panic hook that restores the terminal
    let result = run(&mut terminal, &mut app);
    ratatui::restore();
    result
}

fn run(
    terminal: &mut ratatui::DefaultTerminal,
    app: &mut app::App,
) -> Result<(), Box<dyn std::error::Error>> {
    while !app.should_quit {
        terminal.draw(|f| ui::render(f, app))?;
        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    app.handle_key(key);
                }
            }
        }
        app.tick();
    }
    Ok(())
}
