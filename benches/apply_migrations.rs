/* Copyright 2022 Clément Joly

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.

*/

use rusqlite::Connection;
use rusqlite_migration::{Migrations, M};

use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};

fn migrations_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("Upward migrations");
    group.sample_size(1000);
    for i in [10usize, 30, 60, 100, 500] {
        group.bench_with_input(BenchmarkId::new("upward", i), &i, |b, _| {
            let sql_migrations = (0..=i)
                .map(|i| format!("CREATE TABLE t{}(a, b, c)", i))
                .collect::<Vec<_>>();
            let migrations = Migrations::new_iter(sql_migrations.iter().map(|sql| M::up(sql)));

            b.iter_batched(
                || Connection::open_in_memory().unwrap(),
                |mut conn| {
                    // We need to hold this for the benchmark to be valid, but we don’t need to check
                    // it if we haven’t changed the code
                    // assert_eq!(
                    // rusqlite_migration::SchemaVersion::NoneSet,
                    // migrations.current_version(&conn).unwrap()
                    // );
                    migrations.to_latest(&mut conn).unwrap();
                },
                BatchSize::SmallInput,
            )
        });
    }
    group.finish()
}

criterion_group!(benches, migrations_benchmark);
criterion_main!(benches);
