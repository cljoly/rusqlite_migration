use iai::black_box;
use rusqlite::Connection;
use rusqlite_migration::{Migrations, M};

// Why criterion and iai? It’s actually recommended:
// https://bheisler.github.io/criterion.rs/book/iai/comparison.html

fn upward(i: u64) {
    let sql_migrations = (0..=i)
        .map(|i| {
            (
                format!("CREATE TABLE t{}(a, b, c);", i),
                format!("DROP TABLE t{};", i),
            )
        })
        .collect::<Vec<_>>();
    let migrations =
        Migrations::new_iter(sql_migrations.iter().enumerate().map(|(i, (up, down))| {
            let m = M::up(up).down(down);
            if i % 500 == 0 {
                m.foreign_key_check()
            } else {
                m
            }
        }));

    let mut conn = Connection::open_in_memory().unwrap();

    migrations.to_latest(&mut conn).unwrap();
}

fn upward_migration_short() {
    upward(black_box(10))
}

fn upward_migration_long() {
    upward(black_box(100))
}

iai::main!(upward_migration_short, upward_migration_long);
