use std::num::NonZeroUsize;

use rusqlite::{params, Connection};
use rusqlite_migration::{MigrationDefinitionError, Migrations, SchemaVersion, M};

#[test]
fn main_test() {
    simple_logging::log_to_stderr(log::LevelFilter::Trace);

    let ms = vec![
        // 0
        M::up("CREATE TABLE animals (id INTEGER PRIMARY KEY, name TEXT);")
            .down("DROP TABLE animals;"),
        // 1
        M::up("CREATE TABLE food (id INTEGER PRIMARY KEY, name TEXT);").down("DROP TABLE food;"),
        // 2
        M::up("CREATE TABLE animal_food (animal_id INTEGER, food_id INTEGER);")
            .down("DROP TABLE animal_food;"),
        // 3
        M::up("CREATE TABLE habitats (id INTEGER PRIMARY KEY, name TEXT);")
            .down("DROP TABLE habitats;"),
        // 4
        M::up("CREATE TABLE animal_habitat (animal_id INTEGER, habitat_id INTEGER);")
            .down("DROP TABLE animal_habitat;"),
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
            Ok(SchemaVersion::Inside(NonZeroUsize::new(1).unwrap())),
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
            "INSERT INTO animals (id, name) VALUES (?1, ?2)",
            params![15, "Fox"],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO animal_habitat (animal_id, habitat_id) VALUES (?1, ?2)",
            params![15, 0],
        )
        .unwrap();

        // go back
        migrations.to_version(&mut conn, 3).unwrap();

        // the table is gone now
        assert!(conn
            .execute("INSERT INTO habitats (name) VALUES (?1)", params!["Beach"],)
            .is_err());

        conn.execute("INSERT INTO food (name) VALUES (?1)", params!["Cheese"])
            .unwrap();

        conn.execute("INSERT INTO animals (name) VALUES (?1)", params!["Mouse"])
            .unwrap();

        conn.execute(
            "INSERT INTO animal_food (animal_id, food_id) VALUES (?1, ?2)",
            params![1, 0],
        )
        .unwrap();
    }
}

#[test]
fn test_errors() {
    simple_logging::log_to_stderr(log::LevelFilter::Trace);

    let ms = vec![
        // 0
        M::up("CREATE TABLE animals (id INTEGER, name TEXT);").down("DROP TABLE animals;"),
        // 1
        M::up("CREATE TABLE food (id INTEGER, name TEXT);"), // no down!!!
        // 2
        M::up("CREATE TABLE animal_food (animal_id INTEGER, food_id INTEGER);")
            .down("DROP TABLE animal_food;"),
    ];

    {
        let mut conn = Connection::open_in_memory().unwrap();

        let migrations = Migrations::new(ms.clone());

        migrations.to_latest(&mut conn).unwrap();

        assert_eq!(
            Ok(SchemaVersion::Inside(NonZeroUsize::new(3).unwrap())),
            migrations.current_version(&conn)
        );

        conn.execute("INSERT INTO animals (name) VALUES (?1)", params!["Dog"])
            .unwrap();

        // go back
        assert!(migrations.to_version(&mut conn, 0).is_err()); // oops

        assert_eq!(
            Ok(SchemaVersion::Inside(NonZeroUsize::new(3).unwrap())),
            migrations.current_version(&conn)
        );

        // one is fine
        assert!(migrations.to_version(&mut conn, 2).is_ok());

        // boom
        assert_eq!(
            Err(rusqlite_migration::Error::MigrationDefinition(
                MigrationDefinitionError::DownNotDefined { migration_index: 1 }
            )),
            migrations.to_version(&mut conn, 1)
        );
    }
}
