# productivo

A single-binary terminal productivity dashboard. Habits, todos, calendar,
ideas, pomodoro, and stats — one glanceable home screen, zoom into any panel
with vim-ish keys. No server, no accounts: everything lives in a local
SQLite file.

![screenshot placeholder](docs/screenshot.png)
<!-- TODO: replace with a real screenshot of the home grid -->

## Install

```bash
cargo install --path .
```

Prebuilt binaries for Linux (x86_64), macOS (aarch64), and Windows (x86_64)
are attached to each [GitHub release](../../releases).

## Keys

### Global (any screen)

| Key       | Action                              |
|-----------|--------------------------------------|
| `Tab` / `Shift+Tab` | Move focus between home panels |
| `Enter`   | Zoom into the focused panel          |
| `1`–`6`   | Jump straight to a panel by position |
| `Esc`     | Back to the home screen              |
| `q`       | Quit                                 |

### Habits

| Key | Action |
|-----|--------|
| `j`/`k` or `↓`/`↑` | Move selection |
| `space` | Toggle checked for the selected day |
| `y` | Toggle viewing yesterday (only yesterday is editable) |
| `a` | Add a habit |
| `d` | Archive the selected habit |
| `J`/`K` | Reorder selected habit down/up |

### Todos

| Key | Action |
|-----|--------|
| `j`/`k` or `↓`/`↑` | Move selection |
| `Enter` | Expand/collapse subtasks |
| `space` / `x` | Complete (recurring todos spawn their next occurrence) |
| `u` | Undo the last completion |
| `a` | Add a todo |
| `A` | Add a subtask under the selected todo |
| `e` | Edit the selected todo |
| `d` | Delete the selected todo |
| `/` | Filter by title/project/tags |
| `g` | Toggle grouping by project |
| `p` | Start a pomodoro linked to the selected todo |

### Calendar

| Key | Action |
|-----|--------|
| `h`/`j`/`k`/`l` or arrows | Move a day / week |
| `[` / `]` | Previous / next month |
| `t` | Jump to today |
| `a` | Add an event on the selected day |
| `d` | Delete the first event on the selected day |

### Ideas

| Key | Action |
|-----|--------|
| `j`/`k` or `↓`/`↑` | Move selection |
| `a` | Capture a new idea |
| `Enter` | Edit the body |
| `s` | Cycle status (spark → brewing → active → shipped → dropped) |
| `d` | Delete the selected idea |

### Pomodoro

| Key | Action |
|-----|--------|
| `s` | Start a focus session (or the suggested break once one finishes) |
| `space` | Pause / resume |
| `x` | Abandon the running session |

### Stats

| Key | Action |
|-----|--------|
| `r` | Cycle range (week → month → year) |

## Config

Copy [`config.example.toml`](./config.example.toml) to your platform's
config directory as `config.toml`:

| Platform | Path |
|----------|------|
| Linux    | `~/.config/productivo/config.toml` |
| macOS    | `~/Library/Application Support/productivo/config.toml` |
| Windows  | `%APPDATA%\productivo\config\config.toml` |

It controls the home-panel order, pomodoro focus/break lengths, and theme
color overrides (defaults to Catppuccin Mocha). Missing or invalid values
fall back to defaults — there's no need to keep the whole file in sync.

## Data

All data lives in a single SQLite file at the platform data directory:

| Platform | Path |
|----------|------|
| Linux    | `~/.local/share/productivo/dash.db` |
| macOS    | `~/Library/Application Support/productivo/dash.db` |
| Windows  | `%APPDATA%\productivo\data\dash.db` |

Nothing leaves your machine.

## Roadmap

Phase 2 ideas — ActivityWatch screen-time integration, a remote sync backend
over Tailscale, and Google Calendar sync — are sketched in the
[design spec](docs/superpowers/specs/2026-07-15-productivo-tui-dashboard-design.md).
