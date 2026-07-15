# Productivo — TUI Productivity Dashboard: Design Spec

**Date:** 2026-07-15
**Status:** Approved design, pre-implementation
**Scope:** Phase 1 (local TUI + SQLite). Phase 2+ documented as direction only.

## What it is

A Rust TUI productivity dashboard for a single machine: habits, rich todos,
calendar, ideas capture, pomodoro, and a stats panel with real visualizations.
Ships as a single static binary. Data lives in one local SQLite file, kept
behind a clean data layer so a future self-hosted backend (old laptop +
Tailscale) and AI agent integrations (nemoclaw) bolt on without redesign.

Secondary goal: the author is learning Rust — the design stays synchronous,
single-crate, and idiomatic-simple on purpose.

## Decisions already made

| Decision | Choice | Rejected alternatives |
|---|---|---|
| Language / TUI | Rust + ratatui + crossterm | Python/Textual, Go/bubbletea |
| Storage | rusqlite (`bundled`), synchronous | sqlx + tokio (async before learning sync Rust), client/server now |
| Architecture | Single crate, `db.rs` as the only SQL boundary | Cargo workspace now (split later is mechanical), API server now |
| Distribution | `cargo install` + prebuilt binaries via CI | Docker for the TUI (bad TTY/volume story; Docker reserved for future backend) |
| Users/auth | None in phase 1 — every install is local single-user | Users table now |
| Screen-time tracking | Phase 2, via ActivityWatch REST API | Building per-OS trackers (Wayland/macOS permission swamp) |

## Data model

One SQLite file at `~/.local/share/productivo/dash.db` (via `directories`
crate, so platform-correct on macOS/Windows too). Created and migrated by
`db.rs` on startup using a `schema_version` pragma and plain `ALTER TABLE`
statements — no migration framework.

```sql
todos       id, title, notes, priority (0-2), due_date, project, tags (csv),
            parent_id (self-ref, one level of nesting enforced in code),
            recur_rule (nullable), done_at, created_at

habits      id, name, position, archived, created_at
habit_log   habit_id, date          -- one row = checked that day

events      id, title, date, time (nullable), category, color, notes

ideas       id, title, body, status (spark|brewing|active|shipped|dropped),
            created_at

pomodoros   id, todo_id (nullable), started_at, ended_at,
            kind (focus|break), completed
```

Key points:

- **Subtasks** = `parent_id` on todos. One level only.
- **Recurrence** = small text DSL in `recur_rule`: `daily`, `weekly:mon,thu`,
  `every:3d`. Completing a recurring todo stamps `done_at` and inserts the
  next occurrence. Nothing pre-materialized.
- **Habits** are definitions; `habit_log` is history. Streaks and heatmaps
  derive from the log via queries.
- **Pomodoros always log**, linked to a todo or not. Per-project focus stats
  come from the join.
- **No stats tables** — stats are queries over the above.
- Deliberately absent: users, soft-delete, sync/audit columns. The backend
  phase adds them when it exists.

## App architecture

**Event loop** — classic synchronous ratatui:

```
loop {
    terminal.draw(|f| ui::render(f, &app))?;
    if event::poll(250ms) { app.handle_key(event::read()?); }
    app.tick();   // pomodoro countdown, midnight rollover
}
```

No tokio, no threads, no channels. Pomodoro time remaining is computed at
render from `started_at + duration - now()`, so the timer survives restarts
(sessions are DB rows) and needs no timer thread.

**State** — one `App` struct owns everything: current screen (Home or a
zoomed module), per-module UI state (selections, scroll offsets), and an
`InputMode` enum (`Normal` / `Editing`) so form typing never collides with
hotkeys.

**Data flow** — UI never touches SQL. Every mutation is a
`db::function(&conn, args)` call followed by reloading the affected list into
`App`. Reads happen on state change, not per-frame. `db.rs` is the seam where
a future HTTP client replaces SQLite calls.

**Config** — `~/.config/productivo/config.toml` (serde + toml, parsed once at
startup): home-screen panel arrangement (which panel in which slot), theme
colors, pomodoro durations. No hot-reload, no settings UI.

**Dependencies (complete list):** `ratatui`, `crossterm`, `rusqlite`
(bundled), `serde`, `toml`, `chrono`, `directories`.

**Module layout:**

```
src/
├── main.rs      # terminal setup/teardown, event loop
├── app.rs       # App state, key dispatch, tick
├── db.rs        # all SQL, schema migration
├── models.rs    # plain structs
└── ui/          # home.rs + one file per module panel
```

## Modules

**Home** — composite dashboard; panels arranged per config. `Tab`/arrows move
panel focus, `Enter` zooms a module full-screen, `Esc` returns, `1-6` jumps
directly. Quick actions (toggle habit, start pomodoro) work from home.

**Habits** — today's checklist, `space` toggles, reorder, archive (history
kept). Inline current streak. Midnight rollover via tick. Yesterday is
editable; older days are not.

**Todos** — grouped by project or due date (toggleable). Form overlay for
add/edit (title, notes, priority, due, project, tags, recurrence). Subtasks
indent under parent; completing the last subtask prompts to complete the
parent. Recurring todos show `↻`. `/` filters, `!` cycles priority sort.
Overdue items surface red at top. `p` starts a pomodoro linked to the
selected todo.

**Calendar** — month grid, vim keys/arrows to move between days, dots colored
by event category under dates. Right pane: agenda for selected day + next-7-
days digest (events + due todos + habit status — the pre-AI "summarization").
`a` adds an event on the selected date.

**Ideas** — capture-first: `a` opens instant input (title now, body later).
Status badge per idea, `s` cycles `spark → brewing → active → shipped/
dropped`. Deliberately minimal: this is the surface AI agents later read and
act on, so its one job is frictionless capture.

**Pomodoro** — big ASCII countdown zoomed, compact `▶ mm:ss` on home. `s`
start (optionally todo-linked), `space` pause, break auto-suggested after
focus. Completion signal = terminal bell (`\x07`). Durations from config.

**Stats** — ratatui `Sparkline`/`BarChart`/`Chart` + block-character grids:

- GitHub-style habit heatmap (per habit and combined), intensity by completion
- Current + best streak per habit
- Todo velocity: created vs completed, 30-day sparkline
- Focus time: minutes/day bar chart + per-project breakdown
- Weekly review card: this week vs last (habits %, todos closed, focus hours,
  ▲/▼ deltas)
- Range switch: week / month / year

## Error handling

- DB errors: surface in a status line, never panic mid-session; startup
  failure (can't open/create DB) exits with a clear message.
- Terminal is restored on exit and on panic (panic hook that leaves raw mode).
- Config parse failure: fall back to defaults and show a one-line warning.

## Testing

Per module: unit tests where logic is non-trivial — recurrence rule parsing
and next-occurrence generation, streak/heatmap queries (against an in-memory
SQLite), midnight rollover. UI rendering is verified by running the app, not
by snapshot tests.

## Build order (phase 1)

Each step leaves a working app:

1. Skeleton — event loop, empty home grid, config load, DB init
2. Habits (simplest full loop: ratatui ⇄ rusqlite)
3. Todos (forms, subtasks, recurrence)
4. Calendar + events
5. Ideas
6. Pomodoro
7. Stats
8. Polish — config-driven panel arrangement, theming, release CI (prebuilt
   binaries for Linux/macOS/Windows)

## Phase 2+ (direction only — nothing built now, nothing blocked)

- **Screen time:** ActivityWatch REST API (`localhost:5600`) → daily summary
  rows (`app, category, seconds, date`) → stats section. Panel shows
  "ActivityWatch not detected" when absent. Building our own per-OS trackers
  is explicitly rejected (Wayland fragmentation, macOS permissions).
- **Backend:** split `models.rs` + `db.rs` into a `core` crate, wrap in an
  axum API, ship as a Docker image on the Tailscale'd laptop. TUI gains a
  `remote` mode where `db::` calls become HTTP. Users/auth arrive here.
- **Google Calendar sync:** `source` column on events + importer in the
  backend.
- **AI agents (nemoclaw):** connect at the backend API (or read SQLite
  directly pre-backend). Timestamped rows enable progress monitoring; the
  ideas table is the agent's work queue; screen-time summaries give it
  evidence to "push further." Requires no phase-1 changes.
