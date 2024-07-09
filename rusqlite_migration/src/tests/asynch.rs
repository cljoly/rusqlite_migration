use std::iter::FromIterator;

use crate::{
    tests::helpers::{
        all_valid, m_invalid0, m_invalid1, m_invalid_down_fk, m_invalid_fk, m_valid0, m_valid10,
        m_valid11, m_valid20, m_valid21, m_valid_fk,
    },
    AsyncMigrations, Error, MigrationDefinitionError
};
use tokio_rusqlite::Connection as AsyncConnection;

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
async fn invalid_down_fk_check_test() {
    let migrations = AsyncMigrations::new(vec![m_invalid_down_fk()]);

    let mut conn = AsyncConnection::open_in_memory().await.unwrap();
    migrations.to_latest(&mut conn).await.unwrap();

    assert!(matches!(
        dbg!(migrations.to_version(&mut conn, 0).await),
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

        // Before to_latest.
        assert_eq!(2, migrations.latest_schema_version());
        assert_eq!(Ok(false), migrations.is_latest_schema_version(&conn).await);

        migrations.to_latest(&mut conn).await.unwrap();

        // After to_latest
        assert_eq!(
            2_usize,
            migrations.current_version(&conn).await.unwrap().into()
        );
        assert_eq!(Ok(true), migrations.is_latest_schema_version(&conn).await);
    }

    // We now have fewer migrations
    let migrations = AsyncMigrations::new(vec![m_valid0()]);
    assert_eq!(1, migrations.latest_schema_version());
    assert_eq!(
        2_usize,
        migrations.current_version(&conn).await.unwrap().into()
    );

    assert_eq!(Ok(false), migrations.is_latest_schema_version(&conn).await);

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

#[tokio::test]
async fn test_tokio_rusqlite_conversion() {
    use tokio_rusqlite::Error as TError;

    insta::assert_debug_snapshot!(
        "convert_connection_closed_error",
        crate::Error::from(TError::ConnectionClosed)
    );
    insta::assert_debug_snapshot!(
        "convert_rusqlite_error",
        crate::Error::from(TError::Rusqlite(rusqlite::Error::InvalidQuery))
    );
}
