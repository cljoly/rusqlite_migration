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

// All valid Ms in the right order
fn all_valid() -> Vec<M<'static>> {
    vec![
        m_valid0(),
        m_valid10(),
        m_valid11(),
        m_valid20(),
        m_valid21(),
    ]
}

fn m_invalid0() -> M<'static> {
    M::up("CREATE TABLE table3()")
}
fn m_invalid1() -> M<'static> {
    M::up("something invalid")
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
fn all_valid_test() {
    assert_eq!(Ok(()), Migrations::new(all_valid()).validate())
}
