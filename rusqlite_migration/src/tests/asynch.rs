use std::iter::FromIterator;

use crate::{AsyncMigrations, Error, MigrationDefinitionError, M};
use tokio_rusqlite::Connection as AsyncConnection;

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

#[tokio::test]
async fn invalid_migration_statement_test() {
    for m in &[m_invalid0(), m_invalid1(), m_valid11(), m_valid21()] {
        let migrations = AsyncMigrations::new(vec![m.clone()]);
        assert_ne!(Ok(()), migrations.validate().await)
    }
}

#[tokio::test]
async fn invalid_migration_multiple_statement_test() {
    let migrations = AsyncMigrations::new(vec![m_valid0(), m_invalid1()]);
    assert!(matches!(
        dbg!(migrations.validate().await),
        Err(Error::RusqliteError { query: _, err: _ })
    ));
}

#[tokio::test]
async fn valid_migration_multiple_statement_test() {
    for m in &[m_valid0(), m_valid10(), m_valid20()] {
        let migrations = AsyncMigrations::new(vec![m.clone()]);
        assert_eq!(Ok(()), migrations.validate().await)
    }
}

#[tokio::test]
async fn valid_fk_check_test() {
    assert_eq!(
        Ok(()),
        AsyncMigrations::new(vec![m_valid_fk()]).validate().await
    )
}

#[tokio::test]
async fn invalid_fk_check_test() {
    let migrations = AsyncMigrations::new(vec![m_invalid_fk()]);
    assert!(matches!(
        dbg!(migrations.validate().await),
        Err(Error::ForeignKeyCheck(_))
    ));
}

#[tokio::test]
async fn all_valid_test() {
    assert_eq!(Ok(()), AsyncMigrations::new(all_valid()).validate().await)
}

#[tokio::test]
async fn current_version_gt_max_schema_version_async_test() {
    let mut conn = AsyncConnection::open_in_memory().await.unwrap();

    // Set migrations to a higher number
    {
        let migrations = AsyncMigrations::new(vec![m_valid0(), m_valid10()]);
        migrations.to_latest(&mut conn).await.unwrap();
    }

    // We now have less migrations
    let migrations = AsyncMigrations::new(vec![m_valid0()]);

    // We should get an error
    assert_eq!(
        migrations.to_latest(&mut conn).await,
        Err(Error::MigrationDefinition(
            MigrationDefinitionError::DatabaseTooFarAhead
        ))
    );
}

#[tokio::test]
async fn empty_migrations_test() {
    let mut conn = AsyncConnection::open_in_memory().await.unwrap();
    let m = AsyncMigrations::new(vec![]);

    assert_eq!(
        Err(Error::MigrationDefinition(
            MigrationDefinitionError::NoMigrationsDefined
        )),
        m.to_latest(&mut conn).await
    );

    for v in 0..4 {
        assert_eq!(
            Err(Error::MigrationDefinition(
                MigrationDefinitionError::NoMigrationsDefined
            )),
            m.to_version(&mut conn, v).await
        )
    }
}

#[tokio::test]
async fn test_from_iter() {
    let migrations = AsyncMigrations::from_iter(vec![m_valid0(), m_valid10()]);
    assert_eq!(Ok(()), migrations.validate().await);
}
