use anyhow::Result;
use env_logger;
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

// Define migrations
lazy_static! {
    static ref MIGRATIONS: Migrations<'static> = {
        Migrations::new(vec![
            M::up(include_str!("friend_car.sql")),
            M::up("ALTER TABLE friend ADD COLUMN birthday TEXT;"),
            // In the future, if the need to change the schema arises, put
            // migrations here, like so:
            // M::up("CREATE INDEX UX_friend_email ON friend(email);"),
            // M::up("CREATE INDEX UX_friend_name ON friend(name);"),
        ])
    };
}

pub fn init_db() -> Result<Connection> {
    let mut conn = Connection::open("./my_db.db3")?;

    // Update the database schema
    (*MIGRATIONS).latest(&mut conn)?;

    Ok(conn)
}

pub fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("trace")).init();

    let conn = init_db().unwrap();

    // Use the db 🥳
    conn.execute(
        "INSERT INTO friend (name, birthday) VALUES (?1, ?2)",
        params!["John", "1970-01-01"],
    )
    .unwrap();

    // Rest of the program
}
