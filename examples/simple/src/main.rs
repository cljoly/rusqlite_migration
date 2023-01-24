use anyhow::Result;
use lazy_static::lazy_static;
use rusqlite::{params, Connection};
use rusqlite_migration::{Migrations, M};

// Test that migrations are working
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_test() {
        assert!(MIGRATIONS.validate().is_ok());
    }
}

// Define migrations. These are applied atomically.
lazy_static! {
    static ref MIGRATIONS: Migrations<'static> =
        Migrations::new(vec![
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
