use anyhow::Result;
use lazy_static::lazy_static;
use rusqlite::params;
use rusqlite_migration::{AsyncMigrations, M};
use tokio_rusqlite::Connection;

// Test that migrations are working
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn migrations_test() {
        assert!(MIGRATIONS.validate().await.is_ok());
    }
}

// Define migrations. These are applied atomically.
lazy_static! {
    static ref MIGRATIONS: AsyncMigrations =
        AsyncMigrations::new(vec![
            M::up(include_str!("../../friend_car.sql")),
            // PRAGMA are better applied outside of migrations, see below for details.
            M::up(r#"
                  ALTER TABLE friend ADD COLUMN birthday TEXT;
                  ALTER TABLE friend ADD COLUMN comment TEXT;
                  "#),

            // This migration can be reverted
            M::up("CREATE TABLE animal(name TEXT);")
            .down("DROP TABLE animal;")

            // In the future, if the need to change the schema arises, put
            // migrations here, like so:
            // M::up("CREATE INDEX UX_friend_email ON friend(email);"),
            // M::up("CREATE INDEX UX_friend_name ON friend(name);"),
        ]);
}

pub async fn init_db() -> Result<Connection> {
    let mut async_conn = Connection::open("./my_db.db3").await?;

    // Update the database schema, atomically
    MIGRATIONS.to_latest(&mut async_conn).await?;

    Ok(async_conn)
}

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("trace")).init();

    let mut async_conn = init_db().await.unwrap();

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

    // If we want to revert the last migration
    MIGRATIONS.to_version(&mut async_conn, 2).await.unwrap();

    // The table was removed
    async_conn
        .call(|conn| Ok(conn.execute("INSERT INTO animal (name) VALUES (?1)", params!["cat"])))
        .await
        .unwrap_err();
}
