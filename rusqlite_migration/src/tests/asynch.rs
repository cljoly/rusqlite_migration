use crate::{AsyncMigrations, Error, MigrationDefinitionError, M};
use tokio_rusqlite::Connection as AsyncConnection;

fn m_valid0() -> M<'static> {
    M::up("CREATE TABLE m1(a, b); CREATE TABLE m2(a, b, c);")
}
fn m_valid10() -> M<'static> {
    M::up("CREATE TABLE t1(a, b);")
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
