use chrono::{Local, NaiveDate};
use crossterm::event::{KeyCode, KeyEvent};
use rusqlite::Connection;

use crate::config::Config;
use crate::theme::Theme;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Screen {
    Home,
    Habits,
    Todos,
    Calendar,
    Ideas,
    Pomodoro,
    Stats,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum InputMode {
    Normal,
    Editing,
}

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
    pub habits: crate::ui::habits::HabitsState,
    pub todos: crate::ui::todos::TodosState,
    pub calendar: crate::ui::calendar::CalendarState,
    pub ideas: crate::ui::ideas::IdeasState,
    pub pomodoro: crate::ui::pomodoro::PomodoroState,
    pub stats: crate::ui::stats::StatsState,
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
        let mut s = Self {
            conn,
            config,
            theme,
            screen: Screen::Home,
            mode: InputMode::Normal,
            focus: 0,
            should_quit: false,
            today: Local::now().date_naive(),
            status,
            habits: crate::ui::habits::HabitsState::default(),
            todos: crate::ui::todos::TodosState::default(),
            calendar: crate::ui::calendar::CalendarState::default(),
            ideas: crate::ui::ideas::IdeasState::default(),
            pomodoro: crate::ui::pomodoro::PomodoroState::default(),
            stats: crate::ui::stats::StatsState::default(),
        };
        s.habits.load(&s.conn, s.today);
        s.todos.load(&s.conn);
        s.calendar.load(&s.conn);
        s.ideas.load(&s.conn);
        s.pomodoro.load(&s.conn, s.today);
        crate::ui::pomodoro::restore_dangling(&mut s);
        s
    }

    /// The module keys act on: the zoomed screen, or the focused Home panel.
    /// Panels are fully interactive from Home — zooming is optional.
    pub fn active_module(&self) -> Screen {
        if self.screen == Screen::Home {
            screen_for(
                self.config
                    .panels
                    .get(self.focus)
                    .map(String::as_str)
                    .unwrap_or("stats"),
            )
        } else {
            self.screen
        }
    }

    fn dispatch_module(&mut self, module: Screen, key: KeyEvent) {
        match module {
            Screen::Habits => crate::ui::habits::handle_key(self, key),
            Screen::Todos => crate::ui::todos::handle_key(self, key),
            Screen::Calendar => crate::ui::calendar::handle_key(self, key),
            Screen::Ideas => crate::ui::ideas::handle_key(self, key),
            Screen::Pomodoro => crate::ui::pomodoro::handle_key(self, key),
            Screen::Stats => crate::ui::stats::handle_key(self, key),
            Screen::Home => {}
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        self.status = None;

        // Editing mode: dispatch straight to the active module; global keys don't run.
        if self.mode == InputMode::Editing {
            let module = self.active_module();
            self.dispatch_module(module, key);
            return;
        }

        // Global keys (Normal mode only)
        match key.code {
            KeyCode::Char('q') => {
                self.should_quit = true;
                return;
            }
            KeyCode::Esc => {
                if self.screen == Screen::Home {
                    // On Home Esc clears the habits yesterday-view if set.
                    if self.habits.day.is_some() {
                        self.habits.day = None;
                        self.habits.load(&self.conn, self.today);
                    }
                } else {
                    self.reset_habits_day_if_leaving();
                    self.screen = Screen::Home;
                }
                return;
            }
            KeyCode::Char(c @ '1'..='6') => {
                let idx = c as usize - '1' as usize;
                if let Some(p) = self.config.panels.get(idx).cloned() {
                    self.reset_habits_day_if_leaving();
                    self.screen = screen_for(&p);
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
                // Everything else acts on the focused panel in place.
                _ => {
                    let module = self.active_module();
                    self.dispatch_module(module, key);
                }
            }
            return;
        }

        self.dispatch_module(self.screen, key);
    }

    fn reset_habits_day_if_leaving(&mut self) {
        if self.screen == Screen::Habits && self.habits.day.is_some() {
            self.habits.day = None;
            self.habits.load(&self.conn, self.today);
        }
    }

    pub fn tick(&mut self) {
        crate::ui::pomodoro::on_tick(self);

        let now = Local::now().date_naive();
        if now != self.today {
            self.today = now;
            self.habits.day = None;
            self.habits.load(&self.conn, self.today);
            self.pomodoro.load(&self.conn, self.today);
        }
    }
}
