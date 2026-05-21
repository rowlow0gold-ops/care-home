use once_cell::sync::OnceCell;
use rusqlite::{Connection, Result};
use std::sync::Mutex;

static DB: OnceCell<Mutex<Connection>> = OnceCell::new();

pub fn init(app_dir: &str) -> Result<()> {
    let db_path = format!("{}/care_home.db", app_dir);
    let conn = Connection::open(&db_path)?;

    // Enable SQLCipher encryption
    conn.execute_batch("PRAGMA key = 'sunshine_care_2024';")?;
    conn.execute_batch("PRAGMA journal_mode = WAL;")?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;

    run_migrations(&conn)?;

    DB.set(Mutex::new(conn))
        .map_err(|_| rusqlite::Error::InvalidQuery)?;

    Ok(())
}

pub fn get() -> &'static Mutex<Connection> {
    DB.get().expect("Database not initialized")
}

fn run_migrations(conn: &Connection) -> Result<()> {
    conn.execute_batch(include_str!("../../migrations/001_initial.sql"))?;

    // 002: add unique index to meal_plans if not exists
    conn.execute_batch(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_meal_plans_unique
         ON meal_plans(week_start, day_of_week, meal_type);"
    )?;

    // 003: add is_flagged column to care_logs if not exists
    let has_flagged: i64 = conn.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('care_logs') WHERE name='is_flagged'",
        [],
        |r| r.get(0),
    ).unwrap_or(0);
    if has_flagged == 0 {
        conn.execute(
            "ALTER TABLE care_logs ADD COLUMN is_flagged INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }

    // 004: add is_deceased column to residents if not exists
    let has_deceased: i64 = conn.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('residents') WHERE name='is_deceased'",
        [],
        |r| r.get(0),
    ).unwrap_or(0);
    if has_deceased == 0 {
        conn.execute(
            "ALTER TABLE residents ADD COLUMN is_deceased INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }

    // 005: schedules table
    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS schedules (
            id           INTEGER PRIMARY KEY AUTOINCREMENT,
            staff_id     INTEGER NOT NULL,
            shift_date   TEXT    NOT NULL,
            shift_start  TEXT    NOT NULL,
            shift_end    TEXT    NOT NULL,
            shift_hours  REAL    NOT NULL,
            notes        TEXT,
            created_by   INTEGER,
            created_at   TEXT    NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (staff_id)   REFERENCES users(id),
            FOREIGN KEY (created_by) REFERENCES users(id)
        );
        CREATE INDEX IF NOT EXISTS idx_schedules_staff_date
            ON schedules(staff_id, shift_date);
    ")?;

    // 007: migrate old EMP-/MGR-/ADM- employee IDs to zero-padded numeric format
    let old_format: i64 = conn.query_row(
        "SELECT COUNT(*) FROM staff WHERE employee_id LIKE 'EMP-%' OR employee_id LIKE 'MGR-%' OR employee_id LIKE 'ADM-%'",
        [], |r| r.get(0),
    ).unwrap_or(0);
    if old_format > 0 {
        let id_map: &[(&str, &str)] = &[
            ("EMP-001", "0000001"), ("EMP-002", "0000002"), ("EMP-003", "0000003"),
            ("EMP-004", "0000004"), ("EMP-005", "0000005"), ("EMP-006", "0000006"),
            ("MGR-001", "0000007"), ("MGR-002", "0000008"),
            ("ADM-001", "0000009"),
        ];
        for (old, new) in id_map {
            conn.execute(
                "UPDATE staff SET employee_id = ?1 WHERE employee_id = ?2",
                rusqlite::params![new, old],
            ).ok();
        }
    }

    // 006: soft-delete (archive) for vitals — critical medical data must never be hard-deleted
    let has_archived: i64 = conn.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('vitals') WHERE name='is_archived'",
        [], |r| r.get(0),
    ).unwrap_or(0);
    if has_archived == 0 {
        conn.execute("ALTER TABLE vitals ADD COLUMN is_archived INTEGER NOT NULL DEFAULT 0", [])?;
        conn.execute("ALTER TABLE vitals ADD COLUMN archived_at TEXT", [])?;
    }

    // 008: teams — small-team care model
    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS teams (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            name        TEXT    NOT NULL,
            color       TEXT    NOT NULL DEFAULT '#607D8B',
            manager_id  INTEGER REFERENCES users(id)
        );
    ")?;
    let has_team_id: i64 = conn.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('staff') WHERE name='team_id'",
        [], |r| r.get(0),
    ).unwrap_or(0);
    if has_team_id == 0 {
        conn.execute("ALTER TABLE staff ADD COLUMN team_id INTEGER REFERENCES teams(id)", [])?;
    }

    Ok(())
}
