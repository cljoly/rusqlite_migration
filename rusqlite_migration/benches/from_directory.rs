/* Copyright 2023 Clément Joly

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

use criterion::{criterion_group, Criterion};
use include_dir::{include_dir, Dir};

use rusqlite_migration::Migrations;

static MIGRATIONS_DIR_10: Dir = include_dir!("$CARGO_MANIFEST_DIR/benches/10_migrations");
static MIGRATIONS_DIR_100: Dir = include_dir!("$CARGO_MANIFEST_DIR/benches/100_migrations");

// Iai
#[allow(dead_code)]
pub fn small() {
    Migrations::from_directory(&MIGRATIONS_DIR_10).unwrap();
}

#[allow(dead_code)]
pub fn big() {
    Migrations::from_directory(&MIGRATIONS_DIR_100).unwrap();
}

// Criterion
#[allow(dead_code)]
pub fn create_bench(c: &mut Criterion) {
    c.bench_function("from_directory_small", |b| {
        b.iter_with_large_drop(|| Migrations::from_directory(&MIGRATIONS_DIR_10).unwrap())
    });

    c.bench_function("from_directory_big", |b| {
        b.iter_with_large_drop(|| Migrations::from_directory(&MIGRATIONS_DIR_100).unwrap())
    });
}

criterion_group!(create, create_bench);
