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

use criterion::measurement::Measurement;
use criterion::BenchmarkGroup;
use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};
use criterion_perf_events::Perf;
use perfcnt::linux::{HardwareEventType, PerfCounterBuilderLinux};
use rusqlite::Connection;
use rusqlite_migration::{Migrations, M};

fn migrations_benchmark<Mes: Measurement>(c: &mut Criterion<Mes>) {
    let mut group = c.benchmark_group("Apply migrations");

    fn iter_batched_connections<Mes: Measurement, S, R>(
        group: &mut BenchmarkGroup<Mes>,
        description: &str,
        param: i32,
        more_setup: S,
        routine: R,
    ) where
        S: Fn(&mut Connection) + Copy,
        R: Fn(&mut Connection) + Copy,
    {
        group.bench_with_input(BenchmarkId::new(description, param), &param, |b, _| {
            b.iter_batched_ref(
                || {
                    let mut conn = Connection::open_in_memory().unwrap();
                    more_setup(&mut conn);
                    conn
                },
                routine,
                BatchSize::SmallInput,
            )
        });
    }

    for i in [10, 30, 100] {
        let sql_migrations = (0..=i)
            .map(|i| {
                (
                    format!("CREATE TABLE t{}(a, b, c);", i),
                    format!("DROP TABLE t{};", i),
                )
            })
            .collect::<Vec<_>>();
        let migrations = Migrations::new_iter(
            sql_migrations
                .iter()
                .map(|(up, down)| M::up(up).down(down).foreign_key_check()),
        );

        iter_batched_connections(
            &mut group,
            "upward only",
            i,
            |_| {},
            |conn| {
                // We need to hold this for the benchmark to be valid, but we don’t need to check
                // it if we haven’t changed the code
                // assert_eq!(
                // rusqlite_migration::SchemaVersion::NoneSet,
                // migrations.current_version(conn).unwrap()
                // );
                migrations.to_latest(conn).unwrap();
            },
        );
    }

    group.finish()
}

// See https://gz.github.io/rust-perfcnt/perfcnt/linux/enum.HardwareEventType.html

criterion_group!(
    name = benches;
    config = Criterion::default().with_measurement(
        Perf::new(PerfCounterBuilderLinux::from_hardware_event(HardwareEventType::Instructions))
    );
    targets = migrations_benchmark
);

criterion_main!(benches);
