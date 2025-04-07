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

use rusqlite::{params, Connection};
use rusqlite_migration::{Migrations, SchemaVersion, M};

#[test]
fn main_test() {
    let mut conn = Connection::open_in_memory().unwrap();
    // Define migrations
    let mut ms = vec![
        M::up("CREATE TABLE t(a);"),
        M::up(include_str!("../../examples/friend_car.sql")),
        M::up("ALTER TABLE friend ADD COLUMN birthday TEXT;"),
    ];

    {
        let migrations = Migrations::new(ms.clone().into());
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

    // Later, we add things to the schema
    ms.push(M::up("CREATE INDEX UX_friend_email ON friend(email);"));
    ms.push(M::up("ALTER TABLE friend RENAME COLUMN birthday TO birth;"));

    {
        let migrations = Migrations::new(ms.clone().into());
        migrations.to_latest(&mut conn).unwrap();

        assert_eq!(
            Ok(SchemaVersion::Inside(NonZeroUsize::new(5).unwrap())),
            migrations.current_version(&conn)
        );

        conn.execute(
            "INSERT INTO friend (name, birth) VALUES (?1, ?2)",
            params!["Alice", "2000-01-01"],
        )
        .unwrap();
    }

    // Later still
    ms.push(M::up("DROP INDEX UX_friend_email;"));

    {
        let migrations = Migrations::new(ms.clone().into());
        migrations.to_latest(&mut conn).unwrap();

        assert_eq!(
            Ok(SchemaVersion::Inside(NonZeroUsize::new(6).unwrap())),
            migrations.current_version(&conn)
        );

        conn.execute(
            "INSERT INTO friend (name, birth) VALUES (?1, ?2)",
            params!["Alice", "2000-01-01"],
        )
        .unwrap();
    }
}
