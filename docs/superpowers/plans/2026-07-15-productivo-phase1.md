# Productivo Phase 1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A Rust TUI productivity dashboard (habits, rich todos, calendar, ideas, pomodoro, stats) backed by a local SQLite file, shipped as a single binary.

**Architecture:** Single crate, fully synchronous. One `App` struct owns all state; a classic ratatui event loop (draw → poll 250ms → handle key → tick). All SQL lives in `db.rs` behind plain functions — the UI never touches SQL. Pure logic (recurrence, streaks) is unit-tested; UI is verified by running the app.

**Tech Stack:** Rust 2021, ratatui 0.29, crossterm 0.28, rusqlite 0.32 (`bundled`), serde + toml, chrono, directories.

**Spec:** `docs/superpowers/specs/2026-07-15-productivo-tui-dashboard-design.md`

## Global Constraints

- Dependencies are EXACTLY: `ratatui`, `crossterm`, `rusqlite` (bundled), `serde`, `toml`, `chrono`, `directories`. No tokio, no anyhow, no other crates without user approval.
- Fully synchronous. No threads, no channels, no async.
- All SQL lives in `src/db.rs`. UI and app code call `db::` functions only.
- Dates stored as TEXT `YYYY-MM-DD`; timestamps as RFC3339 TEXT (UTC).
- DB file: platform data dir via `directories` (`ProjectDirs::from("", "", "productivo")`) → `<data_dir>/dash.db`. Config: `<config_dir>/config.toml`.
- **Aesthetic rule:** every color comes from the `Theme` struct — no hardcoded `Color::` values outside `src/theme.rs`. All panels use rounded borders via `Theme::panel_block`. Focused panel border = accent, unfocused = muted. Consistent footer hint bar on every screen.
- Never panic on user input or DB errors mid-session; errors go to the status line (`app.status`).
- Tests are inline `#[cfg(test)] mod tests` in the file they test, using in-memory SQLite (`Connection::open_in_memory()`) where a DB is needed.
- Commit after every task (git is initialized in Task 1; execution-time git is expected and approved by the plan).
- Binary/crate name: `productivo`.

---

### Task 1: Project skeleton — theme, config, event loop, empty home grid

**Files:**
- Create: `Cargo.toml`, `.gitignore`, `src/main.rs`, `src/app.rs`, `src/config.rs`, `src/theme.rs`, `src/ui/mod.rs`, `src/ui/home.rs`

**Interfaces:**
- Produces: `config::Config { panels: Vec<String>, pomodoro: PomodoroCfg { focus_min: u64, break_min: u64 }, theme: ThemeCfg }`, `config::load() -> Config`
- Produces: `theme::Theme` (colors: `bg, surface, text, muted, accent, green, red, yellow, blue, peach`), `Theme::panel_block(&self, title: &str, focused: bool) -> Block`, `Theme::from_cfg(&ThemeCfg) -> Theme`
- Produces: `app::App { screen: Screen, mode: InputMode, focus: usize, should_quit: bool, today: NaiveDate, status: Option<String>, config, theme, conn }` with `handle_key(&mut self, KeyEvent)`, `tick(&mut self)`; `Screen` enum `{ Home, Habits, Todos, Calendar, Ideas, Pomodoro, Stats }`; `InputMode { Normal, Editing }`
- Produces: `ui::render(f: &mut Frame, app: &mut App)` dispatch; `ui::home::render(f, app)`

- [ ] **Step 1: Init project**

```bash
mkdir -p /home/shaurya/Projects/productivo && cd /home/shaurya/Projects/productivo
git init
cargo init --name productivo
printf 'target/\n' > .gitignore
```

`Cargo.toml`:

```toml
[package]
name = "productivo"
version = "0.1.0"
edition = "2021"

[dependencies]
ratatui = "0.29"
crossterm = "0.28"
rusqlite = { version = "0.32", features = ["bundled"] }
serde = { version = "1", features = ["derive"] }
toml = "0.8"
chrono = "0.4"
directories = "5"
```

- [ ] **Step 2: Write failing config test**

`src/config.rs`:

```rust
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
#[serde(default)]
pub struct Config {
    pub panels: Vec<String>,
    pub pomodoro: PomodoroCfg,
    pub theme: ThemeCfg,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(default)]
pub struct PomodoroCfg {
    pub focus_min: u64,
    pub break_min: u64,
}

#[derive(Deserialize, Debug, Clone, Default)]
#[serde(default)]
pub struct ThemeCfg {
    // optional hex overrides, e.g. accent = "#cba6f7"
    pub accent: Option<String>,
    pub green: Option<String>,
    pub red: Option<String>,
    pub yellow: Option<String>,
    pub blue: Option<String>,
    pub peach: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            panels: ["habits", "calendar", "ideas", "todos", "pomodoro", "stats"]
                .map(String::from)
                .to_vec(),
            pomodoro: PomodoroCfg::default(),
            theme: ThemeCfg::default(),
        }
    }
}

impl Default for PomodoroCfg {
    fn default() -> Self {
        Self { focus_min: 25, break_min: 5 }
    }
}

/// Load config from <config_dir>/config.toml; fall back to defaults on any failure.
/// Returns (config, warning) — warning is shown in the status line.
pub fn load() -> (Config, Option<String>) {
    let Some(dirs) = directories::ProjectDirs::from("", "", "productivo") else {
        return (Config::default(), None);
    };
    let path = dirs.config_dir().join("config.toml");
    match std::fs::read_to_string(&path) {
        Ok(s) => match toml::from_str(&s) {
            Ok(c) => (c, None),
            Err(e) => (Config::default(), Some(format!("config.toml invalid, using defaults: {e}"))),
        },
        Err(_) => (Config::default(), None), // no file = defaults, not an error
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_six_panels() {
        let c = Config::default();
        assert_eq!(c.panels.len(), 6);
        assert_eq!(c.pomodoro.focus_min, 25);
    }

    #[test]
    fn partial_toml_fills_defaults() {
        let c: Config = toml::from_str("panels = [\"todos\", \"stats\"]\n[pomodoro]\nfocus_min = 50\n").unwrap();
        assert_eq!(c.panels, vec!["todos", "stats"]);
        assert_eq!(c.pomodoro.focus_min, 50);
        assert_eq!(c.pomodoro.break_min, 5);
    }
}
```

- [ ] **Step 3: Run tests — expect compile failure (crate skeleton incomplete), then continue building the skeleton so they pass**

Run: `cargo test`
Expected: fails until Steps 4–7 are in place.

- [ ] **Step 4: Theme**

`src/theme.rs`:

```rust
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, BorderType, Borders};

use crate::config::ThemeCfg;

#[derive(Clone, Copy)]
pub struct Theme {
    pub bg: Color,
    pub surface: Color,
    pub text: Color,
    pub muted: Color,
    pub accent: Color,
    pub green: Color,
    pub red: Color,
    pub yellow: Color,
    pub blue: Color,
    pub peach: Color,
}

impl Default for Theme {
    // Catppuccin Mocha — calm, high-contrast, terminal-native
    fn default() -> Self {
        Self {
            bg: Color::Rgb(30, 30, 46),
            surface: Color::Rgb(49, 50, 68),
            text: Color::Rgb(205, 214, 244),
            muted: Color::Rgb(127, 132, 156),
            accent: Color::Rgb(203, 166, 247),
            green: Color::Rgb(166, 227, 161),
            red: Color::Rgb(243, 139, 168),
            yellow: Color::Rgb(249, 226, 175),
            blue: Color::Rgb(137, 180, 250),
            peach: Color::Rgb(250, 179, 135),
        }
    }
}

fn hex(s: &str) -> Option<Color> {
    let s = s.strip_prefix('#')?;
    if s.len() != 6 { return None; }
    let r = u8::from_str_radix(&s[0..2], 16).ok()?;
    let g = u8::from_str_radix(&s[2..4], 16).ok()?;
    let b = u8::from_str_radix(&s[4..6], 16).ok()?;
    Some(Color::Rgb(r, g, b))
}

impl Theme {
    pub fn from_cfg(cfg: &ThemeCfg) -> Self {
        let mut t = Self::default();
        if let Some(c) = cfg.accent.as_deref().and_then(hex) { t.accent = c; }
        if let Some(c) = cfg.green.as_deref().and_then(hex) { t.green = c; }
        if let Some(c) = cfg.red.as_deref().and_then(hex) { t.red = c; }
        if let Some(c) = cfg.yellow.as_deref().and_then(hex) { t.yellow = c; }
        if let Some(c) = cfg.blue.as_deref().and_then(hex) { t.blue = c; }
        if let Some(c) = cfg.peach.as_deref().and_then(hex) { t.peach = c; }
        t
    }

    /// Every panel in the app is drawn with this block. Rounded, titled, focus-aware.
    pub fn panel_block(&self, title: &str, focused: bool) -> Block<'static> {
        let border = if focused { self.accent } else { self.muted };
        let title_style = if focused {
            Style::default().fg(self.accent).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(self.text)
        };
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border))
            .title(ratatui::text::Span::styled(format!(" {title} "), title_style))
    }

    pub fn hint(&self) -> Style {
        Style::default().fg(self.muted)
    }
}
```

- [ ] **Step 5: App state**

`src/app.rs`:

```rust
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
```

- [ ] **Step 6: UI dispatch + home grid**

`src/ui/mod.rs`:

```rust
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
```

`src/ui/home.rs`:

```rust
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
    let title = match panel {
        "habits" => "HABITS", "todos" => "TODOS", "calendar" => "CALENDAR",
        "ideas" => "IDEAS", "pomodoro" => "POMODORO", _ => "STATS",
    };
    let block = app.theme.panel_block(title, focused);
    // Module panel bodies replace this placeholder in Tasks 3–10.
    f.render_widget(
        Paragraph::new("…").style(Style::default().fg(app.theme.muted)).block(block),
        area,
    );
}
```

- [ ] **Step 7: Main + event loop**

`src/main.rs`:

```rust
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
```

- [ ] **Step 8: Verify**

Run: `cargo test`
Expected: PASS (2 config tests).

Run: `cargo run`
Expected: full-screen dashboard, 6 rounded-border panels (HABITS/CALENDAR/IDEAS left, TODOS/POMODORO/STATS right), focused panel highlighted in mauve, hint bar at bottom. `Tab` moves focus, `q` quits, terminal restored cleanly.

- [ ] **Step 9: Commit**

```bash
git add -A && git commit -m "feat: skeleton — event loop, theme, config, home grid"
```

---

### Task 2: DB foundation — open, migrate, models

**Files:**
- Create: `src/db.rs`, `src/models.rs`
- Modify: `src/main.rs` (use `db::open()`)

**Interfaces:**
- Produces: `db::open() -> rusqlite::Result<Connection>` (real file), `db::migrate(conn: &Connection) -> rusqlite::Result<()>` (idempotent, used by tests on in-memory conns)
- Produces (models, all `pub` fields): `Todo { id: i64, title: String, notes: String, priority: u8, due_date: Option<NaiveDate>, project: Option<String>, tags: String, parent_id: Option<i64>, recur_rule: Option<String>, done_at: Option<String>, created_at: String }`, `Habit { id: i64, name: String, position: i64, archived: bool }`, `Event { id: i64, title: String, date: NaiveDate, time: Option<String>, category: String, color: String, notes: String }`, `Idea { id: i64, title: String, body: String, status: String, created_at: String }`

- [ ] **Step 1: Write failing migration test** (bottom of new `src/db.rs`)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrate_creates_all_tables_and_is_idempotent() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        migrate(&conn).unwrap();
        migrate(&conn).unwrap(); // idempotent
        let count: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name IN
                 ('todos','habits','habit_log','events','ideas','pomodoros')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 6);
        let v: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
        assert_eq!(v, 1);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test migrate_creates`
Expected: FAIL — `migrate` not defined.

- [ ] **Step 3: Implement `src/db.rs` (top of file) and `src/models.rs`**

`src/db.rs`:

```rust
use rusqlite::Connection;

pub fn open() -> rusqlite::Result<Connection> {
    let dirs = directories::ProjectDirs::from("", "", "productivo")
        .expect("no home directory found");
    std::fs::create_dir_all(dirs.data_dir()).expect("cannot create data dir");
    let conn = Connection::open(dirs.data_dir().join("dash.db"))?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    migrate(&conn)?;
    Ok(conn)
}

pub fn migrate(conn: &Connection) -> rusqlite::Result<()> {
    let v: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if v < 1 {
        conn.execute_batch(
            "BEGIN;
            CREATE TABLE todos (
                id INTEGER PRIMARY KEY,
                title TEXT NOT NULL,
                notes TEXT NOT NULL DEFAULT '',
                priority INTEGER NOT NULL DEFAULT 0,
                due_date TEXT,
                project TEXT,
                tags TEXT NOT NULL DEFAULT '',
                parent_id INTEGER REFERENCES todos(id) ON DELETE CASCADE,
                recur_rule TEXT,
                done_at TEXT,
                created_at TEXT NOT NULL
            );
            CREATE TABLE habits (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                position INTEGER NOT NULL,
                archived INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL
            );
            CREATE TABLE habit_log (
                habit_id INTEGER NOT NULL REFERENCES habits(id) ON DELETE CASCADE,
                date TEXT NOT NULL,
                UNIQUE(habit_id, date)
            );
            CREATE TABLE events (
                id INTEGER PRIMARY KEY,
                title TEXT NOT NULL,
                date TEXT NOT NULL,
                time TEXT,
                category TEXT NOT NULL DEFAULT 'general',
                color TEXT NOT NULL DEFAULT 'blue',
                notes TEXT NOT NULL DEFAULT ''
            );
            CREATE TABLE ideas (
                id INTEGER PRIMARY KEY,
                title TEXT NOT NULL,
                body TEXT NOT NULL DEFAULT '',
                status TEXT NOT NULL DEFAULT 'spark',
                created_at TEXT NOT NULL
            );
            CREATE TABLE pomodoros (
                id INTEGER PRIMARY KEY,
                todo_id INTEGER REFERENCES todos(id) ON DELETE SET NULL,
                started_at TEXT NOT NULL,
                ended_at TEXT,
                kind TEXT NOT NULL,
                completed INTEGER NOT NULL DEFAULT 0
            );
            PRAGMA user_version = 1;
            COMMIT;",
        )?;
    }
    Ok(())
}
```

`src/models.rs`:

```rust
use chrono::NaiveDate;

#[derive(Debug, Clone)]
pub struct Todo {
    pub id: i64,
    pub title: String,
    pub notes: String,
    pub priority: u8, // 0 low, 1 med, 2 high
    pub due_date: Option<NaiveDate>,
    pub project: Option<String>,
    pub tags: String, // csv
    pub parent_id: Option<i64>,
    pub recur_rule: Option<String>,
    pub done_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct Habit {
    pub id: i64,
    pub name: String,
    pub position: i64,
    pub archived: bool,
}

#[derive(Debug, Clone)]
pub struct Event {
    pub id: i64,
    pub title: String,
    pub date: NaiveDate,
    pub time: Option<String>,
    pub category: String,
    pub color: String,
    pub notes: String,
}

#[derive(Debug, Clone)]
pub struct Idea {
    pub id: i64,
    pub title: String,
    pub body: String,
    pub status: String, // spark|brewing|active|shipped|dropped
    pub created_at: String,
}
```

In `src/main.rs`: add `mod db; mod models;` and replace the in-memory connection line with:

```rust
    let conn = match db::open() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("productivo: cannot open database: {e}");
            std::process::exit(1);
        }
    };
```

- [ ] **Step 4: Run tests to verify pass**

Run: `cargo test`
Expected: PASS (config + migration). `cargo run` still works; a `dash.db` now exists in the platform data dir.

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat: sqlite schema v1, models, db::open"
```

---

### Task 3: Habits module

**Files:**
- Modify: `src/db.rs` (habit functions + tests), `src/app.rs`, `src/ui/mod.rs`, `src/ui/home.rs`
- Create: `src/ui/habits.rs`

**Interfaces:**
- Consumes: `db::migrate`, `models::Habit`, `Theme::panel_block`, `App` fields from Task 1.
- Produces in `db.rs`:
  - `habits_list(conn: &Connection) -> rusqlite::Result<Vec<Habit>>` (unarchived, by position)
  - `habit_add(conn: &Connection, name: &str) -> rusqlite::Result<()>`
  - `habit_archive(conn: &Connection, id: i64) -> rusqlite::Result<()>`
  - `habit_move(conn: &Connection, id: i64, delta: i64) -> rusqlite::Result<()>` (swap position with neighbor)
  - `habit_toggle(conn: &Connection, id: i64, date: NaiveDate) -> rusqlite::Result<()>`
  - `habit_checked_on(conn: &Connection, date: NaiveDate) -> rusqlite::Result<Vec<i64>>` (habit ids checked that day)
  - `habit_streak(conn: &Connection, id: i64, today: NaiveDate) -> rusqlite::Result<u32>`
- Produces in `ui/habits.rs`: `HabitsState { items: Vec<Habit>, checked: Vec<i64>, selected: usize, day: NaiveDate, input: Option<String> }`, `HabitsState::load(&mut self, conn, day)`, `pub fn handle_key(app: &mut App, key: KeyEvent)`, `pub fn render_panel(f, app: &mut App, area, focused: bool)`, `pub fn render_zoomed(f, app: &mut App)`
- App gains field `pub habits: crate::ui::habits::HabitsState`.

- [ ] **Step 1: Write failing streak + toggle tests** (in `src/db.rs` tests mod)

```rust
    fn d(s: &str) -> chrono::NaiveDate { s.parse().unwrap() }

    fn test_conn() -> rusqlite::Connection {
        let c = rusqlite::Connection::open_in_memory().unwrap();
        migrate(&c).unwrap();
        c
    }

    #[test]
    fn habit_toggle_inserts_then_removes() {
        let c = test_conn();
        habit_add(&c, "gym").unwrap();
        habit_toggle(&c, 1, d("2026-07-15")).unwrap();
        assert_eq!(habit_checked_on(&c, d("2026-07-15")).unwrap(), vec![1]);
        habit_toggle(&c, 1, d("2026-07-15")).unwrap();
        assert!(habit_checked_on(&c, d("2026-07-15")).unwrap().is_empty());
    }

    #[test]
    fn streak_counts_consecutive_days_and_survives_unchecked_today() {
        let c = test_conn();
        habit_add(&c, "gym").unwrap();
        for day in ["2026-07-12", "2026-07-13", "2026-07-14"] {
            habit_toggle(&c, 1, d(day)).unwrap();
        }
        // today unchecked: streak still alive from yesterday
        assert_eq!(habit_streak(&c, 1, d("2026-07-15")).unwrap(), 3);
        habit_toggle(&c, 1, d("2026-07-15")).unwrap();
        assert_eq!(habit_streak(&c, 1, d("2026-07-15")).unwrap(), 4);
        // gap breaks the streak
        assert_eq!(habit_streak(&c, 1, d("2026-07-18")).unwrap(), 0);
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test habit`
Expected: FAIL — functions not defined.

- [ ] **Step 3: Implement habit functions in `src/db.rs`**

```rust
use chrono::NaiveDate;
use rusqlite::params;

use crate::models::Habit;

pub fn habits_list(conn: &Connection) -> rusqlite::Result<Vec<Habit>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, position, archived FROM habits WHERE archived = 0 ORDER BY position",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(Habit { id: r.get(0)?, name: r.get(1)?, position: r.get(2)?, archived: r.get::<_, i64>(3)? != 0 })
    })?;
    rows.collect()
}

pub fn habit_add(conn: &Connection, name: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO habits (name, position, created_at)
         VALUES (?1, (SELECT COALESCE(MAX(position), 0) + 1 FROM habits), ?2)",
        params![name, chrono::Utc::now().to_rfc3339()],
    )?;
    Ok(())
}

pub fn habit_archive(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    conn.execute("UPDATE habits SET archived = 1 WHERE id = ?1", [id])?;
    Ok(())
}

pub fn habit_move(conn: &Connection, id: i64, delta: i64) -> rusqlite::Result<()> {
    let pos: i64 = conn.query_row("SELECT position FROM habits WHERE id = ?1", [id], |r| r.get(0))?;
    let target = pos + delta;
    let neighbor: Option<i64> = conn
        .query_row("SELECT id FROM habits WHERE position = ?1 AND archived = 0", [target], |r| r.get(0))
        .ok();
    if let Some(nid) = neighbor {
        conn.execute("UPDATE habits SET position = ?1 WHERE id = ?2", params![pos, nid])?;
        conn.execute("UPDATE habits SET position = ?1 WHERE id = ?2", params![target, id])?;
    }
    Ok(())
}

pub fn habit_toggle(conn: &Connection, id: i64, date: NaiveDate) -> rusqlite::Result<()> {
    let removed = conn.execute(
        "DELETE FROM habit_log WHERE habit_id = ?1 AND date = ?2",
        params![id, date.to_string()],
    )?;
    if removed == 0 {
        conn.execute(
            "INSERT INTO habit_log (habit_id, date) VALUES (?1, ?2)",
            params![id, date.to_string()],
        )?;
    }
    Ok(())
}

pub fn habit_checked_on(conn: &Connection, date: NaiveDate) -> rusqlite::Result<Vec<i64>> {
    let mut stmt = conn.prepare("SELECT habit_id FROM habit_log WHERE date = ?1")?;
    let rows = stmt.query_map([date.to_string()], |r| r.get(0))?;
    rows.collect()
}

pub fn habit_streak(conn: &Connection, id: i64, today: NaiveDate) -> rusqlite::Result<u32> {
    let mut stmt = conn.prepare("SELECT date FROM habit_log WHERE habit_id = ?1 ORDER BY date DESC")?;
    let dates: Vec<NaiveDate> = stmt
        .query_map([id], |r| r.get::<_, String>(0))?
        .filter_map(|s| s.ok().and_then(|s| s.parse().ok()))
        .collect();
    let mut cursor = if dates.first() == Some(&today) {
        today
    } else {
        today.pred_opt().unwrap()
    };
    let mut streak = 0u32;
    for d in dates {
        if d == cursor {
            streak += 1;
            cursor = cursor.pred_opt().unwrap();
        } else if d < cursor {
            break;
        }
    }
    Ok(streak)
}
```

- [ ] **Step 4: Run tests to verify pass**

Run: `cargo test habit`
Expected: PASS (2 tests, plus earlier tests still green).

- [ ] **Step 5: Habits UI**

`src/ui/habits.rs`:

```rust
use chrono::NaiveDate;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, Paragraph};
use ratatui::Frame;

use crate::app::{App, InputMode};
use crate::db;
use crate::models::Habit;

#[derive(Default)]
pub struct HabitsState {
    pub items: Vec<Habit>,
    pub checked: Vec<i64>,
    pub streaks: Vec<u32>,
    pub selected: usize,
    pub day: Option<NaiveDate>, // None = today; Some(d) = viewing yesterday
    pub input: Option<String>,  // Some = add-mode text buffer
}

impl HabitsState {
    pub fn load(&mut self, conn: &rusqlite::Connection, today: NaiveDate) {
        let day = self.day.unwrap_or(today);
        self.items = db::habits_list(conn).unwrap_or_default();
        self.checked = db::habit_checked_on(conn, day).unwrap_or_default();
        self.streaks = self
            .items
            .iter()
            .map(|h| db::habit_streak(conn, h.id, today).unwrap_or(0))
            .collect();
        if self.selected >= self.items.len() && !self.items.is_empty() {
            self.selected = self.items.len() - 1;
        }
    }
}

pub fn handle_key(app: &mut App, key: KeyEvent) {
    let today = app.today;
    // add-mode text entry
    if let Some(buf) = app.habits.input.as_mut() {
        match key.code {
            KeyCode::Enter => {
                let name = buf.trim().to_string();
                if !name.is_empty() {
                    let _ = db::habit_add(&app.conn, &name);
                }
                app.habits.input = None;
                app.mode = InputMode::Normal;
                app.habits.load(&app.conn, today);
            }
            KeyCode::Esc => {
                app.habits.input = None;
                app.mode = InputMode::Normal;
            }
            KeyCode::Backspace => { buf.pop(); }
            KeyCode::Char(c) => buf.push(c),
            _ => {}
        }
        return;
    }

    let n = app.habits.items.len();
    match key.code {
        KeyCode::Up | KeyCode::Char('k') if n > 0 => {
            app.habits.selected = app.habits.selected.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') if n > 0 => {
            app.habits.selected = (app.habits.selected + 1).min(n - 1);
        }
        KeyCode::Char(' ') if n > 0 => {
            let id = app.habits.items[app.habits.selected].id;
            let day = app.habits.day.unwrap_or(today);
            let _ = db::habit_toggle(&app.conn, id, day);
            app.habits.load(&app.conn, today);
        }
        KeyCode::Char('a') => {
            app.habits.input = Some(String::new());
            app.mode = InputMode::Editing;
        }
        KeyCode::Char('d') if n > 0 => {
            let _ = db::habit_archive(&app.conn, app.habits.items[app.habits.selected].id);
            app.habits.load(&app.conn, today);
        }
        KeyCode::Char('K') if n > 0 => {
            let _ = db::habit_move(&app.conn, app.habits.items[app.habits.selected].id, -1);
            app.habits.selected = app.habits.selected.saturating_sub(1);
            app.habits.load(&app.conn, today);
        }
        KeyCode::Char('J') if n > 0 => {
            let _ = db::habit_move(&app.conn, app.habits.items[app.habits.selected].id, 1);
            app.habits.selected = (app.habits.selected + 1).min(n - 1);
            app.habits.load(&app.conn, today);
        }
        // y toggles viewing yesterday (spec: yesterday editable, nothing older)
        KeyCode::Char('y') => {
            app.habits.day = match app.habits.day {
                None => Some(today.pred_opt().unwrap()),
                Some(_) => None,
            };
            app.habits.load(&app.conn, today);
        }
        _ => {}
    }
}

fn habit_lines(app: &App, show_streak: bool) -> Vec<ListItem<'static>> {
    let t = app.theme;
    app.habits
        .items
        .iter()
        .enumerate()
        .map(|(i, h)| {
            let checked = app.habits.checked.contains(&h.id);
            let mark = if checked { "✔" } else { "○" };
            let mark_style = if checked {
                Style::default().fg(t.green)
            } else {
                Style::default().fg(t.muted)
            };
            let mut name_style = Style::default().fg(t.text);
            if i == app.habits.selected {
                name_style = name_style.add_modifier(Modifier::BOLD).fg(t.accent);
            }
            let mut spans = vec![
                Span::styled(format!(" {mark} "), mark_style),
                Span::styled(h.name.clone(), name_style),
            ];
            if show_streak && app.habits.streaks.get(i).copied().unwrap_or(0) > 0 {
                spans.push(Span::styled(
                    format!("  ⚡{}", app.habits.streaks[i]),
                    Style::default().fg(t.peach),
                ));
            }
            ListItem::new(Line::from(spans))
        })
        .collect()
}

pub fn render_panel(f: &mut Frame, app: &mut App, area: Rect, focused: bool) {
    let done = app.habits.checked.len();
    let total = app.habits.items.len();
    let title = format!("HABITS {done}/{total}");
    let block = app.theme.panel_block(&title, focused);
    f.render_widget(List::new(habit_lines(app, false)).block(block), area);
}

pub fn render_zoomed(f: &mut Frame, app: &mut App) {
    let area = f.area();
    f.render_widget(
        ratatui::widgets::Block::default().style(Style::default().bg(app.theme.bg)),
        area,
    );
    let rows = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(area);
    let day_label = match app.habits.day {
        None => "today".to_string(),
        Some(d) => format!("yesterday · {d}"),
    };
    let block = app.theme.panel_block(&format!("HABITS — {day_label}"), true);
    f.render_widget(List::new(habit_lines(app, true)).block(block), rows[0]);

    let hint = if let Some(buf) = &app.habits.input {
        format!(" new habit: {buf}▏  (enter save · esc cancel)")
    } else {
        " space check · a add · d archive · J/K reorder · y yesterday · esc home ".into()
    };
    f.render_widget(Paragraph::new(hint).style(app.theme.hint()), rows[1]);
}
```

- [ ] **Step 6: Wire into app**

In `src/app.rs`: add field `pub habits: crate::ui::habits::HabitsState` to `App`, initialize with `Default::default()` in `new`, then call `app.habits.load(&conn, today)` — easiest: after constructing `App` in `new`, call `s.habits.load(&s.conn, s.today); s` before returning. In `handle_key`, after the global-key block, add:

```rust
        match self.screen {
            Screen::Habits => crate::ui::habits::handle_key(self, key),
            _ => {}
        }
```

(Editing mode: when `self.mode == InputMode::Editing`, skip the global block entirely and dispatch straight to the module — restructure `handle_key` so global keys only run in `Normal` mode. Also route panel-focused quick actions: on Home, `' '` (space) with habits panel focused calls `ui::habits::handle_key` too.)

In `tick()`, on date change also `self.habits.load(...)` (reset `day` to `None` first).

In `src/ui/mod.rs`: `pub mod habits;`, route `Screen::Habits => habits::render_zoomed(f, app)`. In `src/ui/home.rs::render_panel`, replace the `"habits"` placeholder arm with `crate::ui::habits::render_panel(f, app, area, focused)`.

- [ ] **Step 7: Verify by running**

Run: `cargo run`
Checklist: home shows habit list with ✔/○ and count in title; `1` or Enter zooms; `a` adds (typed text visible in hint bar with cursor `▏`); space toggles green ✔; streak `⚡n` shows in zoom; `y` flips to yesterday and toggling there works; `d` archives; J/K reorder; Esc home; `q` quits clean.

- [ ] **Step 8: Commit**

```bash
git add -A && git commit -m "feat: habits module — checklist, streaks, yesterday edit"
```

---

### Task 4: Recurrence engine (`recur.rs`) — pure TDD

**Files:**
- Create: `src/recur.rs`
- Modify: `src/main.rs` (add `mod recur;`)

**Interfaces:**
- Produces: `recur::Recur` enum `{ Daily, Weekly(Vec<chrono::Weekday>), EveryDays(u32) }`, `recur::parse(s: &str) -> Option<Recur>`, `recur::next_after(rule: &Recur, from: NaiveDate) -> NaiveDate`
- DSL grammar (from spec): `daily` | `weekly:mon,thu` | `every:3d`

- [ ] **Step 1: Write the failing tests** (bottom of new `src/recur.rs`, with a stub `pub fn parse(_: &str) -> Option<Recur> { None }` so it compiles)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Weekday;

    fn d(s: &str) -> chrono::NaiveDate { s.parse().unwrap() }

    #[test]
    fn parses_all_forms() {
        assert_eq!(parse("daily"), Some(Recur::Daily));
        assert_eq!(parse("weekly:mon,thu"), Some(Recur::Weekly(vec![Weekday::Mon, Weekday::Thu])));
        assert_eq!(parse("every:3d"), Some(Recur::EveryDays(3)));
    }

    #[test]
    fn rejects_garbage() {
        for bad in ["", "weekly:", "weekly:funday", "every:0d", "every:3", "monthly"] {
            assert_eq!(parse(bad), None, "should reject {bad:?}");
        }
    }

    #[test]
    fn next_after_daily_and_every() {
        assert_eq!(next_after(&Recur::Daily, d("2026-07-15")), d("2026-07-16"));
        assert_eq!(next_after(&Recur::EveryDays(3), d("2026-07-15")), d("2026-07-18"));
    }

    #[test]
    fn next_after_weekly_picks_next_listed_weekday() {
        // 2026-07-15 is a Wednesday
        let rule = Recur::Weekly(vec![Weekday::Mon, Weekday::Thu]);
        assert_eq!(next_after(&rule, d("2026-07-15")), d("2026-07-16")); // Thu
        assert_eq!(next_after(&rule, d("2026-07-16")), d("2026-07-20")); // next Mon
        // completing on a listed day moves to the NEXT occurrence, not the same day
        assert_eq!(next_after(&rule, d("2026-07-20")), d("2026-07-23"));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test recur`
Expected: FAIL (parse stub returns None; next_after missing → add stub or watch compile error, then implement).

- [ ] **Step 3: Implement**

```rust
use chrono::{Datelike, Duration, NaiveDate, Weekday};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Recur {
    Daily,
    Weekly(Vec<Weekday>),
    EveryDays(u32),
}

pub fn parse(s: &str) -> Option<Recur> {
    if s == "daily" {
        return Some(Recur::Daily);
    }
    if let Some(days) = s.strip_prefix("weekly:") {
        if days.is_empty() { return None; }
        let wd: Option<Vec<Weekday>> = days.split(',').map(weekday).collect();
        return wd.map(Recur::Weekly);
    }
    if let Some(n) = s.strip_prefix("every:") {
        let n: u32 = n.strip_suffix('d')?.parse().ok()?;
        if n > 0 { return Some(Recur::EveryDays(n)); }
    }
    None
}

fn weekday(s: &str) -> Option<Weekday> {
    Some(match s {
        "mon" => Weekday::Mon, "tue" => Weekday::Tue, "wed" => Weekday::Wed,
        "thu" => Weekday::Thu, "fri" => Weekday::Fri, "sat" => Weekday::Sat,
        "sun" => Weekday::Sun,
        _ => return None,
    })
}

/// Next occurrence strictly after `from`. `from` is the completion date (today),
/// per spec: nothing pre-materialized, no backlog of missed occurrences.
pub fn next_after(rule: &Recur, from: NaiveDate) -> NaiveDate {
    match rule {
        Recur::Daily => from + Duration::days(1),
        Recur::EveryDays(n) => from + Duration::days(*n as i64),
        Recur::Weekly(days) => {
            let mut d = from + Duration::days(1);
            while !days.contains(&d.weekday()) {
                d += Duration::days(1);
            }
            d
        }
    }
}
```

- [ ] **Step 4: Run tests to verify pass**

Run: `cargo test recur`
Expected: PASS (4 tests).

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat: recurrence DSL — daily / weekly:days / every:Nd"
```

---

### Task 5: Todos data layer

**Files:**
- Modify: `src/db.rs` (todo functions + tests)

**Interfaces:**
- Consumes: `recur::parse`, `recur::next_after`, `models::Todo`.
- Produces in `db.rs`:
  - `pub struct NewTodo { pub title: String, pub notes: String, pub priority: u8, pub due_date: Option<NaiveDate>, pub project: Option<String>, pub tags: String, pub parent_id: Option<i64>, pub recur_rule: Option<String> }`
  - `todo_add(conn, &NewTodo) -> rusqlite::Result<i64>` (returns new id)
  - `todo_update(conn, id: i64, &NewTodo) -> rusqlite::Result<()>`
  - `todos_open(conn) -> rusqlite::Result<Vec<Todo>>` (done_at IS NULL, top-level only, ordered: overdue first, then due date asc nulls last, then priority desc)
  - `subtasks_of(conn, parent_id: i64) -> rusqlite::Result<Vec<Todo>>`
  - `todo_complete(conn, id: i64, today: NaiveDate) -> rusqlite::Result<()>` — stamps `done_at`; if `recur_rule` parses, inserts the next occurrence (same fields, `due_date = next_after(rule, today)`)
  - `todo_uncomplete(conn, id: i64) -> rusqlite::Result<()>`
  - `todo_delete(conn, id: i64) -> rusqlite::Result<()>` (cascade removes subtasks)
  - `open_subtask_count(conn, parent_id: i64) -> rusqlite::Result<(i64, i64)>` (open, total)

- [ ] **Step 1: Write failing tests** (in `src/db.rs` tests mod)

```rust
    fn new_todo(title: &str) -> NewTodo {
        NewTodo {
            title: title.into(), notes: String::new(), priority: 0,
            due_date: None, project: None, tags: String::new(),
            parent_id: None, recur_rule: None,
        }
    }

    #[test]
    fn todo_complete_recurring_spawns_next_occurrence() {
        let c = test_conn();
        let mut t = new_todo("water plants");
        t.recur_rule = Some("every:3d".into());
        t.due_date = Some(d("2026-07-15"));
        let id = todo_add(&c, &t).unwrap();
        todo_complete(&c, id, d("2026-07-15")).unwrap();

        let open = todos_open(&c).unwrap();
        assert_eq!(open.len(), 1, "next occurrence should exist");
        assert_eq!(open[0].due_date, Some(d("2026-07-18")));
        assert_eq!(open[0].recur_rule.as_deref(), Some("every:3d"));
        assert!(open[0].id != id);
    }

    #[test]
    fn todo_complete_non_recurring_just_closes() {
        let c = test_conn();
        let id = todo_add(&c, &new_todo("one-off")).unwrap();
        todo_complete(&c, id, d("2026-07-15")).unwrap();
        assert!(todos_open(&c).unwrap().is_empty());
        todo_uncomplete(&c, id).unwrap();
        assert_eq!(todos_open(&c).unwrap().len(), 1);
    }

    #[test]
    fn todos_open_orders_overdue_then_due_then_priority() {
        let c = test_conn();
        let mut a = new_todo("low no-due"); a.priority = 0;
        let mut b = new_todo("high no-due"); b.priority = 2;
        let mut o = new_todo("overdue"); o.due_date = Some(d("2026-07-01"));
        todo_add(&c, &a).unwrap();
        todo_add(&c, &b).unwrap();
        todo_add(&c, &o).unwrap();
        let titles: Vec<_> = todos_open(&c).unwrap().into_iter().map(|t| t.title).collect();
        assert_eq!(titles, vec!["overdue", "high no-due", "low no-due"]);
    }

    #[test]
    fn subtasks_cascade_and_count() {
        let c = test_conn();
        let parent = todo_add(&c, &new_todo("parent")).unwrap();
        let mut sub = new_todo("child");
        sub.parent_id = Some(parent);
        let sid = todo_add(&c, &sub).unwrap();
        assert_eq!(open_subtask_count(&c, parent).unwrap(), (1, 1));
        todo_complete(&c, sid, d("2026-07-15")).unwrap();
        assert_eq!(open_subtask_count(&c, parent).unwrap(), (0, 1));
        todo_delete(&c, parent).unwrap();
        assert!(subtasks_of(&c, parent).unwrap().is_empty());
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test todo`
Expected: FAIL — functions not defined.

- [ ] **Step 3: Implement in `src/db.rs`**

```rust
use crate::models::Todo;
use crate::recur;

pub struct NewTodo {
    pub title: String,
    pub notes: String,
    pub priority: u8,
    pub due_date: Option<NaiveDate>,
    pub project: Option<String>,
    pub tags: String,
    pub parent_id: Option<i64>,
    pub recur_rule: Option<String>,
}

fn row_to_todo(r: &rusqlite::Row) -> rusqlite::Result<Todo> {
    Ok(Todo {
        id: r.get(0)?,
        title: r.get(1)?,
        notes: r.get(2)?,
        priority: r.get::<_, i64>(3)? as u8,
        due_date: r.get::<_, Option<String>>(4)?.and_then(|s| s.parse().ok()),
        project: r.get(5)?,
        tags: r.get(6)?,
        parent_id: r.get(7)?,
        recur_rule: r.get(8)?,
        done_at: r.get(9)?,
        created_at: r.get(10)?,
    })
}

const TODO_COLS: &str =
    "id, title, notes, priority, due_date, project, tags, parent_id, recur_rule, done_at, created_at";

pub fn todo_add(conn: &Connection, t: &NewTodo) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO todos (title, notes, priority, due_date, project, tags, parent_id, recur_rule, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            t.title, t.notes, t.priority as i64,
            t.due_date.map(|d| d.to_string()), t.project, t.tags,
            t.parent_id, t.recur_rule, chrono::Utc::now().to_rfc3339()
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn todo_update(conn: &Connection, id: i64, t: &NewTodo) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE todos SET title=?1, notes=?2, priority=?3, due_date=?4, project=?5, tags=?6, recur_rule=?7
         WHERE id=?8",
        params![
            t.title, t.notes, t.priority as i64,
            t.due_date.map(|d| d.to_string()), t.project, t.tags, t.recur_rule, id
        ],
    )?;
    Ok(())
}

pub fn todos_open(conn: &Connection) -> rusqlite::Result<Vec<Todo>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {TODO_COLS} FROM todos
         WHERE done_at IS NULL AND parent_id IS NULL
         ORDER BY due_date IS NULL, due_date, priority DESC, created_at"
    ))?;
    let rows = stmt.query_map([], |r| row_to_todo(r))?;
    rows.collect()
}

pub fn subtasks_of(conn: &Connection, parent_id: i64) -> rusqlite::Result<Vec<Todo>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {TODO_COLS} FROM todos WHERE parent_id = ?1 ORDER BY done_at IS NOT NULL, created_at"
    ))?;
    let rows = stmt.query_map([parent_id], |r| row_to_todo(r))?;
    rows.collect()
}

pub fn open_subtask_count(conn: &Connection, parent_id: i64) -> rusqlite::Result<(i64, i64)> {
    conn.query_row(
        "SELECT COALESCE(SUM(done_at IS NULL), 0), COUNT(*) FROM todos WHERE parent_id = ?1",
        [parent_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
}

pub fn todo_complete(conn: &Connection, id: i64, today: NaiveDate) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE todos SET done_at = ?1 WHERE id = ?2",
        params![chrono::Utc::now().to_rfc3339(), id],
    )?;
    let mut stmt = conn.prepare(&format!("SELECT {TODO_COLS} FROM todos WHERE id = ?1"))?;
    let t = stmt.query_row([id], |r| row_to_todo(r))?;
    if let Some(rule) = t.recur_rule.as_deref().and_then(recur::parse) {
        let next = NewTodo {
            title: t.title, notes: t.notes, priority: t.priority,
            due_date: Some(recur::next_after(&rule, today)),
            project: t.project, tags: t.tags, parent_id: None,
            recur_rule: t.recur_rule.clone(),
        };
        todo_add(conn, &next)?;
    }
    Ok(())
}

pub fn todo_uncomplete(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    conn.execute("UPDATE todos SET done_at = NULL WHERE id = ?1", [id])?;
    Ok(())
}

pub fn todo_delete(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM todos WHERE id = ?1", [id])?;
    Ok(())
}
```

Note on ordering test: `overdue` sorts first because it has the earliest due date; items without due dates sort last (`due_date IS NULL` ascending puts non-null first), then priority breaks the tie among no-due items.

- [ ] **Step 4: Run tests to verify pass**

Run: `cargo test`
Expected: PASS — all tests including the 4 new ones.

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat: todos data layer — crud, subtasks, recurrence on complete"
```

---

### Task 6: Todos UI

**Files:**
- Create: `src/ui/todos.rs`
- Modify: `src/app.rs`, `src/ui/mod.rs`, `src/ui/home.rs`

**Interfaces:**
- Consumes: everything from Task 5, `HabitsState` wiring pattern from Task 3.
- Produces: `TodosState { items: Vec<Row>, selected: usize, group_by_project: bool, filter: Option<String>, form: Option<TodoForm>, expanded: Option<i64> }` where `Row { todo: Todo, is_subtask: bool, sub_counts: Option<(i64, i64)> }`; `TodoForm { fields: [String; 7], focus: usize, editing_id: Option<i64>, parent_id: Option<i64> }` (fields: title, notes, priority, due `YYYY-MM-DD`, project, tags, recur rule); `TodosState::load(&mut self, conn)`, `pub fn handle_key(app, key)`, `pub fn render_panel(f, app, area, focused)`, `pub fn render_zoomed(f, app)`
- App gains `pub todos: TodosState`. Todos zoom keys: `a` add, `A` add subtask under selected, `e` edit, `space`/`x` complete (with recurring `↻` feedback via status line), `u` un-complete last, `d` delete, `enter` expand/collapse subtasks, `/` filter (Editing mode, live), `g` toggle group-by-project, `p` reserved for Task 9 (pomodoro link).

- [ ] **Step 1: Implement `TodosState::load`** — flatten: for each open top-level todo (filtered by `filter` on title/project/tags if set), push its Row with `sub_counts = open_subtask_count(...)`; if `expanded == Some(todo.id)`, push each subtask Row after it. When `group_by_project`, sort top-level todos by `project.clone().unwrap_or_default()` first (stable — keeps due-date order within groups).

```rust
use chrono::NaiveDate;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::app::{App, InputMode};
use crate::db::{self, NewTodo};
use crate::models::Todo;
use crate::recur;

pub struct Row {
    pub todo: Todo,
    pub is_subtask: bool,
    pub sub_counts: Option<(i64, i64)>,
}

pub struct TodoForm {
    pub fields: [String; 7], // title, notes, priority(0-2), due, project, tags, recur
    pub focus: usize,
    pub editing_id: Option<i64>,
    pub parent_id: Option<i64>,
}

pub const FIELD_LABELS: [&str; 7] =
    ["title", "notes", "priority", "due (YYYY-MM-DD)", "project", "tags", "repeat"];

#[derive(Default)]
pub struct TodosState {
    pub items: Vec<Row>,
    pub selected: usize,
    pub group_by_project: bool,
    pub filter: Option<String>,
    pub filter_editing: bool,
    pub form: Option<TodoForm>,
    pub expanded: Option<i64>,
    pub last_completed: Option<i64>,
}

impl TodosState {
    pub fn load(&mut self, conn: &rusqlite::Connection) {
        let mut tops = db::todos_open(conn).unwrap_or_default();
        if let Some(f) = self.filter.as_deref() {
            let f = f.to_lowercase();
            tops.retain(|t| {
                t.title.to_lowercase().contains(&f)
                    || t.project.as_deref().unwrap_or("").to_lowercase().contains(&f)
                    || t.tags.to_lowercase().contains(&f)
            });
        }
        if self.group_by_project {
            tops.sort_by_key(|t| t.project.clone().unwrap_or_default());
        }
        self.items = Vec::new();
        for t in tops {
            let id = t.id;
            self.items.push(Row {
                sub_counts: db::open_subtask_count(conn, id).ok().filter(|c| c.1 > 0),
                todo: t,
                is_subtask: false,
            });
            if self.expanded == Some(id) {
                for s in db::subtasks_of(conn, id).unwrap_or_default() {
                    self.items.push(Row { todo: s, is_subtask: true, sub_counts: None });
                }
            }
        }
        if self.selected >= self.items.len() && !self.items.is_empty() {
            self.selected = self.items.len() - 1;
        }
    }
}
```

- [ ] **Step 2: Key handling** (same file)

```rust
pub fn handle_key(app: &mut App, key: KeyEvent) {
    // ---- form entry ----
    if app.todos.form.is_some() {
        form_key(app, key);
        return;
    }
    // ---- live filter entry ----
    if app.todos.filter_editing {
        match key.code {
            KeyCode::Enter | KeyCode::Esc => {
                if key.code == KeyCode::Esc { app.todos.filter = None; }
                app.todos.filter_editing = false;
                app.mode = InputMode::Normal;
            }
            KeyCode::Backspace => { app.todos.filter.get_or_insert_default().pop(); }
            KeyCode::Char(c) => app.todos.filter.get_or_insert_default().push(c),
            _ => {}
        }
        app.todos.load(&app.conn);
        return;
    }

    let n = app.todos.items.len();
    match key.code {
        KeyCode::Up | KeyCode::Char('k') if n > 0 => {
            app.todos.selected = app.todos.selected.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') if n > 0 => {
            app.todos.selected = (app.todos.selected + 1).min(n - 1);
        }
        KeyCode::Enter if n > 0 => {
            let row = &app.todos.items[app.todos.selected];
            if !row.is_subtask {
                let id = row.todo.id;
                app.todos.expanded = if app.todos.expanded == Some(id) { None } else { Some(id) };
                app.todos.load(&app.conn);
            }
        }
        KeyCode::Char(' ') | KeyCode::Char('x') if n > 0 => {
            let row = &app.todos.items[app.todos.selected];
            let (id, recurring, parent) =
                (row.todo.id, row.todo.recur_rule.is_some(), row.todo.parent_id);
            let _ = db::todo_complete(&app.conn, id, app.today);
            app.todos.last_completed = Some(id);
            if recurring {
                app.status = Some("↻ next occurrence scheduled".into());
            }
            // completing the last open subtask offers the parent via status line
            if let Some(pid) = parent {
                if let Ok((0, _)) = db::open_subtask_count(&app.conn, pid) {
                    app.status = Some("all subtasks done — space on parent to close it".into());
                }
            }
            app.todos.load(&app.conn);
        }
        KeyCode::Char('u') => {
            if let Some(id) = app.todos.last_completed.take() {
                let _ = db::todo_uncomplete(&app.conn, id);
                app.todos.load(&app.conn);
            }
        }
        KeyCode::Char('d') if n > 0 => {
            let _ = db::todo_delete(&app.conn, app.todos.items[app.todos.selected].todo.id);
            app.todos.load(&app.conn);
        }
        KeyCode::Char('a') => open_form(app, None, None),
        KeyCode::Char('A') if n > 0 => {
            let row = &app.todos.items[app.todos.selected];
            let pid = row.todo.parent_id.unwrap_or(row.todo.id); // one level max
            open_form(app, None, Some(pid));
        }
        KeyCode::Char('e') if n > 0 => {
            let t = app.todos.items[app.todos.selected].todo.clone();
            open_form(app, Some(t), None);
        }
        KeyCode::Char('/') => {
            app.todos.filter = Some(String::new());
            app.todos.filter_editing = true;
            app.mode = InputMode::Editing;
        }
        KeyCode::Char('g') => {
            app.todos.group_by_project = !app.todos.group_by_project;
            app.todos.load(&app.conn);
        }
        _ => {}
    }
}

fn open_form(app: &mut App, edit: Option<Todo>, parent_id: Option<i64>) {
    let form = match edit {
        Some(t) => TodoForm {
            fields: [
                t.title.clone(), t.notes.clone(), t.priority.to_string(),
                t.due_date.map(|d| d.to_string()).unwrap_or_default(),
                t.project.clone().unwrap_or_default(), t.tags.clone(),
                t.recur_rule.clone().unwrap_or_default(),
            ],
            focus: 0,
            editing_id: Some(t.id),
            parent_id: t.parent_id,
        },
        None => TodoForm {
            fields: Default::default(),
            focus: 0,
            editing_id: None,
            parent_id,
        },
    };
    app.todos.form = Some(form);
    app.mode = InputMode::Editing;
}

fn form_key(app: &mut App, key: KeyEvent) {
    let form = app.todos.form.as_mut().unwrap();
    match key.code {
        KeyCode::Esc => {
            app.todos.form = None;
            app.mode = InputMode::Normal;
        }
        KeyCode::Tab | KeyCode::Down => form.focus = (form.focus + 1) % 7,
        KeyCode::BackTab | KeyCode::Up => form.focus = (form.focus + 6) % 7,
        KeyCode::Left | KeyCode::Right if form.focus == 2 => {
            // priority cycles 0→1→2
            let p: u8 = form.fields[2].parse().unwrap_or(0);
            let p = if key.code == KeyCode::Right { (p + 1) % 3 } else { (p + 2) % 3 };
            form.fields[2] = p.to_string();
        }
        KeyCode::Backspace => { form.fields[form.focus].pop(); }
        KeyCode::Enter => {
            // validate + save
            let title = form.fields[0].trim().to_string();
            if title.is_empty() {
                app.status = Some("title is required".into());
                return;
            }
            let due = match form.fields[3].trim() {
                "" => None,
                s => match s.parse::<NaiveDate>() {
                    Ok(d) => Some(d),
                    Err(_) => { app.status = Some("due must be YYYY-MM-DD".into()); return; }
                },
            };
            let rule = match form.fields[6].trim() {
                "" => None,
                s => {
                    if recur::parse(s).is_none() {
                        app.status = Some("repeat: daily | weekly:mon,thu | every:3d".into());
                        return;
                    }
                    Some(s.to_string())
                }
            };
            let nt = NewTodo {
                title,
                notes: form.fields[1].clone(),
                priority: form.fields[2].parse().unwrap_or(0),
                due_date: due,
                project: match form.fields[4].trim() { "" => None, s => Some(s.into()) },
                tags: form.fields[5].trim().to_string(),
                parent_id: form.parent_id,
                recur_rule: rule,
            };
            let res = match form.editing_id {
                Some(id) => db::todo_update(&app.conn, id, &nt),
                None => db::todo_add(&app.conn, &nt).map(|_| ()),
            };
            if let Err(e) = res {
                app.status = Some(format!("save failed: {e}"));
            }
            app.todos.form = None;
            app.mode = InputMode::Normal;
            app.todos.load(&app.conn);
        }
        KeyCode::Char(c) if form.focus != 2 => form.fields[form.focus].push(c),
        _ => {}
    }
}
```

- [ ] **Step 3: Rendering** (same file)

```rust
fn todo_line(app: &App, row: &Row, selected: bool) -> ListItem<'static> {
    let t = app.theme;
    let td = &row.todo;
    let done = td.done_at.is_some();
    let overdue = !done && td.due_date.map(|d| d < app.today).unwrap_or(false);

    let mut spans: Vec<Span> = Vec::new();
    if row.is_subtask {
        spans.push(Span::raw("    "));
    }
    let mark = if done { "✔" } else if td.priority == 2 { "◉" } else { "○" };
    let mark_color = if done { t.green } else if overdue { t.red }
        else if td.priority == 2 { t.red } else if td.priority == 1 { t.yellow } else { t.muted };
    spans.push(Span::styled(format!(" {mark} "), Style::default().fg(mark_color)));

    let mut title_style = Style::default().fg(if overdue { t.red } else { t.text });
    if done { title_style = title_style.fg(t.muted).add_modifier(Modifier::CROSSED_OUT); }
    if selected { title_style = title_style.add_modifier(Modifier::BOLD | Modifier::REVERSED); }
    spans.push(Span::styled(td.title.clone(), title_style));

    if td.recur_rule.is_some() {
        spans.push(Span::styled(" ↻", Style::default().fg(t.blue)));
    }
    if let Some((open, total)) = row.sub_counts {
        spans.push(Span::styled(
            format!(" [{}/{}]", total - open, total),
            Style::default().fg(t.muted),
        ));
    }
    if let Some(d) = td.due_date {
        let c = if overdue { t.red } else { t.peach };
        spans.push(Span::styled(format!("  {d}"), Style::default().fg(c)));
    }
    if let Some(p) = &td.project {
        spans.push(Span::styled(format!("  #{p}"), Style::default().fg(t.blue)));
    }
    ListItem::new(Line::from(spans))
}

pub fn render_panel(f: &mut Frame, app: &mut App, area: Rect, focused: bool) {
    let block = app.theme.panel_block(&format!("TODOS ({})", app.todos.items.len()), focused);
    let items: Vec<ListItem> = app.todos.items.iter().enumerate()
        .map(|(i, r)| todo_line(app, r, focused && i == app.todos.selected))
        .collect();
    f.render_widget(List::new(items).block(block), area);
}

pub fn render_zoomed(f: &mut Frame, app: &mut App) {
    let area = f.area();
    f.render_widget(
        ratatui::widgets::Block::default().style(Style::default().bg(app.theme.bg)), area);
    let rows = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(area);

    let mut title = String::from("TODOS");
    if let Some(fil) = &app.todos.filter { title = format!("TODOS · filter: {fil}"); }
    if app.todos.group_by_project { title.push_str(" · by project"); }
    let block = app.theme.panel_block(&title, true);
    let items: Vec<ListItem> = app.todos.items.iter().enumerate()
        .map(|(i, r)| todo_line(app, r, i == app.todos.selected))
        .collect();
    f.render_widget(List::new(items).block(block), rows[0]);

    let hint = if app.todos.filter_editing {
        format!(" filter: {}▏  (enter apply · esc clear)", app.todos.filter.as_deref().unwrap_or(""))
    } else {
        " a add · A subtask · e edit · space done · u undo · d delete · enter expand · / filter · g group · p pomodoro · esc home ".into()
    };
    f.render_widget(Paragraph::new(hint).style(app.theme.hint()), rows[1]);

    if app.todos.form.is_some() {
        render_form(f, app, area);
    }
}

fn render_form(f: &mut Frame, app: &mut App, screen: Rect) {
    let form = app.todos.form.as_ref().unwrap();
    let t = app.theme;
    let w = 60.min(screen.width.saturating_sub(4));
    let h = 11u16;
    let popup = Rect {
        x: screen.x + (screen.width.saturating_sub(w)) / 2,
        y: screen.y + (screen.height.saturating_sub(h)) / 2,
        width: w, height: h,
    };
    f.render_widget(Clear, popup);
    let title = if form.editing_id.is_some() { "EDIT TODO" }
        else if form.parent_id.is_some() { "NEW SUBTASK" } else { "NEW TODO" };
    let block = t.panel_block(title, true)
        .style(Style::default().bg(t.surface));
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let lines: Vec<Line> = FIELD_LABELS.iter().enumerate().map(|(i, label)| {
        let focused = i == form.focus;
        let val = if i == 2 {
            ["low", "med", "high"][form.fields[2].parse::<usize>().unwrap_or(0).min(2)].to_string()
        } else {
            form.fields[i].clone()
        };
        let cursor = if focused && i != 2 { "▏" } else { "" };
        Line::from(vec![
            Span::styled(format!(" {label:<18}"), Style::default().fg(if focused { t.accent } else { t.muted })),
            Span::styled(format!("{val}{cursor}"), Style::default().fg(t.text)),
        ])
    }).collect();
    let mut all = lines;
    all.push(Line::from(Span::styled(
        "  tab next · ←/→ priority · enter save · esc cancel",
        Style::default().fg(t.muted),
    )));
    f.render_widget(Paragraph::new(all), inner);
}
```

- [ ] **Step 4: Wire into app** — mirror Task 3 Step 6: `pub todos: TodosState` on `App` (load in `new`), `Screen::Todos => todos::handle_key/render_zoomed`, home `"todos"` arm → `todos::render_panel`. IMPORTANT (from Task 3): `App::handle_key` must dispatch to the module FIRST when `mode == Editing`, before any global key handling, so typing `q`/digits into forms works.

- [ ] **Step 5: Run tests + run app**

Run: `cargo test`
Expected: PASS.

Run: `cargo run`
Checklist: `2` zooms todos; `a` opens centered form; fill title/due/priority (←/→ cycles low/med/high)/repeat `daily`; invalid due or repeat shows red-flag status message and does not save; list shows priority glyphs, due dates (overdue red), `#project`, `↻`; `A` adds subtask, Enter expands showing indented subtasks with `[done/total]` on parent; completing last subtask shows parent nudge; `/` filters live; `g` groups by project; `u` restores last completed.

- [ ] **Step 6: Commit**

```bash
git add -A && git commit -m "feat: todos UI — form overlay, subtasks, filter, recurrence"
```

---

### Task 7: Calendar module

**Files:**
- Modify: `src/db.rs` (event + digest functions + tests), `src/app.rs`, `src/ui/mod.rs`, `src/ui/home.rs`
- Create: `src/ui/calendar.rs`

**Interfaces:**
- Consumes: `models::Event`, todos functions from Task 5, habits from Task 3.
- Produces in `db.rs`:
  - `event_add(conn, title: &str, date: NaiveDate, time: Option<&str>, category: &str, color: &str) -> rusqlite::Result<()>`
  - `event_delete(conn, id: i64) -> rusqlite::Result<()>`
  - `events_between(conn, start: NaiveDate, end: NaiveDate) -> rusqlite::Result<Vec<Event>>` (inclusive)
  - `todos_due_between(conn, start: NaiveDate, end: NaiveDate) -> rusqlite::Result<Vec<Todo>>` (open only, inclusive)
- Produces in `ui/calendar.rs`: `CalendarState { cursor: NaiveDate, events: Vec<Event>, due: Vec<Todo>, form: Option<EventForm> }`, `EventForm { fields: [String; 3], focus: usize }` (title, time `HH:MM` optional, category), `load`, `handle_key`, `render_panel`, `render_zoomed`
- Event category → dot color mapping lives in `ui/calendar.rs`: `fn category_color(theme, category) -> Color` — `work→blue, personal→green, health→peach, deadline→red, _→accent`.

- [ ] **Step 1: Write failing tests** (in `src/db.rs` tests mod)

```rust
    #[test]
    fn events_between_is_inclusive_and_sorted() {
        let c = test_conn();
        event_add(&c, "b", d("2026-07-20"), None, "work", "blue").unwrap();
        event_add(&c, "a", d("2026-07-15"), Some("09:00"), "health", "peach").unwrap();
        event_add(&c, "outside", d("2026-08-01"), None, "work", "blue").unwrap();
        let ev = events_between(&c, d("2026-07-15"), d("2026-07-31")).unwrap();
        assert_eq!(ev.iter().map(|e| e.title.as_str()).collect::<Vec<_>>(), vec!["a", "b"]);
    }

    #[test]
    fn todos_due_between_only_open() {
        let c = test_conn();
        let mut t = new_todo("due in range"); t.due_date = Some(d("2026-07-16"));
        let id = todo_add(&c, &t).unwrap();
        let mut t2 = new_todo("done in range"); t2.due_date = Some(d("2026-07-17"));
        let id2 = todo_add(&c, &t2).unwrap();
        todo_complete(&c, id2, d("2026-07-15")).unwrap();
        let due = todos_due_between(&c, d("2026-07-15"), d("2026-07-21")).unwrap();
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].id, id);
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test between`
Expected: FAIL — functions not defined.

- [ ] **Step 3: Implement db functions**

```rust
use crate::models::Event;

pub fn event_add(
    conn: &Connection, title: &str, date: NaiveDate,
    time: Option<&str>, category: &str, color: &str,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO events (title, date, time, category, color) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![title, date.to_string(), time, category, color],
    )?;
    Ok(())
}

pub fn event_delete(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM events WHERE id = ?1", [id])?;
    Ok(())
}

pub fn events_between(conn: &Connection, start: NaiveDate, end: NaiveDate) -> rusqlite::Result<Vec<Event>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, date, time, category, color, notes FROM events
         WHERE date >= ?1 AND date <= ?2 ORDER BY date, time",
    )?;
    let rows = stmt.query_map(params![start.to_string(), end.to_string()], |r| {
        Ok(Event {
            id: r.get(0)?, title: r.get(1)?,
            date: r.get::<_, String>(2)?.parse().unwrap(),
            time: r.get(3)?, category: r.get(4)?, color: r.get(5)?, notes: r.get(6)?,
        })
    })?;
    rows.collect()
}

pub fn todos_due_between(conn: &Connection, start: NaiveDate, end: NaiveDate) -> rusqlite::Result<Vec<Todo>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {TODO_COLS} FROM todos
         WHERE done_at IS NULL AND due_date >= ?1 AND due_date <= ?2 ORDER BY due_date"
    ))?;
    let rows = stmt.query_map(params![start.to_string(), end.to_string()], |r| row_to_todo(r))?;
    rows.collect()
}
```

- [ ] **Step 4: Run tests to verify pass**

Run: `cargo test`
Expected: PASS.

- [ ] **Step 5: Calendar UI** — `src/ui/calendar.rs`. `CalendarState::load` fetches events + due todos for the cursor's whole visible month plus 7 days after month-end (for the digest). Month grid: Mon-first columns; each day cell 4 chars wide showing day number styled (today = accent bold, cursor = reversed, other month = muted) with up to 3 event dots `•` colored by `category_color` beneath-inline. Agenda pane (right 40%): selected-day section (events with times, due todos) then "NEXT 7 DAYS" digest (per day: `Wed 16 — 2 events · 1 due · habits 3/4` — habit counts via `db::habit_checked_on` + `db::habits_list`). Keys: `h/l/←/→` day, `j/k/↓/↑` week, `[`/`]` month jump, `a` add-event form (title/time/category — 3-field popup reusing the Task 6 popup pattern: `Clear` + centered `Rect` + surface bg), `d` delete first event on cursor day, `t` jump to today.

```rust
use chrono::{Datelike, Duration, NaiveDate, Weekday};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph};
use ratatui::Frame;

use crate::app::{App, InputMode};
use crate::db;
use crate::models::{Event, Todo};

pub struct EventForm {
    pub fields: [String; 3], // title, time HH:MM (optional), category
    pub focus: usize,
}

pub struct CalendarState {
    pub cursor: NaiveDate,
    pub events: Vec<Event>,
    pub due: Vec<Todo>,
    pub form: Option<EventForm>,
}

impl Default for CalendarState {
    fn default() -> Self {
        Self {
            cursor: chrono::Local::now().date_naive(),
            events: Vec::new(), due: Vec::new(), form: None,
        }
    }
}

fn month_start(d: NaiveDate) -> NaiveDate { d.with_day(1).unwrap() }
fn month_end(d: NaiveDate) -> NaiveDate {
    let next = if d.month() == 12 {
        NaiveDate::from_ymd_opt(d.year() + 1, 1, 1).unwrap()
    } else {
        NaiveDate::from_ymd_opt(d.year(), d.month() + 1, 1).unwrap()
    };
    next.pred_opt().unwrap()
}

pub fn category_color(t: &crate::theme::Theme, category: &str) -> Color {
    match category {
        "work" => t.blue, "personal" => t.green,
        "health" => t.peach, "deadline" => t.red,
        _ => t.accent,
    }
}

impl CalendarState {
    pub fn load(&mut self, conn: &rusqlite::Connection) {
        let start = month_start(self.cursor);
        let end = month_end(self.cursor) + Duration::days(7);
        self.events = db::events_between(conn, start, end).unwrap_or_default();
        self.due = db::todos_due_between(conn, start, end).unwrap_or_default();
    }
    fn events_on(&self, d: NaiveDate) -> Vec<&Event> {
        self.events.iter().filter(|e| e.date == d).collect()
    }
    fn due_on(&self, d: NaiveDate) -> Vec<&Todo> {
        self.due.iter().filter(|t| t.due_date == Some(d)).collect()
    }
}

pub fn handle_key(app: &mut App, key: KeyEvent) {
    if app.calendar.form.is_some() {
        form_key(app, key);
        return;
    }
    let c = app.calendar.cursor;
    let mut moved = None;
    match key.code {
        KeyCode::Left | KeyCode::Char('h') => moved = Some(c - Duration::days(1)),
        KeyCode::Right | KeyCode::Char('l') => moved = Some(c + Duration::days(1)),
        KeyCode::Up | KeyCode::Char('k') => moved = Some(c - Duration::days(7)),
        KeyCode::Down | KeyCode::Char('j') => moved = Some(c + Duration::days(7)),
        KeyCode::Char('[') => moved = Some(month_start(c).pred_opt().unwrap()),
        KeyCode::Char(']') => moved = Some(month_end(c) + Duration::days(1)),
        KeyCode::Char('t') => moved = Some(app.today),
        KeyCode::Char('a') => {
            app.calendar.form = Some(EventForm { fields: Default::default(), focus: 0 });
            app.mode = InputMode::Editing;
        }
        KeyCode::Char('d') => {
            if let Some(e) = app.calendar.events_on(c).first() {
                let _ = db::event_delete(&app.conn, e.id);
                app.calendar.load(&app.conn);
            }
        }
        _ => {}
    }
    if let Some(d) = moved {
        let month_changed = d.month() != c.month() || d.year() != c.year();
        app.calendar.cursor = d;
        if month_changed { app.calendar.load(&app.conn); }
    }
}

fn form_key(app: &mut App, key: KeyEvent) {
    let form = app.calendar.form.as_mut().unwrap();
    match key.code {
        KeyCode::Esc => { app.calendar.form = None; app.mode = InputMode::Normal; }
        KeyCode::Tab | KeyCode::Down => form.focus = (form.focus + 1) % 3,
        KeyCode::BackTab | KeyCode::Up => form.focus = (form.focus + 2) % 3,
        KeyCode::Backspace => { form.fields[form.focus].pop(); }
        KeyCode::Enter => {
            let title = form.fields[0].trim().to_string();
            if title.is_empty() { app.status = Some("title is required".into()); return; }
            let time = form.fields[1].trim();
            if !time.is_empty() && chrono::NaiveTime::parse_from_str(time, "%H:%M").is_err() {
                app.status = Some("time must be HH:MM".into());
                return;
            }
            let cat = match form.fields[2].trim() { "" => "general", s => s };
            let _ = db::event_add(
                &app.conn, &title, app.calendar.cursor,
                if time.is_empty() { None } else { Some(time) },
                cat, "themed",
            );
            app.calendar.form = None;
            app.mode = InputMode::Normal;
            app.calendar.load(&app.conn);
        }
        KeyCode::Char(c) => form.fields[form.focus].push(c),
        _ => {}
    }
}

fn month_grid_lines(app: &App, compact: bool) -> Vec<Line<'static>> {
    let t = app.theme;
    let cur = app.calendar.cursor;
    let start = month_start(cur);
    let mut lines = vec![Line::from(Span::styled(
        " Mo  Tu  We  Th  Fr  Sa  Su",
        Style::default().fg(t.muted).add_modifier(Modifier::BOLD),
    ))];
    let lead = start.weekday().num_days_from_monday() as i64;
    let mut day = start - Duration::days(lead);
    let end = month_end(cur);
    while day <= end {
        let mut num_spans: Vec<Span> = Vec::new();
        let mut dot_spans: Vec<Span> = Vec::new();
        for _ in 0..7 {
            let in_month = day.month() == cur.month();
            let mut style = Style::default().fg(if in_month { t.text } else { t.muted });
            if day == app.today { style = style.fg(t.accent).add_modifier(Modifier::BOLD); }
            if day == cur { style = style.add_modifier(Modifier::REVERSED); }
            num_spans.push(Span::styled(format!(" {:>2} ", day.day()), style));

            let evs = app.calendar.events_on(day);
            let due_n = app.calendar.due_on(day).len();
            let mut dots = String::new();
            let mut dot_line: Vec<Span> = Vec::new();
            for e in evs.iter().take(3) {
                dot_line.push(Span::styled("•", Style::default().fg(category_color(&t, &e.category))));
                dots.push('•');
            }
            if due_n > 0 && dots.len() < 3 {
                dot_line.push(Span::styled("▪", Style::default().fg(t.yellow)));
                dots.push('▪');
            }
            for _ in dots.chars().count()..4 { dot_line.push(Span::raw(" ")); }
            dot_spans.extend(dot_line);
            day += Duration::days(1);
        }
        lines.push(Line::from(num_spans));
        if !compact { lines.push(Line::from(dot_spans)); }
    }
    lines
}

fn agenda_lines(app: &App, day: NaiveDate, header: &str) -> Vec<Line<'static>> {
    let t = app.theme;
    let mut out = vec![Line::from(Span::styled(
        header.to_string(),
        Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
    ))];
    for e in app.calendar.events_on(day) {
        let time = e.time.clone().map(|s| format!("{s} ")).unwrap_or_default();
        out.push(Line::from(vec![
            Span::styled(" • ", Style::default().fg(category_color(&t, &e.category))),
            Span::styled(format!("{time}{}", e.title), Style::default().fg(t.text)),
        ]));
    }
    for td in app.calendar.due_on(day) {
        out.push(Line::from(vec![
            Span::styled(" ▪ due: ", Style::default().fg(t.yellow)),
            Span::styled(td.title.clone(), Style::default().fg(t.text)),
        ]));
    }
    if out.len() == 1 {
        out.push(Line::from(Span::styled("   —", Style::default().fg(t.muted))));
    }
    out
}

pub fn render_panel(f: &mut Frame, app: &mut App, area: Rect, focused: bool) {
    let cur = app.calendar.cursor;
    let title = format!("{} {}", ["JAN","FEB","MAR","APR","MAY","JUN","JUL","AUG","SEP","OCT","NOV","DEC"][cur.month0() as usize], cur.year());
    let block = app.theme.panel_block(&title, focused);
    f.render_widget(Paragraph::new(month_grid_lines(app, true)).block(block), area);
}

pub fn render_zoomed(f: &mut Frame, app: &mut App) {
    let area = f.area();
    f.render_widget(
        ratatui::widgets::Block::default().style(Style::default().bg(app.theme.bg)), area);
    let rows = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(area);
    let cols = Layout::horizontal([Constraint::Percentage(55), Constraint::Percentage(45)]).split(rows[0]);

    let cur = app.calendar.cursor;
    let title = format!("CALENDAR — {} {}", ["January","February","March","April","May","June","July","August","September","October","November","December"][cur.month0() as usize], cur.year());
    f.render_widget(
        Paragraph::new(month_grid_lines(app, false)).block(app.theme.panel_block(&title, true)),
        cols[0],
    );

    let mut agenda = agenda_lines(app, cur, &format!("{} · {}", cur.weekday(), cur));
    agenda.push(Line::raw(""));
    agenda.push(Line::from(Span::styled(
        "NEXT 7 DAYS",
        Style::default().fg(app.theme.accent).add_modifier(Modifier::BOLD),
    )));
    for i in 1..=7i64 {
        let d = cur + Duration::days(i);
        let ev = app.calendar.events_on(d).len();
        let due = app.calendar.due_on(d).len();
        if ev + due > 0 {
            agenda.push(Line::from(Span::styled(
                format!(" {} {:>2} — {ev} events · {due} due", d.weekday(), d.day()),
                Style::default().fg(app.theme.text),
            )));
        }
    }
    f.render_widget(
        Paragraph::new(agenda).block(app.theme.panel_block("AGENDA", false)),
        cols[1],
    );

    let hint = " ←↓↑→ move · [/] month · t today · a add event · d delete · esc home ";
    f.render_widget(Paragraph::new(hint).style(app.theme.hint()), rows[1]);

    if app.calendar.form.is_some() {
        render_event_form(f, app, area);
    }
}

fn render_event_form(f: &mut Frame, app: &mut App, screen: Rect) {
    let form = app.calendar.form.as_ref().unwrap();
    let t = app.theme;
    let labels = ["title", "time (HH:MM)", "category"];
    let w = 50.min(screen.width.saturating_sub(4));
    let popup = Rect {
        x: screen.x + (screen.width.saturating_sub(w)) / 2,
        y: screen.y + (screen.height.saturating_sub(7)) / 2,
        width: w, height: 7,
    };
    f.render_widget(Clear, popup);
    let block = t.panel_block(&format!("NEW EVENT — {}", app.calendar.cursor), true)
        .style(Style::default().bg(t.surface));
    let inner = block.inner(popup);
    f.render_widget(block, popup);
    let mut lines: Vec<Line> = labels.iter().enumerate().map(|(i, label)| {
        let focused = i == form.focus;
        let cursor = if focused { "▏" } else { "" };
        Line::from(vec![
            Span::styled(format!(" {label:<14}"), Style::default().fg(if focused { t.accent } else { t.muted })),
            Span::styled(format!("{}{cursor}", form.fields[i]), Style::default().fg(t.text)),
        ])
    }).collect();
    lines.push(Line::from(Span::styled(
        "  categories: work personal health deadline",
        Style::default().fg(t.muted),
    )));
    f.render_widget(Paragraph::new(lines), inner);
}
```

- [ ] **Step 6: Wire into app** — `pub calendar: CalendarState` (load in `new`), `Screen::Calendar` routing, home `"calendar"` arm → `calendar::render_panel`. Also: after any todo mutation (complete/add/delete in `ui/todos.rs`), call `app.calendar.load(&app.conn)` so due-dots stay fresh.

- [ ] **Step 7: Run tests + run app**

Run: `cargo test` → PASS. `cargo run`: `3` zooms calendar; grid shows month with today highlighted; arrows/hjkl move cursor; `[`/`]` change month; `a` adds an event on cursor date, colored dot appears under the day; due todos show yellow `▪`; agenda pane lists selected day + next-7-days digest; home panel shows compact month.

- [ ] **Step 8: Commit**

```bash
git add -A && git commit -m "feat: calendar — month grid, event dots, agenda digest"
```

---

### Task 8: Ideas module

**Files:**
- Modify: `src/db.rs` (idea functions + test), `src/app.rs`, `src/ui/mod.rs`, `src/ui/home.rs`
- Create: `src/ui/ideas.rs`

**Interfaces:**
- Produces in `db.rs`: `ideas_list(conn) -> rusqlite::Result<Vec<Idea>>` (newest first, `dropped` last), `idea_add(conn, title: &str) -> rusqlite::Result<()>`, `idea_set_body(conn, id: i64, body: &str) -> rusqlite::Result<()>`, `idea_cycle_status(conn, id: i64) -> rusqlite::Result<String>` (returns new status), `idea_delete(conn, id: i64) -> rusqlite::Result<()>`
- Status cycle order: `spark → brewing → active → shipped → dropped → spark`.
- Produces in `ui/ideas.rs`: `IdeasState { items: Vec<Idea>, selected: usize, input: Option<String>, body_edit: Option<String> }`, `load`, `handle_key`, `render_panel`, `render_zoomed`. Keys: `a` instant capture (title), `enter` edit body of selected (multiline not needed — single wrapped string, Enter saves), `s` cycle status, `d` delete.

- [ ] **Step 1: Write failing status-cycle test** (in `src/db.rs` tests mod)

```rust
    #[test]
    fn idea_status_cycles_through_all_states() {
        let c = test_conn();
        idea_add(&c, "solar tracker").unwrap();
        let mut seen = vec!["spark".to_string()];
        for _ in 0..5 {
            seen.push(idea_cycle_status(&c, 1).unwrap());
        }
        assert_eq!(seen, vec!["spark", "brewing", "active", "shipped", "dropped", "spark"]);
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test idea`
Expected: FAIL.

- [ ] **Step 3: Implement db functions**

```rust
use crate::models::Idea;

pub fn ideas_list(conn: &Connection) -> rusqlite::Result<Vec<Idea>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, body, status, created_at FROM ideas
         ORDER BY status = 'dropped', created_at DESC",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(Idea { id: r.get(0)?, title: r.get(1)?, body: r.get(2)?, status: r.get(3)?, created_at: r.get(4)? })
    })?;
    rows.collect()
}

pub fn idea_add(conn: &Connection, title: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO ideas (title, created_at) VALUES (?1, ?2)",
        params![title, chrono::Utc::now().to_rfc3339()],
    )?;
    Ok(())
}

pub fn idea_set_body(conn: &Connection, id: i64, body: &str) -> rusqlite::Result<()> {
    conn.execute("UPDATE ideas SET body = ?1 WHERE id = ?2", params![body, id])?;
    Ok(())
}

const IDEA_STATUSES: [&str; 5] = ["spark", "brewing", "active", "shipped", "dropped"];

pub fn idea_cycle_status(conn: &Connection, id: i64) -> rusqlite::Result<String> {
    let cur: String = conn.query_row("SELECT status FROM ideas WHERE id = ?1", [id], |r| r.get(0))?;
    let i = IDEA_STATUSES.iter().position(|s| *s == cur).unwrap_or(0);
    let next = IDEA_STATUSES[(i + 1) % IDEA_STATUSES.len()].to_string();
    conn.execute("UPDATE ideas SET status = ?1 WHERE id = ?2", params![next, id])?;
    Ok(next)
}

pub fn idea_delete(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM ideas WHERE id = ?1", [id])?;
    Ok(())
}
```

- [ ] **Step 4: Run tests to verify pass**

Run: `cargo test` → PASS.

- [ ] **Step 5: Ideas UI** — `src/ui/ideas.rs`, following the exact `habits.rs` structure (state + input buffer + list). Status badges colored: `spark`→yellow `✦`, `brewing`→peach `◌`, `active`→blue `▶`, `shipped`→green `✔`, `dropped`→muted `✕`. Zoomed view: list left (60%), selected idea's body right (40%, wrapped `Paragraph`); `a` opens instant title capture in the hint bar (identical pattern to habits add); `enter` edits body in the hint bar buffer (`body_edit: Some(current_body)`, Enter saves via `idea_set_body`, Esc cancels); `s` cycles status; `d` deletes. Home panel: top 5 ideas as `badge title` lines with muted `created` date. Both entry buffers set `app.mode = InputMode::Editing` while open.

- [ ] **Step 6: Wire into app** — `pub ideas: IdeasState`, `Screen::Ideas` routing, home `"ideas"` arm. Same pattern as Tasks 3/6/7.

- [ ] **Step 7: Run tests + run app**

`cargo test` → PASS. `cargo run`: `4` zooms ideas; `a` captures instantly; `s` cycles badge/color and dropped sinks to bottom; `enter` edits body shown in right pane; home panel lists newest ideas.

- [ ] **Step 8: Commit**

```bash
git add -A && git commit -m "feat: ideas portal — instant capture, status lifecycle"
```

---

### Task 9: Pomodoro module

**Files:**
- Modify: `src/db.rs` (pomodoro functions), `src/app.rs`, `src/ui/mod.rs`, `src/ui/home.rs`, `src/ui/todos.rs` (`p` key)
- Create: `src/ui/pomodoro.rs`

**Interfaces:**
- Produces in `db.rs`:
  - `pomo_start(conn, todo_id: Option<i64>, kind: &str) -> rusqlite::Result<i64>` (inserts row, started_at = now, returns id)
  - `pomo_finish(conn, id: i64, completed: bool) -> rusqlite::Result<()>` (sets ended_at = now, completed)
  - `pomo_count_today(conn, date: NaiveDate) -> rusqlite::Result<u32>` (completed focus sessions where `started_at` date = date)
- Produces in `ui/pomodoro.rs`:

```rust
pub struct ActiveSession {
    pub db_id: i64,
    pub kind: Kind,                     // Kind { Focus, Break }
    pub todo_title: Option<String>,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub duration: chrono::Duration,
    pub paused_at: Option<chrono::DateTime<chrono::Utc>>,
}
impl ActiveSession {
    pub fn remaining(&self, now: chrono::DateTime<chrono::Utc>) -> chrono::Duration;
}
pub struct PomodoroState { pub active: Option<ActiveSession>, pub today_count: u32, pub suggest_break: bool }
pub fn start(app: &mut App, todo_id: Option<i64>, todo_title: Option<String>); // callable from todos.rs
pub fn handle_key(app: &mut App, key: KeyEvent); // s start/skip-suggestion, space pause/resume, x abandon
pub fn on_tick(app: &mut App); // called from App::tick — completion check + bell
pub fn render_panel(f, app, area, focused);
pub fn render_zoomed(f, app);
```

- Consumes: `config.pomodoro.focus_min` / `break_min`; todos selection for `p`.

- [ ] **Step 1: Write failing remaining/pause test** (in `src/ui/pomodoro.rs`)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, TimeZone, Utc};

    #[test]
    fn remaining_counts_down_and_freezes_while_paused() {
        let t0 = Utc.with_ymd_and_hms(2026, 7, 15, 10, 0, 0).unwrap();
        let mut s = ActiveSession {
            db_id: 1, kind: Kind::Focus, todo_title: None,
            started_at: t0, duration: Duration::minutes(25), paused_at: None,
        };
        assert_eq!(s.remaining(t0 + Duration::minutes(10)), Duration::minutes(15));
        s.paused_at = Some(t0 + Duration::minutes(10));
        // clock advances, remaining doesn't
        assert_eq!(s.remaining(t0 + Duration::minutes(20)), Duration::minutes(15));
    }

    #[test]
    fn resume_shifts_started_at_so_no_time_is_lost() {
        let t0 = Utc.with_ymd_and_hms(2026, 7, 15, 10, 0, 0).unwrap();
        let mut s = ActiveSession {
            db_id: 1, kind: Kind::Focus, todo_title: None,
            started_at: t0, duration: Duration::minutes(25),
            paused_at: Some(t0 + Duration::minutes(10)),
        };
        s.resume(t0 + Duration::minutes(30)); // paused 20 min
        assert_eq!(s.remaining(t0 + Duration::minutes(30)), Duration::minutes(15));
        assert_eq!(s.paused_at, None);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test remaining`
Expected: FAIL — types not defined.

- [ ] **Step 3: Implement** — core logic:

```rust
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind { Focus, Break }

impl ActiveSession {
    pub fn remaining(&self, now: chrono::DateTime<chrono::Utc>) -> chrono::Duration {
        let effective_now = self.paused_at.unwrap_or(now);
        self.duration - (effective_now - self.started_at)
    }
    pub fn resume(&mut self, now: chrono::DateTime<chrono::Utc>) {
        if let Some(p) = self.paused_at.take() {
            self.started_at += now - p;
        }
    }
}
```

db functions:

```rust
pub fn pomo_start(conn: &Connection, todo_id: Option<i64>, kind: &str) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO pomodoros (todo_id, started_at, kind) VALUES (?1, ?2, ?3)",
        params![todo_id, chrono::Utc::now().to_rfc3339(), kind],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn pomo_finish(conn: &Connection, id: i64, completed: bool) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE pomodoros SET ended_at = ?1, completed = ?2 WHERE id = ?3",
        params![chrono::Utc::now().to_rfc3339(), completed as i64, id],
    )?;
    Ok(())
}

pub fn pomo_count_today(conn: &Connection, date: NaiveDate) -> rusqlite::Result<u32> {
    conn.query_row(
        "SELECT COUNT(*) FROM pomodoros
         WHERE kind = 'focus' AND completed = 1 AND date(started_at) = ?1",
        [date.to_string()],
        |r| r.get(0),
    )
}
```

Behavior wiring:
- `start(app, todo_id, title)`: kind = Focus with `config.pomodoro.focus_min`; inserts via `pomo_start`, sets `app.pomodoro.active`, clears `suggest_break`.
- `handle_key`: `s` → if `suggest_break` start a Break session (`break_min`), else start unlinked Focus; `space` → pause/resume (`paused_at = Some(now)` / `resume(now)`); `x` → `pomo_finish(id, false)`, clear active.
- `on_tick` (call at top of `App::tick`): if active and `remaining(now) <= Duration::zero()`: `pomo_finish(id, true)`; if it was Focus → `print!("\x07")` (bell), `today_count` reload via `pomo_count_today`, `suggest_break = true`, status `"focus done — s starts a 5m break"`; if Break → bell, status `"break over — s starts focus"`. Clear active either way.
- In `ui/todos.rs` `handle_key`, add: `KeyCode::Char('p') if n > 0 => { let row = &app.todos.items[app.todos.selected]; crate::ui::pomodoro::start(app, Some(row.todo.id), Some(row.todo.title.clone())); app.status = Some(format!("⏱ pomodoro started: {}", row.todo.title)); }`
- `render_zoomed`: centered big timer — `mm:ss` rendered as 5 large glyphs built from `█` blocks (define `const DIGITS: [[&str; 5]; 11]` — a 3×5 block font for `0-9` and `:` — render the remaining time by concatenating digit rows into 5 `Line`s), colored green (Focus running) / peach (Break) / yellow (paused); beneath: linked todo title, `● ● ○ ○` dots for today's completed count, hint ` s start · space pause · x abandon · esc home `.
- `render_panel`: one-liner — `▶ 18:42 focus · ship report` or `3 done today · s to start`, plus `suggest_break` nudge.

- [ ] **Step 4: Run tests to verify pass**

Run: `cargo test` → PASS (2 new).

- [ ] **Step 5: Wire into app** — `pub pomodoro: PomodoroState`; `App::tick` calls `crate::ui::pomodoro::on_tick(self)` before the date-rollover check; `Screen::Pomodoro` routing; home `"pomodoro"` arm. `today_count` loaded in `App::new` and on date rollover.

- [ ] **Step 6: Run the app**

`cargo run`: `5` zooms pomodoro; `s` starts 25:00 big-glyph countdown ticking every 250ms poll; `space` freezes (yellow); from todos, `p` on a task starts a linked session and shows its title; on completion terminal bell rings, count dot fills, break suggested; `x` abandons.

- [ ] **Step 7: Commit**

```bash
git add -A && git commit -m "feat: pomodoro — db-backed sessions, todo link, block-font timer"
```

---

### Task 10: Stats module

**Files:**
- Modify: `src/db.rs` (stats queries + tests), `src/app.rs`, `src/ui/mod.rs`, `src/ui/home.rs`
- Create: `src/ui/stats.rs`

**Interfaces:**
- Produces in `db.rs`:
  - `stat_habit_days(conn, since: NaiveDate) -> rusqlite::Result<Vec<(NaiveDate, u32)>>` (date → habits checked that day)
  - `stat_todo_velocity(conn, since: NaiveDate) -> rusqlite::Result<Vec<(NaiveDate, u32, u32)>>` (date → created, completed)
  - `stat_focus_minutes(conn, since: NaiveDate) -> rusqlite::Result<Vec<(NaiveDate, u32)>>`
  - `stat_focus_by_project(conn, since: NaiveDate) -> rusqlite::Result<Vec<(String, u32)>>` (project or "(none)" → minutes, desc)
  - `pub struct WeekStats { pub habit_pct: u32, pub todos_done: u32, pub focus_min: u32 }` + `stat_week(conn, week_start: NaiveDate) -> rusqlite::Result<WeekStats>` (Mon-start week; habit_pct = checked / (habit_count × 7) × 100)
  - `habit_best_streak(conn, id: i64) -> rusqlite::Result<u32>` (longest consecutive-day run in `habit_log`)
- Produces in `ui/stats.rs`: `StatsState { range: Range }` (`Range { Week, Month, Year }`, key `r` cycles), `render_panel`, `render_zoomed`, `handle_key`. All data fetched inside render via the queries above (stats reads are cheap; no caching state).

- [ ] **Step 1: Write failing query tests** (in `src/db.rs` tests mod)

```rust
    #[test]
    fn stat_todo_velocity_buckets_by_day() {
        let c = test_conn();
        let id = todo_add(&c, &new_todo("t1")).unwrap();
        todo_add(&c, &new_todo("t2")).unwrap();
        todo_complete(&c, id, d("2026-07-15")).unwrap();
        let today = chrono::Utc::now().date_naive();
        let v = stat_todo_velocity(&c, today - chrono::Duration::days(7)).unwrap();
        let row = v.iter().find(|(dt, _, _)| *dt == today).expect("today bucket");
        assert_eq!((row.1, row.2), (2, 1)); // 2 created, 1 completed today
    }

    #[test]
    fn stat_focus_by_project_joins_todo_project() {
        let c = test_conn();
        let mut t = new_todo("work task"); t.project = Some("acme".into());
        let tid = todo_add(&c, &t).unwrap();
        let pid = pomo_start(&c, Some(tid), "focus").unwrap();
        pomo_finish(&c, pid, true).unwrap();
        let by = stat_focus_by_project(&c, d("2020-01-01")).unwrap();
        assert_eq!(by.len(), 1);
        assert_eq!(by[0].0, "acme");
    }

    #[test]
    fn stat_week_computes_habit_pct() {
        let c = test_conn();
        habit_add(&c, "gym").unwrap();
        // check 7/7 days of the week starting 2026-07-13 (a Monday)
        for i in 0..7 {
            habit_toggle(&c, 1, d("2026-07-13") + chrono::Duration::days(i)).unwrap();
        }
        let w = stat_week(&c, d("2026-07-13")).unwrap();
        assert_eq!(w.habit_pct, 100);
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test stat`
Expected: FAIL.

- [ ] **Step 3: Implement queries in `src/db.rs`**

```rust
pub fn stat_habit_days(conn: &Connection, since: NaiveDate) -> rusqlite::Result<Vec<(NaiveDate, u32)>> {
    let mut stmt = conn.prepare(
        "SELECT date, COUNT(*) FROM habit_log WHERE date >= ?1 GROUP BY date ORDER BY date",
    )?;
    let rows = stmt.query_map([since.to_string()], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, u32>(1)?))
    })?;
    Ok(rows.filter_map(|r| r.ok())
        .filter_map(|(s, n)| s.parse().ok().map(|dt| (dt, n)))
        .collect())
}

pub fn stat_todo_velocity(conn: &Connection, since: NaiveDate) -> rusqlite::Result<Vec<(NaiveDate, u32, u32)>> {
    let mut stmt = conn.prepare(
        "SELECT day, SUM(created), SUM(completed) FROM (
            SELECT date(created_at) AS day, 1 AS created, 0 AS completed FROM todos WHERE date(created_at) >= ?1
            UNION ALL
            SELECT date(done_at) AS day, 0, 1 FROM todos WHERE done_at IS NOT NULL AND date(done_at) >= ?1
         ) GROUP BY day ORDER BY day",
    )?;
    let rows = stmt.query_map([since.to_string()], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, u32>(1)?, r.get::<_, u32>(2)?))
    })?;
    Ok(rows.filter_map(|r| r.ok())
        .filter_map(|(s, a, b)| s.parse().ok().map(|dt| (dt, a, b)))
        .collect())
}

pub fn stat_focus_minutes(conn: &Connection, since: NaiveDate) -> rusqlite::Result<Vec<(NaiveDate, u32)>> {
    let mut stmt = conn.prepare(
        "SELECT date(started_at),
                CAST(SUM((julianday(ended_at) - julianday(started_at)) * 1440) AS INTEGER)
         FROM pomodoros
         WHERE kind = 'focus' AND completed = 1 AND ended_at IS NOT NULL AND date(started_at) >= ?1
         GROUP BY date(started_at) ORDER BY date(started_at)",
    )?;
    let rows = stmt.query_map([since.to_string()], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, u32>(1)?))
    })?;
    Ok(rows.filter_map(|r| r.ok())
        .filter_map(|(s, n)| s.parse().ok().map(|dt| (dt, n)))
        .collect())
}

pub fn stat_focus_by_project(conn: &Connection, since: NaiveDate) -> rusqlite::Result<Vec<(String, u32)>> {
    let mut stmt = conn.prepare(
        "SELECT COALESCE(t.project, '(none)') AS proj,
                CAST(SUM((julianday(p.ended_at) - julianday(p.started_at)) * 1440) AS INTEGER) AS mins
         FROM pomodoros p LEFT JOIN todos t ON t.id = p.todo_id
         WHERE p.kind = 'focus' AND p.completed = 1 AND p.ended_at IS NOT NULL AND date(p.started_at) >= ?1
         GROUP BY proj ORDER BY mins DESC",
    )?;
    let rows = stmt.query_map([since.to_string()], |r| Ok((r.get(0)?, r.get(1)?)))?;
    rows.collect()
}

pub struct WeekStats {
    pub habit_pct: u32,
    pub todos_done: u32,
    pub focus_min: u32,
}

pub fn stat_week(conn: &Connection, week_start: NaiveDate) -> rusqlite::Result<WeekStats> {
    let end = week_start + chrono::Duration::days(6);
    let habit_count: u32 = conn.query_row(
        "SELECT COUNT(*) FROM habits WHERE archived = 0", [], |r| r.get(0))?;
    let checked: u32 = conn.query_row(
        "SELECT COUNT(*) FROM habit_log WHERE date >= ?1 AND date <= ?2",
        params![week_start.to_string(), end.to_string()], |r| r.get(0))?;
    let habit_pct = if habit_count == 0 { 0 } else { checked * 100 / (habit_count * 7) };
    let todos_done: u32 = conn.query_row(
        "SELECT COUNT(*) FROM todos WHERE done_at IS NOT NULL AND date(done_at) >= ?1 AND date(done_at) <= ?2",
        params![week_start.to_string(), end.to_string()], |r| r.get(0))?;
    let focus_min: u32 = conn.query_row(
        "SELECT COALESCE(CAST(SUM((julianday(ended_at) - julianday(started_at)) * 1440) AS INTEGER), 0)
         FROM pomodoros WHERE kind = 'focus' AND completed = 1 AND ended_at IS NOT NULL
           AND date(started_at) >= ?1 AND date(started_at) <= ?2",
        params![week_start.to_string(), end.to_string()], |r| r.get(0))?;
    Ok(WeekStats { habit_pct, todos_done, focus_min })
}
```

- [ ] **Step 4: Run tests to verify pass**

Run: `cargo test` → PASS.

- [ ] **Step 5: Stats UI** — `src/ui/stats.rs`. Zoomed layout: 2×2 grid of themed panels + range in title (`STATS — month`, `r` cycles Week/Month/Year → `since = today - 7/30/365 days`).

  1. **HABIT HEATMAP** (top-left): GitHub-style grid — columns = weeks, 7 rows Mon–Sun, cell = `■ ` colored by intensity from `stat_habit_days` relative to unarchived habit count (0 / ≤⅓ / ≤⅔ / full). Per the global theme rule the ramp lives in `theme.rs`, not inline — add to `Theme`:

```rust
    pub heat: [Color; 4], // habit heatmap intensity ramp, 0 → max
```

  default `[Color::Rgb(49, 50, 68), Color::Rgb(87, 116, 84), Color::Rgb(126, 171, 120), Color::Rgb(166, 227, 161)]` (surface → green). Week columns fit the selected `range` (Year = last 52 weeks), truncated to panel width from the left (oldest drops first). Below the grid, one line per habit: `gym  ⚡12 now · 31 best` — current streak via `db::habit_streak`, best via a new `db::habit_best_streak(conn, id) -> rusqlite::Result<u32>` (fetch all dates ascending, single scan counting the longest consecutive run — same date-walk shape as `habit_streak`; add a db test: days 1,2,3,5,6 → best 3).
  2. **TODO VELOCITY** (top-right): two `Sparkline`s stacked (created / completed, blue / green) from `stat_todo_velocity` densified into a contiguous `Vec<u64>` (missing days = 0), with `▲ n created · ✔ n done` totals line.
  3. **FOCUS** (bottom-left): `BarChart` of `stat_focus_minutes` per day (last 14 bars max, label = day-of-month, bar style peach) + per-project lines `#acme ████████ 3h 20m` (bar = proportional `█` repeat, project names blue).
  4. **WEEK REVIEW** (bottom-right): from `stat_week(this_monday)` vs `stat_week(this_monday - 7d)`: three rows `habits 78% ▲ +12`, `todos closed 14 ▼ -3`, `focus 6h 40m ▲ +1h 05m` — ▲ green, ▼ red, formatted `xh ym`.

  Home panel (`render_panel`): compact — streak line for best habit, `▁▂▅▇` mini sparkline of last-7-day completions, `focus 2h 10m today`.

- [ ] **Step 6: Wire into app** — `pub stats: StatsState`, `Screen::Stats` routing (`r` key in `handle_key`), home `"stats"` arm.

- [ ] **Step 7: Run the app**

`cargo run`: `6` zooms stats; heatmap shows green ramp for checked days (verify by checking habits across a few fake days: toggle with `y` for yesterday, today); velocity sparklines move after adding/completing todos; focus bars appear after a completed pomodoro (test with `focus_min = 1` in config for a 1-minute session); week review shows deltas; `r` cycles ranges.

- [ ] **Step 8: Commit**

```bash
git add -A && git commit -m "feat: stats — heatmap, velocity sparklines, focus bars, week review"
```

---

### Task 11: Polish + release pipeline

**Files:**
- Create: `.github/workflows/release.yml`, `README.md`, `config.example.toml`
- Modify: anything `cargo clippy` flags

**Interfaces:**
- Consumes: everything.
- Produces: tagged releases with binaries for Linux x86_64, macOS aarch64, Windows x86_64.

- [ ] **Step 1: Clippy + fmt clean**

Run: `cargo clippy -- -D warnings` and `cargo fmt`
Fix every warning (typical: needless clones, `&String` params). Re-run until clean.

- [ ] **Step 2: `config.example.toml`**

```toml
# Copy to your config dir as config.toml:
#   Linux:   ~/.config/productivo/config.toml
#   macOS:   ~/Library/Application Support/productivo/config.toml
#   Windows: %APPDATA%\productivo\config.toml

# Home-screen panel order: first three fill the left column top-to-bottom,
# next three fill the right column.
panels = ["habits", "calendar", "ideas", "todos", "pomodoro", "stats"]

[pomodoro]
focus_min = 25
break_min = 5

[theme]
# Optional hex overrides (defaults: Catppuccin Mocha)
# accent = "#cba6f7"
# green  = "#a6e3a1"
# red    = "#f38ba8"
# yellow = "#f9e2af"
# blue   = "#89b4fa"
# peach  = "#fab387"
```

- [ ] **Step 3: README** — short: what it is, screenshot placeholder, install (`cargo install --path .` + releases link), keys table (global + per module), config file explanation, data location, phase-2 roadmap line (ActivityWatch, remote backend, GCal — link to the spec).

- [ ] **Step 4: Release workflow**

`.github/workflows/release.yml`:

```yaml
name: release
on:
  push:
    tags: ["v*"]
permissions:
  contents: write
jobs:
  build:
    strategy:
      matrix:
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
            ext: ""
          - os: macos-latest
            target: aarch64-apple-darwin
            ext: ""
          - os: windows-latest
            target: x86_64-pc-windows-msvc
            ext: ".exe"
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}
      - run: cargo build --release --target ${{ matrix.target }}
      - shell: bash
        run: |
          cp target/${{ matrix.target }}/release/productivo${{ matrix.ext }} \
             productivo-${{ matrix.target }}${{ matrix.ext }}
      - uses: softprops/action-gh-release@v2
        with:
          files: productivo-${{ matrix.target }}${{ matrix.ext }}
```

- [ ] **Step 5: Full verification pass**

Run: `cargo test` → all green. `cargo run` → walk every module: home glance, zoom each of the 6, add/complete/recur a todo, check habits incl. yesterday, add calendar event, capture idea, run a 1-minute pomodoro to the bell, view stats in all 3 ranges, edit `config.toml` panel order and confirm the home grid rearranges. Resize the terminal while on each screen — no panics.

- [ ] **Step 6: Commit**

```bash
git add -A && git commit -m "chore: clippy clean, readme, example config, release CI"
```
