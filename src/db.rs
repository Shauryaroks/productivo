use chrono::NaiveDate;
use rusqlite::params;
use rusqlite::Connection;
use std::error::Error;
use std::fmt;

use crate::models::{Event, Habit, Idea, Todo};
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

#[derive(Debug)]
struct SimpleError(&'static str);

impl fmt::Display for SimpleError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Error for SimpleError {}

pub fn open() -> Result<Connection, Box<dyn Error>> {
    let dirs = directories::ProjectDirs::from("", "", "productivo")
        .ok_or_else(|| Box::new(SimpleError("no home directory found")) as Box<dyn Error>)?;
    std::fs::create_dir_all(dirs.data_dir())?;
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

pub fn habit_move(conn: &Connection, id: i64, delta: i64) -> rusqlite::Result<bool> {
    let pos: i64 = conn.query_row("SELECT position FROM habits WHERE id = ?1", [id], |r| r.get(0))?;
    let neighbor: Option<(i64, i64)> = if delta > 0 {
        conn.query_row(
            "SELECT id, position FROM habits WHERE archived = 0 AND position > ?1 ORDER BY position LIMIT 1",
            [pos],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .ok()
    } else if delta < 0 {
        conn.query_row(
            "SELECT id, position FROM habits WHERE archived = 0 AND position < ?1 ORDER BY position DESC LIMIT 1",
            [pos],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .ok()
    } else {
        None
    };
    if let Some((nid, npos)) = neighbor {
        conn.execute("UPDATE habits SET position = ?1 WHERE id = ?2", params![npos, id])?;
        conn.execute("UPDATE habits SET position = ?1 WHERE id = ?2", params![pos, nid])?;
        Ok(true)
    } else {
        Ok(false)
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> chrono::NaiveDate { s.parse().unwrap() }

    fn test_conn() -> rusqlite::Connection {
        let c = rusqlite::Connection::open_in_memory().unwrap();
        migrate(&c).unwrap();
        c
    }

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
    fn habit_move_swaps_with_adjacent_unarchived_across_gaps() {
        let c = test_conn();
        habit_add(&c, "one").unwrap();
        habit_add(&c, "two").unwrap();
        habit_add(&c, "three").unwrap();
        habit_archive(&c, 2).unwrap();

        let moved = habit_move(&c, 3, -1).unwrap();
        assert!(moved);
        let names: Vec<i64> = habits_list(&c).unwrap().iter().map(|h| h.id).collect();
        assert_eq!(names, vec![3, 1]);

        let moved_again = habit_move(&c, 3, -1).unwrap();
        assert!(!moved_again);
        let names_after: Vec<i64> = habits_list(&c).unwrap().iter().map(|h| h.id).collect();
        assert_eq!(names_after, vec![3, 1]);
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
    fn idea_status_cycles_through_all_states() {
        let c = test_conn();
        idea_add(&c, "solar tracker").unwrap();
        let mut seen = vec!["spark".to_string()];
        for _ in 0..5 {
            seen.push(idea_cycle_status(&c, 1).unwrap());
        }
        assert_eq!(seen, vec!["spark", "brewing", "active", "shipped", "dropped", "spark"]);
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
}
