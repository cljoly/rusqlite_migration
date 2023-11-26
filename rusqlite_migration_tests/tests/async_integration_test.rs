use std::num::NonZeroUsize;

use rusqlite::params;
use rusqlite_migration::{AsyncMigrations, SchemaVersion, M};
use tokio_rusqlite::Connection;

#[tokio::test]
async fn main_test() {
    let mut conn = Connection::open_in_memory().await.unwrap();
    // Define migrations
    let mut ms = vec![
        M::up("CREATE TABLE t(a);"),
        M::up(include_str!("../../examples/friend_car.sql")),
        M::up("ALTER TABLE friend ADD COLUMN birthday TEXT;"),
    ];

    {
        let migrations = AsyncMigrations::new(ms.clone());
        migrations.to_latest(&mut conn).await.unwrap();

        assert_eq!(
            Ok(SchemaVersion::Inside(NonZeroUsize::new(3).unwrap())),
            migrations.current_version(&conn).await
        );

        conn.call_unwrap(|c| {
            c.execute(
                "INSERT INTO friend (name, birthday) VALUES (?1, ?2)",
                params!["John", "1970-01-01"],
            )
        })
        .await
        .unwrap();
    }

    // Later, we add things to the schema
    ms.push(M::up("CREATE INDEX UX_friend_email ON friend(email);"));
    ms.push(M::up("ALTER TABLE friend RENAME COLUMN birthday TO birth;"));

    {
        let migrations = AsyncMigrations::new(ms.clone());
        migrations.to_latest(&mut conn).await.unwrap();

        assert_eq!(
            Ok(SchemaVersion::Inside(NonZeroUsize::new(5).unwrap())),
            migrations.current_version(&conn).await
        );

        conn.call_unwrap(|c| {
            c.execute(
                "INSERT INTO friend (name, birth) VALUES (?1, ?2)",
                params!["Alice", "2000-01-01"],
            )
        })
        .await
        .unwrap();
    }

    // Later still
    ms.push(M::up("DROP INDEX UX_friend_email;"));

    {
        let migrations = AsyncMigrations::new(ms.clone());
        migrations.to_latest(&mut conn).await.unwrap();

        assert_eq!(
            Ok(SchemaVersion::Inside(NonZeroUsize::new(6).unwrap())),
            migrations.current_version(&conn).await
        );

        conn.call_unwrap(|c| {
            c.execute(
                "INSERT INTO friend (name, birth) VALUES (?1, ?2)",
                params!["Alice", "2000-01-01"],
            )
        })
        .await
        .unwrap();
    }
}
