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

use std::borrow::Cow;

use anyhow::Result;
use rusqlite::{params, Connection};
use rusqlite_migration::{Migrations, M};

// Test that migrations are working
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_test() {
        assert!(MIGRATIONS.validate().is_ok());
    }
}

// Define migrations. These are applied atomically.
const MIGRATION_ARRAY: &[M] = &[
    M::up(include_str!("../../friend_car.sql")),
    // PRAGMA are better applied outside of migrations, see below for details.
    M::up(
        r#"
        ALTER TABLE friend ADD COLUMN birthday TEXT;
        ALTER TABLE friend ADD COLUMN comment TEXT;
        "#,
    ),
    // This migration can be reverted
    M::up("CREATE TABLE animal(name TEXT);").down("DROP TABLE animal;"),
    // In the future, if the need to change the schema arises, put
    // migrations here, like so:
    // M::up("CREATE INDEX UX_friend_email ON friend(email);"),
    // M::up("CREATE INDEX UX_friend_name ON friend(name);"),
];
const MIGRATIONS: Migrations = Migrations::new(Cow::Borrowed(MIGRATION_ARRAY));

pub fn init_db() -> Result<Connection> {
    let mut conn = Connection::open("./my_db.db3")?;

    // Update the database schema, atomically
    MIGRATIONS.to_latest(&mut conn)?;

    Ok(conn)
}

pub fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("trace")).init();

    let mut conn = init_db().unwrap();

    // Apply some PRAGMA. These are often better applied outside of migrations, as some needs to be
    // executed for each connection (like `foreign_keys`) or to be executed outside transactions
    // (`journal_mode` is a noop in a transaction).
    conn.pragma_update(None, "journal_mode", "WAL").unwrap();
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();

    // Use the db 🥳
    conn.execute(
        "INSERT INTO friend (name, birthday) VALUES (?1, ?2)",
        params!["John", "1970-01-01"],
    )
    .unwrap();

    conn.execute("INSERT INTO animal (name) VALUES (?1)", params!["dog"])
        .unwrap();

    // If we want to revert the last migration
    MIGRATIONS.to_version(&mut conn, 2).unwrap();

    // The table was removed
    conn.execute("INSERT INTO animal (name) VALUES (?1)", params!["cat"])
        .unwrap_err();
}
