use std::{iter::FromIterator, num::NonZeroUsize};

use rusqlite::Connection;

use crate::{Migrations, MigrationsBuilder, SchemaVersion, M};

#[test]
#[should_panic]
fn test_non_existing_index() {
    let ms = vec![M::up("CREATE TABLE t(a);")];

    let _ = MigrationsBuilder::from_iter(ms.clone()).edit(100, move |t| t);
}

#[test]
#[should_panic]
fn test_0_index() {
    let ms = vec![M::up("CREATE TABLE t(a);")];

    let _ = MigrationsBuilder::from_iter(ms).edit(0, move |t| t);
}

#[test]
fn test_len_builder() {
    let mut conn = Connection::open_in_memory().unwrap();
    // Define migrations
    let ms = vec![
        M::up("CREATE TABLE friend(name TEXT);"),
        M::up("ALTER TABLE friend ADD COLUMN birthday TEXT;"),
    ];

    {
        let builder = MigrationsBuilder::from_iter(ms);

        let migrations: Migrations = builder.finalize();

        migrations.to_latest(&mut conn).unwrap();

        assert_eq!(migrations.ms.len(), 2);
        assert_eq!(
            Ok(SchemaVersion::Inside(NonZeroUsize::new(2).unwrap())),
            migrations.current_version(&conn)
        );
    }
}
