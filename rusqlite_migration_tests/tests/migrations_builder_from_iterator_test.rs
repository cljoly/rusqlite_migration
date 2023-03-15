use std::{iter::FromIterator, num::NonZeroUsize};

use rusqlite::{params, Connection, Transaction};
use rusqlite_migration::{Migrations, MigrationsBuilder, SchemaVersion, M};

#[test]
fn main_test() {
    let mut conn = Connection::open_in_memory().unwrap();
    // Define migrations
    let ms = vec![
        M::up("CREATE TABLE t(a);"),
        M::up(include_str!("../../examples/friend_car.sql")),
        M::up("ALTER TABLE friend ADD COLUMN birthday TEXT;"),
    ];

    {
        let builder = MigrationsBuilder::from_iter(ms);

        let migrations: Migrations = builder
            .edit(1, move |m| m.set_down_hook(move |_tx: &Transaction| Ok(())))
            .edit(1, move |m| m.set_up_hook(move |_tx: &Transaction| Ok(())))
            .finalize();

        migrations.to_latest(&mut conn).unwrap();

        assert_eq!(
            Ok(SchemaVersion::Inside(NonZeroUsize::new(3).unwrap())),
            migrations.current_version(&conn)
        );

        conn.execute(
            "INSERT INTO friend (name, birthday) VALUES (?1, ?2)",
            params!["John", "1970-01-01"],
        )
        .unwrap();
    }
}
