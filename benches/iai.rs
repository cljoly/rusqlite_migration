use iai::black_box;
use rusqlite::Connection;
use rusqlite_migration::{Migrations, M};

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

fn iai_benchmark_short() {
    upward(black_box(10))
}

fn iai_benchmark_long() {
    upward(black_box(100))
}

iai::main!(iai_benchmark_short, iai_benchmark_long);
