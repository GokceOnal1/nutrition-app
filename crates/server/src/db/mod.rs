//! Connection pool setup and schema migrations. Repository functions (one
//! module per aggregate) land here starting in M1.

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions};
use std::str::FromStr;

/// Connects to the configured SQLite database, creating the file if it does
/// not exist yet, and enables WAL mode per `CLAUDE.md`'s stack notes.
pub async fn connect(database_url: &str) -> Result<SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::from_str(database_url)?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal);
    SqlitePoolOptions::new().connect_with(options).await
}

/// Runs our own schema migrations (`crates/server/migrations/`). This is
/// separate from `tower-sessions-sqlx-store`'s own `.migrate()` call, which
/// manages its session table independently.
pub async fn migrate(pool: &SqlitePool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("./migrations").run(pool).await
}
