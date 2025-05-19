// SPDX-License-Identifier: Apache-2.0
// Copyright Clément Joly and contributors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//    http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::{iter::FromIterator, num::NonZeroUsize};

use rusqlite::{Connection, OpenFlags, Transaction};

use crate::tests::helpers::{all_valid_down, m_valid0_down, m_valid_fk_down};
use crate::{
    tests::helpers::{
        all_valid_up, m_invalid_fk, m_invalid_fk_down, m_valid0_up, m_valid10_up, m_valid11_up,
        m_valid_fk_up,
    },
    user_version, Error, MigrationDefinitionError, Migrations, SchemaVersion, SchemaVersionError,
    M,
};

use super::helpers::{m_invalid0, m_invalid1, m_valid20_up, m_valid21_up, raw_set_user_version};

#[test]
fn max_migration_test() {
    use crate::{set_user_version, user_version};

    let mut conn = Connection::open_in_memory().unwrap();
    let migrations_max = crate::MIGRATIONS_MAX;
    set_user_version(&conn, migrations_max).unwrap();
    assert_eq!(
        user_version(&conn),
        Ok(migrations_max),
        "Migration max is too high, it’s not the actual limit",
    );

    // Unfortunately SQLite fails silently. But the internal set_user_version returns an error.
    assert_eq!(
        set_user_version(&conn, migrations_max + 1),
        Err(Error::SpecifiedSchemaVersion(SchemaVersionError::TooHigh))
    );
    assert_eq!(
        user_version(&conn),
        Ok(migrations_max),
        "set_user_version returned an error but user_version was changed",
    );
    raw_set_user_version(&mut conn, migrations_max as isize + 1);
    assert_eq!(
        user_version(&conn),
        Ok(0),
        "Migration max is too low, it’s not the actual limit",
    );
}

// Weirdly, SQLite supports negative numbers, but let’s make sure we fail loudly in that case
#[test]
fn min_migrations_test() {
    let mut conn = Connection::open_in_memory().unwrap();

    crate::set_user_version(&conn, 0).unwrap();

    // The rest of the test also ascertain the behavior of SQLite (and rusqlite)

    raw_set_user_version(&mut conn, -3);
    assert_eq!(
        conn.query_row("PRAGMA user_version", [], |row| row.get(0)),
        Ok(-3),
    );
    assert_eq!(crate::user_version(&conn), Err(Error::InvalidUserVersion));

    // The minimum user version is a i32::MIN
    raw_set_user_version(&mut conn, i32::MIN as isize);
    assert_eq!(
        conn.query_row("PRAGMA user_version", [], |row| row.get(0)),
        Ok(i32::MIN),
    );
    assert_eq!(crate::user_version(&conn), Err(Error::InvalidUserVersion));

    // Anything lower than i32::MIN is silently replaced by sqlite
    raw_set_user_version(&mut conn, (i32::MIN as isize).checked_sub(1).unwrap());
    assert_eq!(
        conn.query_row("PRAGMA user_version", [], |row| row.get(0)),
        Ok(0),
    );
}

#[test]
fn empty_migrations_test() {
    let mut conn = Connection::open_in_memory().unwrap();
    let m = Migrations::new(vec![]);

    assert_eq!(
        Err(Error::MigrationDefinition(
            MigrationDefinitionError::NoMigrationsDefined
        )),
        m.to_latest(&mut conn)
    );

    for v in 0..4 {
        assert_eq!(
            Err(Error::MigrationDefinition(
                MigrationDefinitionError::NoMigrationsDefined
            )),
            m.to_version(&mut conn, v)
        )
    }
}

#[test]
fn test_db_version_to_schema_empty() {
    let m = Migrations::new(vec![]);

    assert_eq!(m.db_version_to_schema(0), SchemaVersion::NoneSet);
    assert_eq!(
        m.db_version_to_schema(1),
        SchemaVersion::Outside(NonZeroUsize::new(1).unwrap())
    );
    assert_eq!(
        m.db_version_to_schema(10),
        SchemaVersion::Outside(NonZeroUsize::new(10).unwrap())
    );
}

#[test]
fn test_db_version_to_schema_two() {
    let m = Migrations::new(vec![m_valid10_up(), m_valid11_up()]);

    assert_eq!(m.db_version_to_schema(0), SchemaVersion::NoneSet);
    assert_eq!(
        m.db_version_to_schema(1),
        SchemaVersion::Inside(NonZeroUsize::new(1).unwrap())
    );
    assert_eq!(
        m.db_version_to_schema(10),
        SchemaVersion::Outside(NonZeroUsize::new(10).unwrap())
    );
}

#[test]
fn schema_version_partial_cmp_test() {
    assert_eq!(SchemaVersion::NoneSet, SchemaVersion::NoneSet);
    assert_eq!(
        SchemaVersion::Inside(NonZeroUsize::new(1).unwrap()),
        SchemaVersion::Inside(NonZeroUsize::new(1).unwrap())
    );
    assert_eq!(
        SchemaVersion::Outside(NonZeroUsize::new(1).unwrap()),
        SchemaVersion::Outside(NonZeroUsize::new(1).unwrap())
    );
    assert_ne!(
        SchemaVersion::Outside(NonZeroUsize::new(1).unwrap()),
        SchemaVersion::Inside(NonZeroUsize::new(1).unwrap())
    );
    assert_ne!(
        SchemaVersion::Outside(NonZeroUsize::new(1).unwrap()),
        SchemaVersion::NoneSet
    );
    assert_ne!(
        SchemaVersion::Inside(NonZeroUsize::new(1).unwrap()),
        SchemaVersion::NoneSet
    );
    assert!(SchemaVersion::NoneSet < SchemaVersion::Inside(NonZeroUsize::new(1).unwrap()));
    assert!(SchemaVersion::NoneSet < SchemaVersion::Outside(NonZeroUsize::new(1).unwrap()));
    assert!(
        SchemaVersion::Inside(NonZeroUsize::new(1).unwrap())
            < SchemaVersion::Outside(NonZeroUsize::new(2).unwrap())
    );
    assert!(
        SchemaVersion::Outside(NonZeroUsize::new(1).unwrap())
            < SchemaVersion::Inside(NonZeroUsize::new(2).unwrap())
    );
}

#[test]
fn test_migration_hook_debug() {
    let m = M::up_with_hook("", |_: &Transaction| Ok(()));
    insta::assert_debug_snapshot!(m);
}

#[test]
fn user_version_convert_test() {
    let mut conn = Connection::open_in_memory().unwrap();
    let migrations = Migrations::new(vec![m_valid10_up()]);
    assert_eq!(Ok(()), migrations.to_latest(&mut conn));
    assert_eq!(Ok(1), user_version(&conn));
    assert_eq!(
        Ok(SchemaVersion::Inside(NonZeroUsize::new(1).unwrap())),
        migrations.current_version(&conn)
    );
    assert_eq!(1usize, migrations.current_version(&conn).unwrap().into());
}

#[test]
fn user_version_migrate_test() {
    let mut conn = Connection::open_in_memory().unwrap();
    let migrations = Migrations::new(vec![m_valid10_up()]);

    assert_eq!(Ok(0), user_version(&conn));

    assert_eq!(Ok(()), migrations.to_latest(&mut conn));
    assert_eq!(Ok(1), user_version(&conn));
    assert_eq!(
        Ok(SchemaVersion::Inside(NonZeroUsize::new(1).unwrap())),
        migrations.current_version(&conn)
    );

    let migrations = Migrations::new(vec![m_valid10_up(), m_valid11_up()]);
    assert_eq!(Ok(()), migrations.to_latest(&mut conn));
    assert_eq!(Ok(2), user_version(&conn));
    assert_eq!(
        Ok(SchemaVersion::Inside(NonZeroUsize::new(2).unwrap())),
        migrations.current_version(&conn)
    );
}

#[test]
fn migration_partial_eq_test() {
    let m1 = M::up("");
    let m2 = M::up("");
    let m3 = M::up("TEST");

    assert_eq!(m1, m2);
    assert_ne!(m1, m3);
}

#[test]
fn user_version_start_0_test() {
    let conn = Connection::open_in_memory().unwrap();
    assert_eq!(Ok(0), user_version(&conn))
}

#[test]
fn invalid_migration_statement_test() {
    for m in &[m_invalid0(), m_invalid1(), m_valid11_up(), m_valid21_up()] {
        let migrations = Migrations::new(vec![m.clone()]);
        assert_ne!(Ok(()), migrations.validate())
    }
}

#[test]
fn invalid_migration_multiple_statement_test() {
    let migrations = Migrations::new(vec![m_valid0_up(), m_invalid1()]);
    assert!(matches!(
        dbg!(migrations.validate()),
        Err(Error::RusqliteError { query: _, err: _ })
    ));
}

#[test]
fn valid_migration_multiple_statement_test() {
    for m in &[m_valid0_up(), m_valid10_up(), m_valid20_up()] {
        let migrations = Migrations::new(vec![m.clone()]);
        assert_eq!(Ok(()), migrations.validate())
    }
}

#[test]
fn valid_fk_check_test() {
    assert_eq!(Ok(()), Migrations::new(vec![m_valid_fk_up()]).validate())
}

#[test]
fn invalid_fk_check_test() {
    let migrations = Migrations::new(vec![m_invalid_fk()]);
    insta::assert_debug_snapshot!(migrations.validate());
}

#[test]
fn invalid_down_fk_check_test() {
    let migrations = Migrations::new(vec![m_invalid_fk_down()]);

    let mut conn = Connection::open_in_memory().unwrap();
    migrations.to_latest(&mut conn).unwrap();

    assert!(matches!(
        dbg!(migrations.to_version(&mut conn, 0)),
        Err(Error::ForeignKeyCheck(_))
    ));
}

#[test]
fn all_valid_up_test() {
    let migrations = Migrations::new(all_valid_up());
    assert_eq!(Ok(()), migrations.validate());
    insta::assert_debug_snapshot!(migrations)
}

#[test]
fn all_valid_down_test() {
    let migrations = Migrations::new(all_valid_down());
    assert_eq!(Ok(()), migrations.validate());
    insta::assert_debug_snapshot!(migrations)
}

// When the DB encounters an error, it is surfaced
#[test]
fn test_read_only_db_all_valid() {
    let mut conn = Connection::open_in_memory_with_flags(OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let migrations = Migrations::new(all_valid_up());

    let e = migrations.to_latest(&mut conn);

    assert!(e.is_err());
    insta::assert_debug_snapshot!(e)
}

// If we encounter a database with a migration number higher than the number of defined migration,
// we should return an error, not panic.
// See https://github.com/cljoly/rusqlite_migration/issues/17
#[test]
fn current_version_gt_max_schema_version_test() {
    let mut conn = Connection::open_in_memory().unwrap();

    // Set migrations to a higher number
    {
        let migrations = Migrations::new(vec![m_valid0_up(), m_valid10_up()]);
        migrations.to_latest(&mut conn).unwrap();
    }

    // We now have fewer migrations
    let migrations = Migrations::new(vec![m_valid0_up()]);

    // We should get an error
    assert_eq!(
        migrations.to_latest(&mut conn),
        Err(Error::MigrationDefinition(
            MigrationDefinitionError::DatabaseTooFarAhead
        ))
    );
}

#[test]
fn hook_test() {
    let mut conn = Connection::open_in_memory().unwrap();

    let text = "Lorem ipsum dolor sit amet, consectetur adipisici elit …".to_string();
    let cloned = text.clone();

    let migrations = Migrations::new(vec![
        M::up_with_hook(
            "CREATE TABLE novels (text TEXT);",
            move |tx: &Transaction| {
                tx.execute("INSERT INTO novels (text) VALUES (?1)", (&cloned,))?;
                Ok(())
            },
        ),
        M::up_with_hook(
            "ALTER TABLE novels ADD compressed TEXT;",
            |tx: &Transaction| {
                let mut stmt = tx.prepare("SELECT rowid, text FROM novels").unwrap();
                let rows = stmt.query_map([], |row| {
                    Ok((row.get_unwrap::<_, i64>(0), row.get_unwrap::<_, String>(1)))
                })?;

                for row in rows {
                    let row = row.unwrap();
                    let rowid = row.0;
                    let text = row.1;
                    let compressed = &text[..text.len() / 2];
                    tx.execute(
                        "UPDATE novels SET compressed = ?1 WHERE rowid = ?2;",
                        rusqlite::params![compressed, rowid],
                    )?;
                }

                Ok(())
            },
        )
        .down_with_hook(
            "ALTER TABLE novels DROP COLUMN compressed",
            |_: &Transaction| Ok(()),
        ),
    ]);

    assert_eq!(Ok(()), migrations.to_version(&mut conn, 2));

    let result: (String, String) = conn
        .query_row(
            "SELECT text, compressed FROM novels WHERE rowid = 1",
            [],
            |row| Ok((row.get(0).unwrap(), row.get(1).unwrap())),
        )
        .unwrap();

    assert_eq!(result.0, text);
    assert!(text.starts_with(&result.1));

    assert_eq!(Ok(()), migrations.to_version(&mut conn, 1));
}

#[test]
fn eq_hook_test() {
    let vec_migrations = vec![
        M::up("CREATE TABLE novels (text TEXT);"),
        // Different up
        M::up("CREATE TABLE IF NOT EXISTS novels (text TEXT);"),
        // Same up, different down
        M::up("CREATE TABLE IF NOT EXISTS novels (text TEXT);").down("DROP TABLE novels;"),
        // Use hooks now
        M::up_with_hook(
            "ALTER TABLE novels ADD compressed TEXT;",
            |_: &Transaction| Ok(()),
        )
        .down_with_hook(
            "ALTER TABLE novels DROP COLUMN compressed",
            |_: &Transaction| Ok(()),
        ),
        // Same as above, but different closures
        M::up_with_hook(
            "ALTER TABLE novels ADD compressed TEXT;",
            |_: &Transaction| Ok(()),
        )
        .down_with_hook(
            "ALTER TABLE novels DROP COLUMN compressed",
            |_: &Transaction| Ok(()),
        ),
        // Only with down hooks
        M::up_with_hook(
            "ALTER TABLE novels ADD compressed TEXT;",
            |_: &Transaction| Ok(()),
        )
        .down_with_hook(
            "ALTER TABLE novels DROP COLUMN compressed",
            |_: &Transaction| Ok(()),
        ),
        // Same as above, the closure should be deemed different
        M::up_with_hook(
            "ALTER TABLE novels ADD compressed TEXT;",
            |_: &Transaction| Ok(()),
        )
        .down_with_hook(
            "ALTER TABLE novels DROP COLUMN compressed",
            |_: &Transaction| Ok(()),
        ),
    ];
    // When there are no hooks, migrations can be cloned and still be equal
    {
        let migrations = Migrations::from_iter(vec_migrations.clone().into_iter().take(2));

        assert_eq!(migrations, migrations.clone());
    }

    // Complementary checks that PartialEq works as expected. We use assert_{eq,ne} to make
    // debugging easier
    for i in 0..vec_migrations.len() {
        for j in 0..vec_migrations.len() {
            if i == j {
                assert_eq!(&vec_migrations[i], &vec_migrations[j]);
                continue;
            }
            assert_ne!(&vec_migrations[i], &vec_migrations[j]);
        }
    }
    assert_eq!(&vec_migrations[1], &vec_migrations[1]);
    assert_ne!(&vec_migrations[0], &vec_migrations[1]);
}

#[test]
fn test_from_iter() {
    let migrations = Migrations::from_iter(vec![m_valid0_up(), m_valid10_up()]);
    assert_eq!(Ok(()), migrations.validate());
}

#[test]
fn test_user_version_error() {
    // This will cause error because the DB is read only
    let conn = Connection::open_in_memory_with_flags(OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let e = crate::set_user_version(&conn, 1);

    assert!(e.is_err(), "{:?}", e);
    insta::assert_debug_snapshot!(e)
}

#[test]
fn test_missing_down_migration() {
    let mut conn = Connection::open_in_memory().unwrap();
    let ms = vec![
        M::up("CREATE TABLE t1(a)").down("DROP TABLE t1"),
        M::up("CREATE TABLE t2(a)").down("DROP TABLE t2"),
        M::up("CREATE TABLE t3(a)"),
        M::up("CREATE TABLE t4(a)").down("DROP TABLE t4"),
        M::up("CREATE TABLE t5(a)"),
    ];

    let m = Migrations::new(ms);
    m.to_version(&mut conn, 4).unwrap();

    m.to_version(&mut conn, 3).unwrap();
    assert_eq!(
        Err(Error::MigrationDefinition(
            MigrationDefinitionError::DownNotDefined { migration_index: 2 }
        )),
        m.to_version(&mut conn, 2)
    );
}

// We can build from a Cow type easily enough
#[test]
fn test_build_from_cow() {
    use std::borrow::Cow;

    let _ = Migrations::from_slice(&Cow::from(vec![m_valid0_up()]));
}

#[test]
fn test_pending_migrations() -> Result<(), Box<dyn std::error::Error>> {
    let ms = vec![
        m_valid0_up(),
        m_valid10_up(),
        m_valid11_up(),
        m_valid20_up(),
        m_valid21_up(),
        m_valid_fk_up(),
    ];
    let migrations_0 = Migrations::from_slice(&[]);
    let migrations_1 = Migrations::from_slice(&ms[..1]);
    let migrations_2 = Migrations::from_slice(&ms[..3]);
    let migrations_3 = Migrations::from_slice(&ms[..]);

    {
        let mut conn = Connection::open_in_memory()?;
        // Apply the first one
        migrations_1.to_latest(&mut conn)?;

        assert_eq!(migrations_0.pending_migrations(&conn), Ok(-1));
        assert_eq!(migrations_1.pending_migrations(&conn), Ok(0));
        assert_eq!(migrations_2.pending_migrations(&conn), Ok(2));
        assert_eq!(migrations_3.pending_migrations(&conn), Ok(5));
    }
    {
        let mut conn = Connection::open_in_memory()?;
        // Apply until the middle
        migrations_2.to_latest(&mut conn)?;

        assert_eq!(migrations_1.pending_migrations(&conn), Ok(-2));
        assert_eq!(migrations_2.pending_migrations(&conn), Ok(0));
        assert_eq!(migrations_3.pending_migrations(&conn), Ok(3));
    }
    {
        let mut conn = Connection::open_in_memory()?;
        // Apply until the last one
        migrations_3.to_latest(&mut conn)?;

        assert_eq!(migrations_0.pending_migrations(&conn), Ok(-6));
        assert_eq!(migrations_1.pending_migrations(&conn), Ok(-5));
        assert_eq!(migrations_2.pending_migrations(&conn), Ok(-3));
        assert_eq!(migrations_3.pending_migrations(&conn), Ok(0));
    }

    Ok(())
}

#[test]
fn test_pending_migrations_errors() -> Result<(), Box<dyn std::error::Error>> {
    let mut conn = Connection::open_in_memory()?;

    let migrations = Migrations::new(vec![m_valid0_up(), m_valid10_up()]);

    // If the database is somehow corrupted, this returns an error
    raw_set_user_version(&mut conn, -325);
    assert_eq!(
        migrations.pending_migrations(&conn),
        Err(Error::InvalidUserVersion)
    );

    Ok(())
}

#[test]
fn test_display() {
    insta::assert_snapshot!("up_only", m_valid0_up());
    insta::assert_snapshot!("up_only_alt", format!("{:#}", m_valid0_up()));

    insta::assert_snapshot!("up_down", m_valid0_down());
    insta::assert_snapshot!("up_down_alt", format!("{:#}", m_valid0_down()));

    insta::assert_snapshot!("up_down_fk", m_valid_fk_down());
    insta::assert_snapshot!("up_down_fk_alt", format!("{:#}", m_valid_fk_down()));

    let everything = M {
        up: "UP",
        up_hook: Some(Box::new(|_: &Transaction| Ok(()))),
        down: Some("DOWN"),
        down_hook: Some(Box::new(|_: &Transaction| Ok(()))),
        foreign_key_check: true,
        comment: Some("Comment, likely a filename in practice!"),
    };
    insta::assert_snapshot!("everything", everything);
    insta::assert_debug_snapshot!("everything_debug", everything);
    insta::assert_compact_debug_snapshot!("everything_compact_debug", everything);
    insta::assert_snapshot!("everything_alt", format!("{everything:#}"));
}
