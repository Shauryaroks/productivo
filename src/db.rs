use rusqlite::Connection;
use std::error::Error;
use std::fmt;

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
