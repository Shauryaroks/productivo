use chrono::{Local, NaiveDate};
use crossterm::event::{KeyCode, KeyEvent};
use rusqlite::Connection;

use crate::config::Config;
use crate::theme::Theme;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Screen { Home, Habits, Todos, Calendar, Ideas, Pomodoro, Stats }

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum InputMode { Normal, Editing }

pub struct App {
    pub conn: Connection,
    pub config: Config,
    pub theme: Theme,
    pub screen: Screen,
    pub mode: InputMode,
    pub focus: usize, // focused panel index on Home (into config.panels)
    pub should_quit: bool,
    pub today: NaiveDate,
    pub status: Option<String>,
}

pub fn screen_for(panel: &str) -> Screen {
    match panel {
        "habits" => Screen::Habits,
        "todos" => Screen::Todos,
        "calendar" => Screen::Calendar,
        "ideas" => Screen::Ideas,
        "pomodoro" => Screen::Pomodoro,
        _ => Screen::Stats,
    }
}

impl App {
    pub fn new(conn: Connection, config: Config, status: Option<String>) -> Self {
        let theme = Theme::from_cfg(&config.theme);
        Self {
            conn,
            config,
            theme,
            screen: Screen::Home,
            mode: InputMode::Normal,
            focus: 0,
            should_quit: false,
            today: Local::now().date_naive(),
            status,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        self.status = None;
        // Global keys (Normal mode only; Editing is handled per-module later)
        if self.mode == InputMode::Normal {
            match key.code {
                KeyCode::Char('q') => { self.should_quit = true; return; }
                KeyCode::Esc => { self.screen = Screen::Home; return; }
                KeyCode::Char(c @ '1'..='6') => {
                    let idx = c as usize - '1' as usize;
                    if let Some(p) = self.config.panels.get(idx) {
                        self.screen = screen_for(p);
                    }
                    return;
                }
                _ => {}
            }
            if self.screen == Screen::Home {
                match key.code {
                    KeyCode::Tab => {
                        self.focus = (self.focus + 1) % self.config.panels.len();
                    }
                    KeyCode::BackTab => {
                        let n = self.config.panels.len();
                        self.focus = (self.focus + n - 1) % n;
                    }
                    KeyCode::Enter => {
                        self.screen = screen_for(&self.config.panels[self.focus].clone());
                    }
                    _ => {}
                }
            }
        }
    }

    pub fn tick(&mut self) {
        let now = Local::now().date_naive();
        if now != self.today {
            self.today = now; // midnight rollover; modules reload in later tasks
        }
    }
}
