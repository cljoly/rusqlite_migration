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

use std::{iter::FromIterator, num::NonZeroUsize};

use rusqlite::Connection;

use crate::{Migrations, MigrationsBuilder, SchemaVersion, M};

#[test]
#[should_panic]
fn test_non_existing_index() {
    let ms = vec![M::up("CREATE TABLE t(a);")];

    let _ = MigrationsBuilder::from_iter(ms.clone()).edit(100, move |m| m);
}

#[test]
#[should_panic]
fn test_0_index() {
    let ms = vec![M::up("CREATE TABLE t(a);")];

    let _ = MigrationsBuilder::from_iter(ms).edit(0, move |m| m);
}

#[test]
fn test_valid_index() {
    let ms = vec![M::up("CREATE TABLE t1(a);"), M::up("CREATE TABLE t2(a);")];

    insta::assert_debug_snapshot!(MigrationsBuilder::from_iter(ms)
        .edit(1, move |m| m.down("DROP TABLE t1;"))
        .edit(2, move |m| m.down("DROP TABLE t2;"))
        .finalize());
}

#[test]
fn test_len_builder() {
    let mut conn = Connection::open_in_memory().unwrap();
    // Define migrations
    let ms = vec![
        M::up("CREATE TABLE friend(name TEXT);"),
        M::up("ALTER TABLE friend ADD COLUMN birthday TEXT;"),
    ];

    {
        let builder = MigrationsBuilder::from_iter(ms);

        let migrations: Migrations = builder.finalize();

        migrations.to_latest(&mut conn).unwrap();

        insta::assert_debug_snapshot!(migrations);
        assert_eq!(migrations.ms.len(), 2);
        assert_eq!(
            Ok(SchemaVersion::Inside(NonZeroUsize::new(2).unwrap())),
            migrations.current_version(&conn)
        );
    }
}
