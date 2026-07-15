use chrono::NaiveDate;
use rusqlite::params;
use rusqlite::Connection;
use std::error::Error;
use std::fmt;

use crate::models::Habit;

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
}
