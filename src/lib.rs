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

#![forbid(unsafe_code)]

//! Rusqlite Migration is a simple schema migration tool for [rusqlite](https://lib.rs/crates/rusqlite) using [user_version](https://sqlite.org/pragma.html#pragma_user_version) instead of an SQL table to maintain the current schema version.
//!
//! It aims for:
//! - **simplicity**: there is a set of SQL statements and you just append to it to change the schema,
//! - **performance**: no need to add a table to be parsed, the `user_version` field is at a fixed offset in the sqlite file format.
//!
//! ## Example
//!
//! ```
//! use anyhow::Result;
//! use env_logger;
//! use lazy_static::lazy_static;
//! use rusqlite::{params, Connection};
//! use rusqlite_migration::{Migrations, M};
//!
//! // Define migrations. These are applied atomically.
//! lazy_static! {
//!     static ref MIGRATIONS: Migrations<'static> =
//!         Migrations::new(vec![
//!             M::up(r#"
//!                 CREATE TABLE friend(
//!                     friend_id INTEGER PRIMARY KEY,
//!                     name TEXT NOT NULL,
//!                     email TEXT UNIQUE,
//!                     phone TEXT UNIQUE,
//!                     picture BLOB
//!                 );
//!    
//!                 CREATE TABLE car(
//!                     registration_plate TEXT PRIMARY KEY,
//!                     cost REAL NOT NULL,
//!                     bought_on TEXT NOT NULL
//!                 );
//!             "#),
//!             // PRAGMA are better applied outside of migrations, see below for details.
//!             M::up(r#"
//!                       ALTER TABLE friend ADD COLUMN birthday TEXT;
//!                       ALTER TABLE friend ADD COLUMN comment TEXT;
//!                   "#),
//!             // In the future, if the need to change the schema arises, put
//!             // migrations here, like so:
//!             // M::up("CREATE INDEX UX_friend_email ON friend(email);"),
//!             // M::up("CREATE INDEX UX_friend_name ON friend(name);"),
//!         ]);
//! }
//!
//! pub fn init_db() -> Result<Connection> {
//!     let mut conn = Connection::open("./my_db.db3")?;
//!
//!     // Update the database schema, atomically
//!     MIGRATIONS.to_latest(&mut conn)?;
//!
//!     Ok(conn)
//! }
//!
//! pub fn main() {
//!     env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("trace")).init();
//!
//!     let conn = init_db().unwrap();
//!
//!     // Apply some PRAGMA. These are often better applied outside of migrations, as some needs to be
//!     // executed for each connection (like `foreign_keys`) or to be executed outside transactions
//!     // (`journal_mode` is a noop in a transaction).
//!     conn.pragma_update(None, "journal_mode", &"WAL").unwrap();
//!     conn.pragma_update(None, "foreign_keys", &"ON").unwrap();
//!
//!     // Use the db 🥳
//!     conn.execute(
//!         "INSERT INTO friend (name, birthday) VALUES (?1, ?2)",
//!         params!["John", "1970-01-01"],
//!     )
//!     .unwrap();
//! }
//! ```
//!
//! To test that the migrations are working, you can add this to your other tests:
//!
//! ```
//!     #[test]
//!     fn migrations_test() {
//!         assert!(MIGRATIONS.validate().is_ok());
//!     }
//! ```
//!

//! Please see the [examples](https://github.com/cljoly/rusqlite_migrate/tree/master/examples) folder for more.

use log::{debug, info, trace, warn};
use rusqlite::Connection;
#[allow(deprecated)] // To keep compatibility with lower rusqlite versions
use rusqlite::NO_PARAMS;

mod errors;

#[cfg(test)]
mod tests;
pub use errors::{Error, MigrationDefinitionError, Result, SchemaVersionError};
use std::cmp::Ordering;

/// One migration
#[derive(Debug, PartialEq, Clone)]
pub struct M<'u> {
    up: &'u str,
    down: Option<&'u str>,
}

impl<'u> M<'u> {
    /// Create a schema update.
    ///
    /// # Please note
    ///
    /// * PRAGMA statements are discouraged here. They are often better applied outside of
    /// migrations, because:
    ///   * Some PRAGMA need to be executed for each connection (like `foreign_keys`).
    ///   * Some PRAGMA are no-op when executed inside transactions (that will be the case for the
    ///   SQL written in `up`) (like `journal_mode`).
    ///   * Multiple SQL commands contaning `PRAGMA` are [known not to
    ///   work](https://github.com/rusqlite/rusqlite/pull/794) with the `extra_check` feature of
    ///   rusqlite.
    /// * SQL commands should end with a “;”.
    pub fn up(sql: &'u str) -> Self {
        Self {
            up: sql,
            down: None,
        }
    }

    /// Define a down-migration. This SQL statement should exactly reverse the changes
    /// performed in `up()`.
    pub fn down(mut self, sql: &'u str) -> Self {
        self.down = Some(sql);
        self
    }
}

/// Schema version, in the context of Migrations
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum SchemaVersion {
    /// No schema version set
    NoneSet,
    /// The current version in the database is outside any migration defined
    Outside(usize),
    /// The current version in the database is inside the range of defined
    /// migrations
    Inside(usize),
}

impl From<SchemaVersion> for usize {
    /// Translate schema version to db version
    fn from(schema_version: SchemaVersion) -> usize {
        match schema_version {
            SchemaVersion::NoneSet => 0,
            SchemaVersion::Inside(v) | SchemaVersion::Outside(v) => v + 1,
        }
    }
}

/// Set of migrations
#[derive(Debug, PartialEq, Clone)]
pub struct Migrations<'m> {
    ms: Vec<M<'m>>,
}

impl<'m> Migrations<'m> {
    /// Create a set of migrations.
    pub fn new(ms: Vec<M<'m>>) -> Self {
        Self { ms }
    }

    /// Performs allocations transparently.
    pub fn new_iter<I: IntoIterator<Item = M<'m>>>(ms: I) -> Self {
        use std::iter::FromIterator;
        Self::new(Vec::from_iter(ms))
    }

    fn db_version_to_schema(&self, db_version: usize) -> SchemaVersion {
        match db_version {
            0 => SchemaVersion::NoneSet,
            v if v > 0 && v <= self.ms.len() => SchemaVersion::Inside(v - 1),
            v => SchemaVersion::Outside(v - 1),
        }
    }

    /// Get current schema version
    pub fn current_version(&self, conn: &Connection) -> Result<SchemaVersion> {
        user_version(conn)
            .map(|v| self.db_version_to_schema(v))
            .map_err(|e| e.into())
    }

    /// Migrate upward methods. This is rolled back on error.
    /// On success, returns the number of update performed
    /// All versions are db versions
    fn goto_up(
        &self,
        conn: &mut Connection,
        current_version: usize,
        target_version: usize,
    ) -> Result<()> {
        debug_assert!(current_version <= target_version);
        debug_assert!(target_version <= self.ms.len());

        trace!("start migration transaction");
        let tx = conn.transaction()?;
        for v in current_version..target_version {
            let m = &self.ms[v];
            debug!("Running: {}", m.up);

            tx.execute_batch(m.up)
                .map_err(|e| Error::with_sql(e, m.up))?;
        }
        set_user_version(&tx, target_version)?;
        tx.commit()?;
        trace!("commited migration transaction");

        Ok(())
    }

    /// Migrate downward. This is rolled back on error.
    /// All versions are db versions
    fn goto_down(
        &self,
        conn: &mut Connection,
        current_version: usize,
        target_version: usize,
    ) -> Result<()> {
        debug_assert!(current_version >= target_version);
        debug_assert!(target_version <= self.ms.len());

        // First, check if all the migrations have a "down" version
        if let Some((i, bad_m)) = self
            .ms
            .iter()
            .enumerate()
            .skip(target_version)
            .take(current_version - target_version)
            .find(|(_, m)| m.down.is_none())
        {
            warn!("Cannot revert: {:?}", bad_m);
            return Err(Error::MigrationDefinition(
                MigrationDefinitionError::DownNotDefined { migration_index: i },
            ));
        }

        trace!("start migration transaction");
        let tx = conn.transaction()?;
        for v in (target_version..current_version).rev() {
            let m = &self.ms[v];
            if let Some(ref down) = m.down {
                debug!("Running: {}", down);
                tx.execute_batch(down)
                    .map_err(|e| Error::with_sql(e, down))?;
            } else {
                unreachable!();
            }
        }
        set_user_version(&tx, target_version)?;
        tx.commit()?;
        trace!("committed migration transaction");
        Ok(())
    }

    /// Go to a given db version
    fn goto(&self, conn: &mut Connection, target_db_version: usize) -> Result<()> {
        let current_version = user_version(conn)?;

        let res = match target_db_version.cmp(&current_version) {
            Ordering::Less => {
                debug!(
                    "rollback to older version requested, target_db_version: {}, current_version: {}",
                    target_db_version, current_version
                );
                self.goto_down(conn, current_version, target_db_version)
            }
            Ordering::Equal => {
                debug!("no migration to run, db already up to date");
                return Ok(()); // return directly, so the migration message is not printed
            }
            Ordering::Greater => {
                debug!(
                    "some migrations to run, target_db_version: {}, current_version: {}",
                    target_db_version, current_version
                );
                self.goto_up(conn, current_version, target_db_version)
            }
        };

        if res.is_ok() {
            info!("Database migrated to version {}", target_db_version);
        }
        res
    }

    /// Maximum version defined in the migration set
    fn max_schema_version(&self) -> SchemaVersion {
        let len = self.ms.len();
        if len == 0 {
            SchemaVersion::NoneSet
        } else {
            SchemaVersion::Inside(len - 1)
        }
    }

    #[deprecated(since = "0.4", note = "This was renammed to to_latest")]
    pub fn latest(&self, conn: &mut Connection) -> Result<()> {
        return self.to_latest(conn);
    }

    /// Migrate the database to latest schema version. The migrations are applied atomically.
    pub fn to_latest(&self, conn: &mut Connection) -> Result<()> {
        let v_max = self.max_schema_version();
        match v_max {
            SchemaVersion::NoneSet => {
                warn!("no migration defined");
                Ok(())
            }
            SchemaVersion::Inside(_) => {
                debug!("some migrations defined, try to migrate");
                self.goto(conn, v_max.into())
            }
            SchemaVersion::Outside(_) => unreachable!(),
        }
    }

    /// Migrate the database to a given schema version. The migrations are applied atomically.
    ///
    /// # Version numbering
    ///
    /// - Empty database (no migrations run yet) has version `0`.
    /// - The version increases after each migration, so after the first migration has run, the schema version is `1`.
    /// - If there are 3 migrations, version 3 is after all migrations have run.
    ///
    /// # Errors
    ///
    /// Attempts to migrate to a higher version than is supported will result in an error.
    ///
    /// When migrating downwards, all the reversed migrations must have a `.down()` variant,
    /// otherwise no migrations are run and the function returns an error.
    pub fn to_version(&self, conn: &mut Connection, version: usize) -> Result<()> {
        let v_max = self.max_schema_version();
        match v_max {
            SchemaVersion::NoneSet => {
                warn!("no migrations defined");
                Ok(())
            }
            SchemaVersion::Inside(_) => {
                let max_version = v_max.into();
                if version > max_version {
                    warn!("specified version is higher than the max supported version");
                    return Err(Error::SpecifiedSchemaVersion(
                        SchemaVersionError::TargetVersionOutOfRange {
                            specified: version,
                            highest: max_version,
                        },
                    ));
                }

                self.goto(conn, version)
            }
            SchemaVersion::Outside(_) => unreachable!(),
        }
    }

    /// Run migrations on a temporary in-memory database from first to last, one by one.
    /// Convenience method for testing.
    pub fn validate(&self) -> Result<()> {
        let mut conn = Connection::open_in_memory()?;
        self.to_latest(&mut conn)
    }
}

// Read user version field from the SQLite db
fn user_version(conn: &Connection) -> Result<usize, rusqlite::Error> {
    #[allow(deprecated)] // To keep compatibility with lower rusqlite versions
    conn.query_row("PRAGMA user_version", NO_PARAMS, |row| row.get(0))
        .map(|v: i64| v as usize)
}

// Set user version field from the SQLite db
fn set_user_version(conn: &Connection, v: usize) -> Result<()> {
    trace!("set user version to: {}", v);
    let v = v as u32;
    conn.pragma_update(None, "user_version", &v)
        .map_err(|e| Error::RusqliteError {
            query: format!("PRAGMA user_version = {}; -- Approximate query", v),
            err: e,
        })
}
