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
#![warn(missing_docs)]

//! Rusqlite Migration is a simple schema migration library for [rusqlite](https://lib.rs/crates/rusqlite) using [user_version][uv] instead of an SQL table to maintain the current schema version.
//!
//! It aims for:
//! - **simplicity**: define a set of SQL statements. Just add more SQL statement to change the schema. No external CLI, no macro.
//! - **performance**: no need to add a table to be parsed, the [`user_version`][uv] field is at a fixed offset in the sqlite file format.
//!
//! It works especially well with other small libraries complementing rusqlite, like [serde_rusqlite](https://crates.io/crates/serde_rusqlite).
//!
//! [uv]: https://sqlite.org/pragma.html#pragma_user_version
//!
//! ## Example
//!
//! Here, we define SQL statements to run with [Migrations::new](crate::Migrations::new) and run these (if necessary) with [.to_latest()](crate::Migrations::to_latest).
//!
//! ```
//! use rusqlite::{params, Connection};
//! use rusqlite_migration::{Migrations, M};
//!
//! // 1️⃣ Define migrations
//! let migrations = Migrations::new(vec![
//!     M::up("CREATE TABLE friend(name TEXT NOT NULL);"),
//!     // In the future, add more migrations here:
//!     //M::up("ALTER TABLE friend ADD COLUMN email TEXT;"),
//! ]);
//!
//! let mut conn = Connection::open_in_memory().unwrap();
//!
//! // Apply some PRAGMA, often better to do it outside of migrations
//! conn.pragma_update(None, "journal_mode", &"WAL").unwrap();
//!
//! // 2️⃣ Update the database schema, atomically
//! migrations.to_latest(&mut conn).unwrap();
//!
//! // 3️⃣ Use the database 🥳
//! conn.execute("INSERT INTO friend (name) VALUES (?1)", params!["John"])
//!     .unwrap();
//! ```
//!
//! Please see the [examples](https://github.com/cljoly/rusqlite_migrate/tree/master/examples) folder for more, in particular:
//! - migrations with multiple SQL statements (using for instance `r#"…"` or `include_str!(…)`)
//! - use of lazy_static
//! - migrations to previous versions (downward migrations)
//!
//! I’ve also made a [cheatsheet of SQLite pragma for improved performance and consistency](https://cj.rs/blog/sqlite-pragma-cheatsheet-for-performance-and-consistency/).
//!
//! ### Built-in tests
//!
//! To test that the migrations are working, you can add this in your test module:
//!
//! ```
//! #[test]
//! fn migrations_test() {
//!     assert!(MIGRATIONS.validate().is_ok());
//! }
//! ```
//!
//! ## Contributing
//!
//! Contributions (documentation or code improvements in particular) are welcome, see [contributing](https://cj.rs/docs/contribute/)!
//!
//! ## Acknowledgments
//!
//! I would like to thank all the contributors, as well as the authors of the dependencies this crate uses.

use log::{debug, info, trace, warn};
use rusqlite::Connection;
#[allow(deprecated)] // To keep compatibility with lower rusqlite versions
use rusqlite::NO_PARAMS;

mod errors;

#[cfg(test)]
mod tests;
pub use errors::{Error, MigrationDefinitionError, Result, SchemaVersionError};
use std::{
    cmp::{self, Ordering},
    fmt,
    num::NonZeroUsize,
};

/// One migration
#[derive(Debug, PartialEq, Clone)]
pub struct M<'u> {
    up: &'u str,
    down: Option<&'u str>,
}

impl<'u> M<'u> {
    /// Create a schema update. The SQL command will be executed only when the migration has not been
    /// executed on the underlying database.
    ///
    /// # Please note
    ///
    /// * PRAGMA statements are discouraged here. They are often better applied outside of
    /// migrations, because:
    ///   * a PRAGMA executed this way may not be applied consistently. For instance:
    ///     * [`foreign_keys`](https://sqlite.org/pragma.html#pragma_foreign_keys) needs to be
    ///     executed for each sqlite connection, not just once per database as a migration,
    ///     * [`journal_mode`](https://sqlite.org/pragma.html#pragma_journal_mode) has no effect
    ///     when executed inside transactions (that will be the case for the SQL written in `up`).
    ///   * Multiple SQL commands contaning `PRAGMA` are [not
    ///   working](https://github.com/rusqlite/rusqlite/pull/794) with the `extra_check` feature of
    ///   rusqlite.
    /// * SQL commands should end with a “;”.
    ///
    /// # Example
    ///
    /// ```
    /// use rusqlite_migration::M;
    ///
    /// M::up("CREATE TABLE animals (name TEXT);");
    /// ```
    pub fn up(sql: &'u str) -> Self {
        Self {
            up: sql,
            down: None,
        }
    }

    /// Define a down-migration. This SQL statement should exactly reverse the changes
    /// performed in `up()`.
    ///
    /// A call to this method is **not** required.
    ///
    /// # Example
    ///
    /// ```
    /// use rusqlite_migration::M;
    ///
    /// M::up("CREATE TABLE animals (name TEXT);")
    ///     .down("DROP TABLE animals;");
    /// ```
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
    /// The current version in the database is inside the range of defined
    /// migrations
    Inside(NonZeroUsize),
    /// The current version in the database is outside any migration defined
    Outside(NonZeroUsize),
}

impl From<&SchemaVersion> for usize {
    /// Translate schema version to db version
    fn from(schema_version: &SchemaVersion) -> usize {
        match schema_version {
            SchemaVersion::NoneSet => 0,
            SchemaVersion::Inside(v) | SchemaVersion::Outside(v) => From::from(*v),
        }
    }
}

impl From<SchemaVersion> for usize {
    fn from(schema_version: SchemaVersion) -> Self {
        From::from(&schema_version)
    }
}

impl fmt::Display for SchemaVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SchemaVersion::NoneSet => write!(f, "0 (no version set)"),
            SchemaVersion::Inside(v) => write!(f, "{} (inside)", v),
            SchemaVersion::Outside(v) => write!(f, "{} (outside)", v),
        }
    }
}

impl cmp::PartialOrd for SchemaVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let self_usize: usize = self.into();
        let other_usize: usize = other.into();

        self_usize.partial_cmp(&other_usize)
    }
}

/// Set of migrations
#[derive(Debug, PartialEq, Clone)]
pub struct Migrations<'m> {
    ms: Vec<M<'m>>,
}

impl<'m> Migrations<'m> {
    /// Create a set of migrations.
    ///
    /// # Example
    ///
    /// ```
    /// use rusqlite_migration::{Migrations, M};
    ///
    /// let migrations = Migrations::new(vec![
    ///     M::up("CREATE TABLE animals (name TEXT);"),
    ///     M::up("CREATE TABLE food (name TEXT);"),
    /// ]);
    /// ```
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
            v if v > 0 && v <= self.ms.len() => SchemaVersion::Inside(
                NonZeroUsize::new(v).expect("schema version should not be equal to 0"),
            ),
            v => SchemaVersion::Outside(
                NonZeroUsize::new(v).expect("schema version should not be equal to 0"),
            ),
        }
    }

    /// Get the current schema version
    ///
    /// # Example
    ///
    /// ```
    /// use rusqlite_migration::{Migrations, M, SchemaVersion};
    /// use std::num::NonZeroUsize;
    ///
    /// let mut conn = rusqlite::Connection::open_in_memory().unwrap();
    ///
    /// let migrations = Migrations::new(vec![
    ///     M::up("CREATE TABLE animals (name TEXT);"),
    ///     M::up("CREATE TABLE food (name TEXT);"),
    /// ]);
    ///
    /// assert_eq!(SchemaVersion::NoneSet, migrations.current_version(&conn).unwrap());
    ///
    /// // Go to the latest version
    /// migrations.to_latest(&mut conn).unwrap();
    ///
    /// assert_eq!(SchemaVersion::Inside(NonZeroUsize::new(2).unwrap()), migrations.current_version(&conn).unwrap());
    /// ```
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
        match self.ms.len() {
            0 => SchemaVersion::NoneSet,
            v => SchemaVersion::Inside(
                NonZeroUsize::new(v).expect("schema version should not be equal to 0"),
            ),
        }
    }

    /// Migrate the database to latest schema version. The migrations are applied atomically.
    #[deprecated(since = "0.4.0", note = "Renammed to “to_latest”")]
    pub fn latest(&self, conn: &mut Connection) -> Result<()> {
        self.to_latest(conn)
    }

    /// Migrate the database to latest schema version. The migrations are applied atomically.
    ///
    /// # Example
    ///
    /// ```
    /// use rusqlite_migration::{Migrations, M};
    /// let mut conn = rusqlite::Connection::open_in_memory().unwrap();
    ///
    /// let migrations = Migrations::new(vec![
    ///     M::up("CREATE TABLE animals (name TEXT);"),
    ///     M::up("CREATE TABLE food (name TEXT);"),
    /// ]);
    ///
    /// // Go to the latest version
    /// migrations.to_latest(&mut conn).unwrap();
    ///
    /// // You can then insert values in the database
    /// conn.execute("INSERT INTO animals (name) VALUES ('dog')", []).unwrap();
    /// conn.execute("INSERT INTO food (name) VALUES ('carrot')", []).unwrap();
    /// ```
    pub fn to_latest(&self, conn: &mut Connection) -> Result<()> {
        let v_max = self.max_schema_version();
        match v_max {
            SchemaVersion::NoneSet => {
                warn!("no migration defined");
                Err(Error::MigrationDefinition(
                    MigrationDefinitionError::NoMigrationsDefined,
                ))
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
    /// # Specifying versions
    ///
    /// - Empty database (no migrations run yet) has version `0`.
    /// - The version increases after each migration, so after the first migration has run, the schema version is `1`. For instance, if there are 3 migrations, version `3` is after all migrations have run.
    ///
    /// *Note*: As a result, the version is the index in the migrations vector *starting from 1*.
    ///
    /// # Example
    ///
    /// ```
    /// use rusqlite_migration::{Migrations, M};
    /// let mut conn = rusqlite::Connection::open_in_memory().unwrap();
    /// let migrations = Migrations::new(vec![
    ///     // 0: version 0, before having run any migration
    ///     M::up("CREATE TABLE animals (name TEXT);").down("DROP TABLE animals;"),
    ///     // 1: version 1, after having created the “animals” table
    ///     M::up("CREATE TABLE food (name TEXT);").down("DROP TABLE food;"),
    ///     // 2: version 2, after having created the food table
    /// ]);
    ///
    /// migrations.to_latest(&mut conn).unwrap(); // Create all tables
    ///
    /// // Go back to version 1, i.e. after running the first migration
    /// migrations.to_version(&mut conn, 1);
    /// conn.execute("INSERT INTO animals (name) VALUES ('dog')", []).unwrap();
    /// conn.execute("INSERT INTO food (name) VALUES ('carrot')", []).unwrap_err();
    ///
    /// // Go back to an empty database
    /// migrations.to_version(&mut conn, 0);
    /// conn.execute("INSERT INTO animals (name) VALUES ('cat')", []).unwrap_err();
    /// conn.execute("INSERT INTO food (name) VALUES ('milk')", []).unwrap_err();
    /// ```
    ///
    /// # Errors
    ///
    /// Attempts to migrate to a higher version than is supported will result in an error.
    ///
    /// When migrating downwards, all the reversed migrations must have a `.down()` variant,
    /// otherwise no migrations are run and the function returns an error.
    pub fn to_version(&self, conn: &mut Connection, version: usize) -> Result<()> {
        let target_version: SchemaVersion = self.db_version_to_schema(version);
        let v_max = self.max_schema_version();
        match v_max {
            SchemaVersion::NoneSet => {
                warn!("no migrations defined");
                Err(Error::MigrationDefinition(
                    MigrationDefinitionError::NoMigrationsDefined,
                ))
            }
            SchemaVersion::Inside(_) => {
                if target_version > v_max {
                    warn!("specified version is higher than the max supported version");
                    return Err(Error::SpecifiedSchemaVersion(
                        SchemaVersionError::TargetVersionOutOfRange {
                            specified: target_version,
                            highest: v_max,
                        },
                    ));
                }

                self.goto(conn, target_version.into())
            }
            SchemaVersion::Outside(_) => unreachable!(),
        }
    }

    /// Run migrations on a temporary in-memory database from first to last, one by one.
    /// Convenience method for testing.
    ///
    /// # Example
    ///
    /// ```
    /// #[cfg(test)]
    /// mod tests {
    ///
    ///     // … Other tests …
    ///
    ///     #[test]
    ///     fn migrations_test() {
    ///         assert!(migrations.validate().is_ok());
    ///     }
    /// }
    /// ```
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
