use std::num::NonZeroUsize;

use rusqlite::{params, Connection};
use rusqlite_migration::{Migrations, SchemaVersion, M};

const NO_PARAMS: &[&dyn rusqlite::ToSql] = &[];

#[test]
fn main_test() {
    simple_logging::log_to_stderr(log::LevelFilter::Trace);

    let db_file = mktemp::Temp::new_file().unwrap();
    // Define a multiline migration
    let mut ms = vec![M::up(
        r#"
              CREATE TABLE friend (name TEXT PRIMARY KEY, email TEXT) WITHOUT ROWID;
              PRAGMA journal_mode = WAL;
              PRAGMA foreign_keys = ON;
              ALTER TABLE friend ADD COLUMN birthday TEXT;
              "#,
    )];

    {
        let mut conn = Connection::open(&db_file).unwrap();

        let migrations = Migrations::new(ms.clone());
        migrations.to_latest(&mut conn).unwrap();

        conn.pragma_update(None, "journal_mode", &"WAL").unwrap();
        conn.pragma_update(None, "foreign_keys", &"ON").unwrap();

        assert_eq!(
            Ok(SchemaVersion::Inside(NonZeroUsize::new(ms.len()).unwrap())),
            migrations.current_version(&conn)
        );

        conn.execute(
            "INSERT INTO friend (name, birthday) VALUES (?1, ?2)",
            params!["John", "1970-01-01"],
        )
        .unwrap();

        conn.query_row(
            "SELECT * FROM pragma_journal_mode",
            #[allow(deprecated)] // To keep compatibility with lower rusqlite versions
            NO_PARAMS,
            |row| {
                assert_eq!(row.get::<_, String>(0), Ok(String::from("wal")));
                Ok(())
            },
        )
        .unwrap();

        conn.query_row(
            "SELECT * FROM pragma_foreign_keys",
            #[allow(deprecated)] // To keep compatibility with lower rusqlite versions
            NO_PARAMS,
            |row| {
                assert_eq!(row.get::<_, bool>(0), Ok(true));
                Ok(())
            },
        )
        .unwrap();
    }

    // Using a new connection to ensure the pragma were taken into account
    {
        let conn = Connection::open(&db_file).unwrap();

        conn.query_row(
            "SELECT * FROM pragma_journal_mode",
            #[allow(deprecated)] // To keep compatibility with lower rusqlite versions
            NO_PARAMS,
            |row| {
                assert_eq!(row.get::<_, String>(0), Ok(String::from("wal")));
                Ok(())
            },
        )
        .unwrap();

        conn.execute(
            "INSERT INTO friend (name, birthday) VALUES (?1, ?2)",
            params!["Anna", "1971-11-11"],
        )
        .unwrap();
    }

    // Later, we add things to the schema
    ms.push(M::up("CREATE INDEX UX_friend_email ON friend(email)"));
    ms.push(M::up("ALTER TABLE friend RENAME COLUMN birthday TO birth;"));

    {
        let mut conn = Connection::open(&db_file).unwrap();

        let migrations = Migrations::new(ms.clone());
        migrations.to_latest(&mut conn).unwrap();

        assert_eq!(
            Ok(SchemaVersion::Inside(NonZeroUsize::new(3).unwrap())),
            migrations.current_version(&conn)
        );

        conn.execute(
            "INSERT INTO friend (name, birth) VALUES (?1, ?2)",
            params!["Alice", "2000-01-01"],
        )
        .unwrap();
    }
}
