use rusqlite::{params, Connection};
use rusqlite_migration::{Migrations, SchemaVersion, M};

#[test]
fn main_test() {
    simple_logging::log_to_stderr(log::LevelFilter::Trace);

    let mut conn = Connection::open_in_memory().unwrap();
    // Define migrations
    let mut ms = vec![
        M::up("PRAGMA journal_mode=WAL;"),
        M::up(include_str!("../examples/friend_car.sql")),
        M::up("ALTER TABLE friend ADD COLUMN birthday TEXT;"),
    ];

    {
        let migrations = Migrations::new(ms.clone());
        migrations.latest(&mut conn).unwrap();

        assert_eq!(
            Ok(SchemaVersion::Inside(2)),
            migrations.current_version(&conn)
        );

        conn.execute(
            "INSERT INTO friend (name, birthday) VALUES (?1, ?2)",
            params!["John", "1970-01-01"],
        )
        .unwrap();
    }

    // Later, we add things to the schema
    ms.push(M::up("CREATE INDEX UX_friend_email ON friend(email);"));
    ms.push(M::up("ALTER TABLE friend RENAME COLUMN birthday TO birth;"));

    {
        let migrations = Migrations::new(ms.clone());
        migrations.latest(&mut conn).unwrap();

        assert_eq!(
            Ok(SchemaVersion::Inside(4)),
            migrations.current_version(&conn)
        );

        conn.execute(
            "INSERT INTO friend (name, birth) VALUES (?1, ?2)",
            params!["Alice", "2000-01-01"],
        )
        .unwrap();
    }

    // Later still
    ms.push(M::up("DROP INDEX UX_friend_email;"));

    {
        let migrations = Migrations::new(ms.clone());
        migrations.latest(&mut conn).unwrap();

        assert_eq!(
            Ok(SchemaVersion::Inside(5)),
            migrations.current_version(&conn)
        );

        conn.execute(
            "INSERT INTO friend (name, birth) VALUES (?1, ?2)",
            params!["Alice", "2000-01-01"],
        )
        .unwrap();
    }
}
