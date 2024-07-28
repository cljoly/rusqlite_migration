use std::sync::LazyLock;

use anyhow::Result;
use include_dir::{include_dir, Dir};
use rusqlite::{params, Connection};
use rusqlite_migration::Migrations;

static MIGRATIONS_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/migrations");

// Define migrations. These are applied atomically.
static MIGRATIONS: LazyLock<Migrations<'static>> =
    LazyLock::new(|| Migrations::from_directory(&MIGRATIONS_DIR).unwrap());

pub fn init_db() -> Result<Connection> {
    let mut conn = Connection::open("./my_db.db3")?;

    // Update the database schema, atomically
    MIGRATIONS.to_latest(&mut conn)?;

    Ok(conn)
}

pub fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("trace")).init();

    let mut conn = init_db().unwrap();

    // Apply some PRAGMA. These are often better applied outside of migrations, as some needs to be
    // executed for each connection (like `foreign_keys`) or to be executed outside transactions
    // (`journal_mode` is a noop in a transaction).
    conn.pragma_update(None, "journal_mode", "WAL").unwrap();
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();

    // Use the db 🥳
    conn.execute(
        "INSERT INTO friend (name, birthday) VALUES (?1, ?2)",
        params!["John", "1970-01-01"],
    )
    .unwrap();

    conn.execute("INSERT INTO animal (name) VALUES (?1)", params!["dog"])
        .unwrap();

    // If we want to revert the last migration
    MIGRATIONS.to_version(&mut conn, 2).unwrap();

    // The table was removed
    conn.execute("INSERT INTO animal (name) VALUES (?1)", params!["cat"])
        .unwrap_err();
}

// Test that migrations are working
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_test() {
        assert!(MIGRATIONS.validate().is_ok());
    }
}
