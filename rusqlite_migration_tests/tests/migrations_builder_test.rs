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

use std::num::NonZeroUsize;

use include_dir::{include_dir, Dir};

use rusqlite::{params, Connection, Transaction};
use rusqlite_migration::{Migrations, MigrationsBuilder, SchemaVersion};

static MIGRATIONS_DIR: Dir =
    include_dir!("$CARGO_MANIFEST_DIR/../examples/from-directory/migrations");

#[test]
fn main_test() {
    let mut conn = Connection::open_in_memory().unwrap();
    // Define migrations

    {
        let builder = MigrationsBuilder::from_directory(&MIGRATIONS_DIR).unwrap();

        let migrations: Migrations = builder
            .edit(1, move |m| m.set_down_hook(move |_tx: &Transaction| Ok(())))
            .edit(1, move |m| m.set_up_hook(move |_tx: &Transaction| Ok(())))
            .finalize();

        migrations.to_latest(&mut conn).unwrap();

        assert_eq!(
            Ok(SchemaVersion::Inside(NonZeroUsize::new(3).unwrap())),
            migrations.current_version(&conn)
        );

        conn.execute(
            "INSERT INTO friend (name, birthday) VALUES (?1, ?2)",
            params!["John", "1970-01-01"],
        )
        .unwrap();
    }
}
