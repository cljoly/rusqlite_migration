use super::*;

fn m_valid0() -> M<'static> {
    M::up("CREATE TABLE m1(a, b); CREATE TABLE m2(a, b, c);")
}
fn m_valid10() -> M<'static> {
    M::up("CREATE TABLE t1(a, b);")
}
fn m_valid11() -> M<'static> {
    M::up("ALTER TABLE t1 RENAME COLUMN b TO c;")
}
fn m_valid20() -> M<'static> {
    M::up("CREATE TABLE t2(b);")
}
fn m_valid21() -> M<'static> {
    M::up("ALTER TABLE t2 ADD COLUMN a;")
}

fn m_valid_fk() -> M<'static> {
    M::up(
        "CREATE TABLE fk1(a PRIMARY KEY); \
        CREATE TABLE fk2( \
            a, \
            FOREIGN KEY(a) REFERENCES fk1(a) \
        ); \
        INSERT INTO fk1 (a) VALUES ('foo'); \
        INSERT INTO fk2 (a) VALUES ('foo'); \
    ",
    )
    .foreign_key_check()
}

// All valid Ms in the right order
fn all_valid() -> Vec<M<'static>> {
    vec![
        m_valid0(),
        m_valid10(),
        m_valid11(),
        m_valid20(),
        m_valid21(),
        m_valid_fk(),
    ]
}

fn m_invalid0() -> M<'static> {
    M::up("CREATE TABLE table3()")
}
fn m_invalid1() -> M<'static> {
    M::up("something invalid")
}

fn m_invalid_fk() -> M<'static> {
    M::up(
        "CREATE TABLE fk1(a PRIMARY KEY); \
        CREATE TABLE fk2( \
            a, \
            FOREIGN KEY(a) REFERENCES fk1(a) \
        ); \
        INSERT INTO fk2 (a) VALUES ('foo'); \
    ",
    )
    .foreign_key_check()
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
fn user_version_convert_test() {
    let mut conn = Connection::open_in_memory().unwrap();
    let migrations = Migrations::new(vec![m_valid10()]);
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
    let migrations = Migrations::new(vec![m_valid10()]);

    assert_eq!(Ok(0), user_version(&conn));

    assert_eq!(Ok(()), migrations.to_latest(&mut conn));
    assert_eq!(Ok(1), user_version(&conn));
    assert_eq!(
        Ok(SchemaVersion::Inside(NonZeroUsize::new(1).unwrap())),
        migrations.current_version(&conn)
    );

    let migrations = Migrations::new(vec![m_valid10(), m_valid11()]);
    assert_eq!(Ok(()), migrations.to_latest(&mut conn));
    assert_eq!(Ok(2), user_version(&conn));
    assert_eq!(
        Ok(SchemaVersion::Inside(NonZeroUsize::new(2).unwrap())),
        migrations.current_version(&conn)
    );
}

#[test]
fn user_version_start_0_test() {
    let conn = Connection::open_in_memory().unwrap();
    assert_eq!(Ok(0), user_version(&conn))
}

#[test]
fn invalid_migration_statement_test() {
    for m in &[m_invalid0(), m_invalid1(), m_valid11(), m_valid21()] {
        let migrations = Migrations::new(vec![m.clone()]);
        assert_ne!(Ok(()), migrations.validate())
    }
}

#[test]
fn invalid_migration_multiple_statement_test() {
    let migrations = Migrations::new(vec![m_valid0(), m_invalid1()]);
    assert!(match dbg!(migrations.validate()) {
        Err(Error::RusqliteError { query: _, err: _ }) => true,
        _ => false,
    })
}

#[test]
fn valid_migration_multiple_statement_test() {
    for m in &[m_valid0(), m_valid10(), m_valid20()] {
        let migrations = Migrations::new(vec![m.clone()]);
        assert_eq!(Ok(()), migrations.validate())
    }
}

#[test]
fn valid_fk_check_test() {
    assert_eq!(Ok(()), Migrations::new(vec![m_valid_fk()]).validate())
}

#[test]
fn invalid_fk_check_test() {
    let migrations = Migrations::new(vec![m_invalid_fk()]);

    assert!(match dbg!(migrations.validate()) {
        Err(Error::ForeignKeyCheck(_)) => true,
        _ => false,
    })
}

#[test]
fn all_valid_test() {
    assert_eq!(Ok(()), Migrations::new(all_valid()).validate())
}

// If we encounter a database with a migration number higher than the number of defined migration,
// we should return an error, not panic.
// See https://github.com/cljoly/rusqlite_migration/issues/17
#[test]
fn current_version_gt_max_schema_version_test() {
    let mut conn = Connection::open_in_memory().unwrap();

    // Set migrations to a higher number
    {
        let migrations = Migrations::new(vec![m_valid0(), m_valid10()]);
        migrations.to_latest(&mut conn).unwrap();
    }

    // We now have less migrations
    let migrations = Migrations::new(vec![m_valid0()]);

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
