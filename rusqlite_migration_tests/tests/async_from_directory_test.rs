use std::num::NonZeroUsize;

use include_dir::{include_dir, Dir};

use rusqlite::params;
use rusqlite_migration::{AsyncMigrations, SchemaVersion};
use tokio_rusqlite::Connection;

static MIGRATIONS_DIR: Dir =
    include_dir!("$CARGO_MANIFEST_DIR/../examples/from-directory/migrations");

#[tokio::test]
async fn main_test() {
    let mut conn = Connection::open_in_memory().await.unwrap();
    {
        let migrations = AsyncMigrations::from_directory(&MIGRATIONS_DIR).unwrap();

        migrations.to_latest(&mut conn).await.unwrap();

        assert_eq!(
            Ok(SchemaVersion::Inside(NonZeroUsize::new(3).unwrap())),
            migrations.current_version(&conn).await
        );

        conn.call(|conn| {
            conn.execute(
                "INSERT INTO friend (name, birthday) VALUES (?1, ?2)",
                params!["John", "1970-01-01"],
            )
            .unwrap();
            Ok(())
        })
        .await
        .unwrap();
    }
}
