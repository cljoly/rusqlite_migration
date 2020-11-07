/* Copyright 2020 Clément Joly

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
use rusqlite::NO_PARAMS;

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn empty_migrations_test() {
        let conn = Connection::open_in_memory().unwrap();
        let _ = Migrations::new(&conn, &[]);
    }
    // TODO
    // Error in the middle of  a migration (right version number + proper errro)
    // Test function for SQL statements that panics & don’t panic

    #[test]
    fn user_version_start_0_test() {
        let conn = Connection::open_in_memory().unwrap();

        {
            let migrations = Migrations::new(&conn, &[]);
            assert_eq!(0, migrations.user_version())
        }

        {
            let migrations = Migrations::new(&conn, &[M::up("something valid")]);
            assert_eq!(0, migrations.user_version())
        }
    }

    #[test]
    fn invalid_migration_statement_test() {
        let conn = Connection::open_in_memory().unwrap();
        let migrations = Migrations::new(&conn, &[M::up("something valid")]);
        assert_eq!(Ok(0), migrations.latest())
    }
}

// TODO Remove unwrap and handle errors

/// One migration
pub struct M<'u> {
    up: &'u str,
    // TODO Maybe later
    // down: Option<&'s2 str>,
}

impl<'u> M<'u> {
    // Into string? Up required?
    pub fn up(sql: &'u str) -> Self {
        Self { up: sql }
    }

    // Optional, not implemented
    pub fn down(self, sql: &str) -> Self {
        unimplemented!()
    }
}

/// Set of migrations
pub struct Migrations<'c> {
    conn: &'c Connection,
}

impl<'c> Migrations<'c> {
    pub fn new(conn: &'c Connection, schemas: &[M]) -> Self {
        Self { conn }
    }

    // Is it useful to have this public?
    fn user_version(&self) -> i64 {
        self.conn
            .query_row("PRAGMA user_version", NO_PARAMS, |row| row.get(0))
            .unwrap()
    }

    fn set_user_version(&self, v: i64) {
        self.conn.pragma_update(None, "user_version", &v).unwrap()
    }

    /// Go to a given schema version
    pub fn goto(version: i64) {
        unimplemented!()
    }

    /// Get current schema version

    /// Go to latest schema version. Returns the schema number on success
    pub fn latest(&self) -> Result<i64, ()> {
        unimplemented!()
    }
}

/// Build one by one to test
/// Try to compile and run each statement
/// Run upward and downward migrations
pub fn validate_all(migrations: &Migrations) -> Result<i64, ()> {
    unimplemented!()
}
