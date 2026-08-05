# productivo

A single-binary terminal productivity dashboard. Habits, todos, calendar,
ideas, pomodoro, and stats — one glanceable home screen, zoom into any panel
with vim-ish keys. No server, no accounts: everything lives in a local
SQLite file.

```text
╭ HABITS 0/4 ───────────────╮                 ···:::~~~~~~~~~:::·····     ···  ╭ AUG 2026 ─────────────────────────────╮
│ ○ gym                     │~::::····  ···::~~≈≈▒▒▓▓▓▓▓▓▓▓▓▓▓▓▒▒▒▒≈≈≈≈≈≈≈≈≈~~~│   Mo   Tu   We   Th   Fr   Sa   Su    │
│ ○ read 30m                │▓▓▓▒▒▒≈≈≈≈≈≈≈▒▒▓▓██████▓▓▓▓▓▓█████████████████████│   27   28   29   30   31    1    2    │
│ ○ meditate                │████████████████████▓▓▒▒▒▒▒▓▓▓████████████████████│                                       │
│ ○ no sugar                │██████████████████████████████████████████████████│    3    4    5    6    7    8    9    │
╰───────────────────────────╯██████████████████████████████████████████████████│                        •              │
╭ POMODORO ─────────────────╮███████████████████████████████████▓▓▒▒≈≈≈≈≈≈≈▒▒▓▓│   10   11   12   13   14   15   16    │
│     ███ ███   ███ ███     │╭ TODOS (3) ─────────────────────────────────────╮│                                       │
│       █ █   █ █ █ █ █     ││ ◉ ship v0.1.0  #productivo                     ││   17   18   19   20   21   22   23    │
│     ███ ███   █ █ █ █     ││ ○ write launch post  #productivo               ││   24   25   26   27   28   29   30    │
│     █     █ █ █ █ █ █     ││ ○ water the plants                             ││                                       │
│     ███ ███   ███ ███     ││                                                ││   31    1    2    3    4    5    6    │
│ 0 done today · s to start ││                                                ││                                       │
╰───────────────────────────╯│                                                │╰───────────────────────────────────────╯
╭ IDEAS (2) ────────────────╮│                                                │╭ PET · lv2 kitten · comet ─────────────╮
│ ◌ weekly email digest     ││                                                ││                             ♥  60%    │
│ ✦ terminal screensaver mod││                                                ││                             ☺  40%    │
│    aquarium when idle     │╰────────────────────────────────────────────────╯│▄▄▄▄    ▄▄▄▄    ▄▄▄                    │
│                           │╭ SUBS & TOOLS · 205/mo ─────────────────────────╮│▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀           ▀▄   ▄▀  │
│                           ││ ◆ domain name  900/y · d1                      ││▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▄▄▀▀▀▀▀▀▀▄ │
│                           ││ ◆ google one  130 · d15                        ││▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀ │
│                           ││ ⚒ obsidian                                     ││▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀ │
│                           ││ ⚒ ripgrep                                      ││▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀  ▀▀▀  ▀▀▀  ▀▀▀ ▀▀▀ │
│                           ││                                                ││ lv2 ▰▰▱▱▱ 2/5 to lv3                  │
╰───────────────────────────╯╰────────────────────────────────────────────────╯╰───────────────────────────────────────╯
 space check · a add · d archive · J/K reorder · y yesterday · tab next · enter zoom · q quit
```

(It's much prettier in color — Catppuccin theme, truecolor pet, aurora strip.)

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
| `1`–`7`   | Jump straight to a panel by position (`7` = subs) |
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
| `+` / `-` | Focus length ±5 min (applies to the next session) |
| `]` / `[` | Break length ±1 min |

Startup defaults live in `config.toml` (`[pomodoro] focus_min / break_min`),
and `sound = "/path/to/chime.wav"` plays a sound file when a timer ends
(via `paplay`/`pw-play`/`aplay`/`afplay`; unset = terminal bell).

### Subs & Tools

Track subscriptions (`◆`) and manually-noted CLI tools/packages (`⚒`) in one
strip, with a monthly total in the title.

| Key | Action |
|-----|--------|
| `a` | Add a subscription — `name [price] [renew day]`, add `y`/`yearly` for annual billing |
| `t` | Add a tool |
| `d` | Delete the selected entry |

### Stats & Pet

The stats slot is home to a pixel pet that levels up as you complete focus
sessions (1 level per 5). It's fed by finishing at least one session a day —
hungry pets mope at half speed. Needs a truecolor terminal (kitty, wezterm,
foot, alacritty, most modern ones).

| Key | Action |
|-----|--------|
| `p` | Pet it |
| `b` | Boop it |
| `c` | Cycle skins (comet / toast) |
| `r` | Cycle stats range (week → month → year) |

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
