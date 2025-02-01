use std::sync::LazyLock;

use anyhow::Result;
use rusqlite::params;
use rusqlite_migration::{Migrations, M};
use tokio_rusqlite::Connection;

/// The general idea with this example is to use [`Connection::call`][call] and
/// [`Connection::call_unwrap`][call_unwrap] to run the migration in a sync context.
///
/// [call]: https://docs.rs/tokio-rusqlite/0.6.0/tokio_rusqlite/struct.Connection.html#method.call
/// [call_unwrap]: https://docs.rs/tokio-rusqlite/0.6.0/tokio_rusqlite/struct.Connection.html#method.call_unwrap

// Test that migrations are working
#[cfg(test)]
mod tests {
    use super::*;

    // Validating that migrations are correctly defined. It is enough to test in the sync context,
    // because under the hood, tokio_rusqlite executes the migrations in a sync context anyway.
    #[test]
    fn migrations_test() {
        assert!(MIGRATIONS.validate().is_ok());
    }
}

// Define migrations. These are applied atomically.
static MIGRATIONS: LazyLock<Migrations> = LazyLock::new(|| {
    Migrations::new(vec![
        M::up(include_str!("../../friend_car.sql")),
        // PRAGMA are better applied outside of migrations, see below for details.
        M::up(
            r#"
                  ALTER TABLE friend ADD COLUMN birthday TEXT;
                  ALTER TABLE friend ADD COLUMN comment TEXT;
                  "#,
        ),
        // This migration can be reverted
        M::up("CREATE TABLE animal(name TEXT);").down("DROP TABLE animal;"),
        // In the future, if the need to change the schema arises, put
        // migrations here, like so:
        // M::up("CREATE INDEX UX_friend_email ON friend(email);"),
        // M::up("CREATE INDEX UX_friend_name ON friend(name);"),
    ])
});

pub async fn init_db() -> Result<Connection> {
    let async_conn = Connection::open("./my_db.db3").await?;

    // Update the database schema, atomically
    // Using `call_unwrap` is appropriate because we have just opened the connection before and
    // this function still returns any error returned inside of it. In other words, according to
    // the [documentation][docs], it only panics if there is an issue with the connection, not if
    // an error is returned by the closure.
    //
    // [docs]: https://docs.rs/tokio-rusqlite/0.6.0/tokio_rusqlite/struct.Connection.html#method.call_unwrap
    async_conn
        .call_unwrap(|conn| MIGRATIONS.to_latest(conn))
        .await?;

    Ok(async_conn)
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("trace")).init();

    let async_conn = init_db().await.unwrap();

    // Apply some PRAGMA. These are often better applied outside of migrations, as some needs to be
    // executed for each connection (like `foreign_keys`) or to be executed outside transactions
    // (`journal_mode` is a noop in a transaction).
    async_conn
        .call(|conn| Ok(conn.pragma_update(None, "journal_mode", "WAL")))
        .await
        .unwrap()
        .unwrap();
    async_conn
        .call(|conn| Ok(conn.pragma_update(None, "foreign_keys", "ON")))
        .await
        .unwrap()
        .unwrap();

    // Use the db 🥳
    async_conn
        .call(|conn| {
            Ok(conn.execute(
                "INSERT INTO friend (name, birthday) VALUES (?1, ?2)",
                params!["John", "1970-01-01"],
            ))
        })
        .await
        .unwrap()
        .unwrap();

    async_conn
        .call(|conn| Ok(conn.execute("INSERT INTO animal (name) VALUES (?1)", params!["dog"])))
        .await
        .unwrap()
        .unwrap();

    // We can revert to the last migration
    // Let’s also demonstrate using `Connection::call` instead of `Connection::call_unwrap`.
    // Notice how in effect, we have handle two `Result`s, one for the errors happening when we try
    // to use the connection and the other one when applying the migration proper.
    async_conn
        .call(|conn| Ok(MIGRATIONS.to_version(conn, 2)))
        .await??;

    // The table was removed
    async_conn
        .call(|conn| Ok(conn.execute("INSERT INTO animal (name) VALUES (?1)", params!["cat"])))
        .await
        .unwrap_err();

    Ok(())
}
