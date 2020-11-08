use rusqlite::{params, Connection};
use rusqlite_migration::{Migrations, M};

pub fn main() {
    let mut conn = Connection::open("./my_db.db3").unwrap();
    // Define migrations
    let migrations = Migrations::new(vec![
        M::up(include_str!("friend_car.sql")),
        M::up("ALTER TABLE friend ADD COLUMN birthday TEXT;"),
        // In the future, if the need to change the schema arises, put
        // migrations here, like so:
        // M::up("CREATE INDEX UX_friend_email ON friend(email);"),
        // M::up("CREATE INDEX UX_friend_name ON friend(name);"),
    ]);
    // Update the database schema
    migrations.latest(&mut conn).unwrap();

    // Use the db 🥳
    conn.execute(
        "INSERT INTO friend (name, birthday) VALUES (?1, ?2)",
        params!["John", "1970-01-01"],
    )
    .unwrap();
}
