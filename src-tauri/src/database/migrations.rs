use rusqlite::{Connection, params};

use crate::core::error::{AppError, AppResult};

/// Ordered list of embedded migrations. Never edit an applied migration: add a new one.
const MIGRATIONS: &[(i64, &str)] = &[
    (1, include_str!("../../migrations/0001_initial.sql")),
    (2, include_str!("../../migrations/0002_attachments.sql")),
];

pub fn latest_version() -> i64 {
    MIGRATIONS.last().map_or(0, |(version, _)| *version)
}

pub fn apply(conn: &mut Connection) -> AppResult<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
             version    INTEGER PRIMARY KEY,
             applied_at INTEGER NOT NULL
         );",
    )?;

    let current: i64 = conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
        [],
        |row| row.get(0),
    )?;

    // A database written by a newer app version must not be touched by an older one.
    if current > latest_version() {
        return Err(AppError::UnsupportedVersion);
    }

    for (version, sql) in MIGRATIONS.iter().filter(|(v, _)| *v > current) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.execute(
            "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, ?2)",
            params![version, crate::core::time::now_millis()],
        )?;
        tx.commit()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn version(conn: &Connection) -> i64 {
        conn.query_row("SELECT MAX(version) FROM schema_migrations", [], |r| {
            r.get(0)
        })
        .unwrap()
    }

    #[test]
    fn applies_all_migrations_once() {
        let mut conn = Connection::open_in_memory().unwrap();
        apply(&mut conn).unwrap();
        apply(&mut conn).unwrap();
        assert_eq!(version(&conn), latest_version());

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, MIGRATIONS.len() as i64);
    }

    #[test]
    fn refuses_databases_from_the_future() {
        let mut conn = Connection::open_in_memory().unwrap();
        apply(&mut conn).unwrap();
        conn.execute(
            "INSERT INTO schema_migrations (version, applied_at) VALUES (999, 0)",
            [],
        )
        .unwrap();
        assert!(matches!(
            apply(&mut conn),
            Err(AppError::UnsupportedVersion)
        ));
    }
}
