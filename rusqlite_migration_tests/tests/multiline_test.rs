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
    let db_file = mktemp::Temp::new_file().unwrap();
    // Define a multiline migration
    let mut ms = vec![M::up(
        r#"
              CREATE TABLE friend (name TEXT PRIMARY KEY, email TEXT) WITHOUT ROWID;
              ALTER TABLE friend ADD COLUMN birthday TEXT;
              ALTER TABLE friend ADD COLUMN notes TEXT;
              "#,
    )];

    {
        let mut conn = Connection::open(&db_file).unwrap();

        let migrations = Migrations::new(ms.clone());
        migrations.to_latest(&mut conn).unwrap();

        conn.pragma_update_and_check(None, "journal_mode", "WAL", |r| {
            match r.get::<_, String>(0) {
                Ok(v) if v.to_lowercase() == "wal" => Ok(()),
                val => panic!("unexpected journal_mode after setting it: {:?}", val),
            }
        })
        .unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();

        assert_eq!(
            Ok(SchemaVersion::Inside(NonZeroUsize::new(ms.len()).unwrap())),
            migrations.current_version(&conn)
        );

        conn.execute(
            "INSERT INTO friend (name, birthday, notes) VALUES (?1, ?2, ?3)",
            params!["John", "1970-01-01", "fun fact: ..."],
        )
        .unwrap();

        conn.query_row("SELECT * FROM pragma_journal_mode", [], |row| {
            assert_eq!(row.get::<_, String>(0), Ok(String::from("wal")));
            Ok(())
        })
        .unwrap();

        conn.query_row("SELECT * FROM pragma_foreign_keys", [], |row| {
            assert_eq!(row.get::<_, bool>(0), Ok(true));
            Ok(())
        })
        .unwrap();
    }

    // Using a new connection to ensure the pragma were taken into account
    {
        let conn = Connection::open(&db_file).unwrap();

        conn.query_row("SELECT * FROM pragma_journal_mode", [], |row| {
            assert_eq!(row.get::<_, String>(0), Ok(String::from("wal")));
            Ok(())
        })
        .unwrap();

        conn.execute(
            "INSERT INTO friend (name, birthday) VALUES (?1, ?2)",
            params!["Anna", "1971-11-11"],
        )
        .unwrap();
    }

    // Later, we add things to the schema
    ms.push(M::up("CREATE INDEX UX_friend_email ON friend(email)"));
    ms.push(M::up("ALTER TABLE friend RENAME COLUMN birthday TO birth;"));

    {
        let mut conn = Connection::open(&db_file).unwrap();

        let migrations = Migrations::new(ms.clone());
        migrations.to_latest(&mut conn).unwrap();

        assert_eq!(
            Ok(SchemaVersion::Inside(NonZeroUsize::new(3).unwrap())),
            migrations.current_version(&conn)
        );

        conn.execute(
            "INSERT INTO friend (name, birth) VALUES (?1, ?2)",
            params!["Alice", "2000-01-01"],
        )
        .unwrap();
    }
}
