// SPDX-License-Identifier: Apache-2.0
// Copyright Clément Joly and contributors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//    http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Why criterion and iai? It’s actually recommended:
//! https://bheisler.github.io/criterion.rs/book/iai/comparison.html

use std::iter::FromIterator;

use iai::black_box;
use rusqlite::Connection;
use rusqlite_migration::{Migrations, M};

fn upward(i: u64) {
    let sql_migrations = (0..=i)
        .map(|i| {
            (
                format!("CREATE TABLE t{i}(a, b, c);"),
                format!("DROP TABLE t{i};"),
            )
        })
        .collect::<Vec<_>>();
    let migrations =
        Migrations::from_iter(sql_migrations.iter().enumerate().map(|(i, (up, down))| {
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

#[cfg(feature = "from-directory")]
mod from_directory;

#[cfg(feature = "from-directory")]
fn from_directory_small() {
    from_directory::small()
}

#[cfg(not(feature = "from-directory"))]
fn from_directory_small() {
    ()
}

#[cfg(feature = "from-directory")]
fn from_directory_big() {
    from_directory::big()
}

#[cfg(not(feature = "from-directory"))]
fn from_directory_big() {
    ()
}

iai::main!(
    upward_migration_short,
    upward_migration_long,
    from_directory_small,
    from_directory_big,
);
