use rusqlite::{params, Connection};
use rusqlite_migration::{Migrations, SchemaVersion, M};

#[test]
fn main_test() {
    simple_logging::log_to_stderr(log::LevelFilter::Trace);

    // Define a multiline migrations
    let ms = vec![
        // 0
        M::up("CREATE TABLE animals (id INTEGER, name TEXT);").down("DROP TABLE animals;"),
        // 1
        M::up("CREATE TABLE food (id INTEGER, name TEXT);").down("DROP TABLE food;"),
        // 2
        M::up("ALTER TABLE animals ADD COLUMN food_id INTEGER;")
            .down("ALTER TABLE animals DROP COLUMN food_id;"),
        // 3
        M::up("CREATE TABLE habitats (id INTEGER, name TEXT);").down("DROP TABLE habitats;"),
        // 4
        M::up("ALTER TABLE animals ADD COLUMN habitat_id INTEGER;")
            .down("ALTER TABLE animals DROP COLUMN habitat_id;"),
        // 5
    ];

    {
        let mut conn = Connection::open_in_memory().unwrap();

        let migrations = Migrations::new(ms.clone());

        assert_eq!(
            Ok(SchemaVersion::NoneSet),
            migrations.current_version(&conn)
        );

        migrations.to_version(&mut conn, 1).unwrap();

        conn.execute("INSERT INTO animals (name) VALUES (?1)", params!["Dog"])
            .unwrap();

        assert_eq!(
            Ok(SchemaVersion::Inside(0 /* this is off by one */)),
            migrations.current_version(&conn)
        );

        // go back
        migrations.to_version(&mut conn, 0).unwrap();

        // the table is gone now
        let _ = conn
            .execute("INSERT INTO animals (name) VALUES (?1)", params!["Dog"])
            .unwrap_err();

        assert_eq!(
            Ok(SchemaVersion::NoneSet),
            migrations.current_version(&conn)
        );
    }

    // Multiple steps
    {
        let mut conn = Connection::open_in_memory().unwrap();

        let migrations = Migrations::new(ms.clone());

        assert_eq!(
            Ok(SchemaVersion::NoneSet),
            migrations.current_version(&conn)
        );

        // Bad version, this should not change the DB
        assert!(migrations.to_version(&mut conn, 6).is_err());

        assert_eq!(
            Ok(SchemaVersion::NoneSet),
            migrations.current_version(&conn)
        );

        migrations.to_version(&mut conn, 5).unwrap();

        conn.execute(
            "INSERT INTO habitats (id, name) VALUES (?1, ?2)",
            params![0, "Forest"],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO animals (name, habitat_id) VALUES (?1, ?2)",
            params!["Fox", 0],
        )
        .unwrap();

        // go back
        migrations.to_version(&mut conn, 3).unwrap();

        // the table is gone now
        assert!(conn
            .execute("INSERT INTO habitats (name) VALUES (?1)", params!["Beach"],)
            .is_err());

        conn.execute(
            "INSERT INTO food (id, name) VALUES (?1, ?2)",
            params![0, "Cheese"],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO animals (name, food_id) VALUES (?1, ?2)",
            params!["Mouse", 0],
        )
        .unwrap();
    }
}

#[test]
fn test_errors() {
    simple_logging::log_to_stderr(log::LevelFilter::Trace);

    // Define a multiline migrations
    let ms = vec![
        // 0
        M::up("CREATE TABLE animals (id INTEGER, name TEXT);").down("DROP TABLE animals;"),
        // 1
        M::up("CREATE TABLE food (id INTEGER, name TEXT);"), // no down!!!
        // 2
        M::up("ALTER TABLE animals ADD COLUMN food_id INTEGER;")
            .down("ALTER TABLE animals DROP COLUMN food_id;"),
        // 3
    ];

    {
        let mut conn = Connection::open_in_memory().unwrap();

        let migrations = Migrations::new(ms.clone());

        migrations.latest(&mut conn).unwrap();

        conn.execute("INSERT INTO animals (name) VALUES (?1)", params!["Dog"])
            .unwrap();

        // go back
        assert!(migrations.to_version(&mut conn, 0)
            .is_err()); // oops

        assert_eq!(
            Ok(SchemaVersion::Inside(2 /* off by one */)),
            migrations.current_version(&conn)
        );

        // one is fine
        assert!(migrations.to_version(&mut conn, 2)
            .is_ok());

        // boom
        assert!(migrations.to_version(&mut conn, 1)
            .is_err()); // oops
    }
}
