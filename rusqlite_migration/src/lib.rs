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

//! Rusqlite Migration is a simple and performant schema migration library for [rusqlite](https://lib.rs/crates/rusqlite).
//!
//! * **Performance**:
//!     * *Fast database opening*: to keep track of the current migration state, most tools create one or more tables in the database. These tables require parsing by SQLite and are queried with SQL statements. This library uses the [`user_version`][uv] value instead. It’s much lighter as it is just an integer at a [fixed offset][uv_offset] in the SQLite file.
//!     * *Fast compilation*: this crate is very small and does not use macros to define the migrations.
//! * **Simplicity**: this crate strives for simplicity. Just define a set of SQL statements as strings in your Rust code. Add more SQL statements over time as needed. No external CLI required. Additionally, rusqlite_migration works especially well with other small libraries complementing rusqlite, like [serde_rusqlite][].
//!
//! [diesel_migrations]: https://lib.rs/crates/diesel_migrations
//! [pgfine]: https://crates.io/crates/pgfine
//! [movine]: https://crates.io/crates/movine
//! [uv]: https://sqlite.org/pragma.html#pragma_user_version
//! [uv_offset]: https://www.sqlite.org/fileformat.html#user_version_number
//! [serde_rusqlite]: https://crates.io/crates/serde_rusqlite
//!
//! ## Example
//!
//! Here, we define SQL statements to run with [`Migrations::new()`][migrations_new] and run these (if necessary) with [`Migrations::to_latest()`][migrations_to_latest].
//!
//! [migrations_new]: https://docs.rs/rusqlite_migration/latest/rusqlite_migration/struct.Migrations.html#method.new
//! [migrations_to_latest]: https://docs.rs/rusqlite_migration/latest/rusqlite_migration/struct.Migrations.html#method.to_latest
//!
//! ``` rust
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
//! - `async` migrations in the [`quick_start_async.rs`][quick_start_async] file
//! - migrations with multiple SQL statements (using for instance `r#"…"` or `include_str!(…)`)
//! - use of lazy_static
//! - migrations to previous versions (downward migrations)
//!
//! [quick_start_async]: https://github.com/cljoly/rusqlite_migrate/tree/master/examples/quick_start_async.rs
//!
//! I’ve also made a [cheatsheet of SQLite pragma for improved performance and consistency](https://cj.rs/blog/sqlite-pragma-cheatsheet-for-performance-and-consistency/).
//!
//! ### Built-in tests
//!
//! To test that the migrations are working, you can add this in your test module:
//!
//! ``` rust
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
//!

use log::{debug, info, trace, warn};
#[allow(deprecated)] // To keep compatibility with lower rusqlite versions
use rusqlite::NO_PARAMS;
use rusqlite::{Connection, OptionalExtension, Transaction};

#[cfg(feature = "async-tokio-rusqlite")]
mod asynch;
mod errors;

#[cfg(test)]
mod tests;
#[cfg(feature = "async-tokio-rusqlite")]
pub use asynch::AsyncMigrations;
pub use errors::{
    Error, ForeignKeyCheckError, HookError, HookResult, MigrationDefinitionError, Result,
    SchemaVersionError,
};
use std::{
    cmp::{self, Ordering},
    fmt::{self, Debug},
    num::NonZeroUsize,
};

/// Helper trait to make hook functions clonable.
pub trait MigrationHook: Fn(&Transaction) -> HookResult + Send + Sync {
    /// Clone self.
    fn clone_box(&self) -> Box<dyn MigrationHook>;
}

impl<T> MigrationHook for T
where
    T: 'static + Clone + Send + Sync + Fn(&Transaction) -> HookResult,
{
    fn clone_box(&self) -> Box<dyn MigrationHook> {
        Box::new(self.clone())
    }
}

impl PartialEq for Box<dyn MigrationHook> {
    fn eq(&self, other: &Self) -> bool {
        self == other
    }
}

impl Eq for Box<dyn MigrationHook> {}

impl Debug for Box<dyn MigrationHook> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Box").field(&self).finish()
    }
}

impl Clone for Box<dyn MigrationHook> {
    fn clone(&self) -> Self {
        (**self).clone_box()
    }
}

/// One migration
#[derive(Debug, PartialEq, Eq, Clone)]
#[must_use]
pub struct M<'u> {
    up: &'u str,
    up_hook: Option<Box<dyn MigrationHook>>,
    down: Option<&'u str>,
    down_hook: Option<Box<dyn MigrationHook>>,
    foreign_key_check: bool,
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
    pub const fn up(sql: &'u str) -> Self {
        Self {
            up: sql,
            up_hook: None,
            down: None,
            down_hook: None,
            foreign_key_check: false,
        }
    }

    /// Create a schema update running additional Rust code. The SQL command will be executed only
    /// when the migration has not been executed on the underlying database. The `hook` code will
    /// be executed *after* the SQL command executed successfully.
    ///
    /// See [`Self::up()`] for additional notes.
    ///
    /// # Example
    ///
    /// ```
    /// use rusqlite_migration::{M, Migrations};
    /// use rusqlite::Transaction;
    ///
    /// let migrations = Migrations::new(vec![
    ///     // This table will later be filled with some novel content
    ///     M::up("CREATE TABLE novels (text TEXT);"),
    ///     M::up_with_hook(
    ///         "ALTER TABLE novels ADD compressed TEXT;",
    ///         |tx: &Transaction| {
    ///             let mut stmt = tx.prepare("SELECT rowid, text FROM novels")?;
    ///             let rows = stmt
    ///                 .query_map([], |row| {
    ///                     Ok((row.get_unwrap::<_, i64>(0), row.get_unwrap::<_, String>(1)))
    ///                 })?;
    ///
    ///             for row in rows {
    ///                 let row = row?;
    ///                 let rowid = row.0;
    ///                 let text = row.1;
    ///                 // Replace with a proper compression strategy ...
    ///                 let compressed = &text[..text.len() / 2];
    ///                 tx.execute(
    ///                     "UPDATE novels SET compressed = ?1 WHERE rowid = ?2;",
    ///                     rusqlite::params![compressed, rowid],
    ///                 )?;
    ///             }
    ///
    ///             Ok(())
    ///         },
    ///     ),
    /// ]);
    /// ```
    pub fn up_with_hook(sql: &'u str, hook: impl MigrationHook + 'static) -> Self {
        let mut m = Self::up(sql);
        m.up_hook = Some(hook.clone_box());
        m
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
    pub const fn down(mut self, sql: &'u str) -> Self {
        self.down = Some(sql);
        self
    }

    /// Define a down-migration running additional Rust code. This SQL statement should exactly
    /// reverse the changes performed in [`Self::up_with_hook()`]. `hook` will run before the SQL
    /// statement is executed.
    pub fn down_with_hook(mut self, sql: &'u str, hook: impl MigrationHook + 'static) -> Self {
        self.down = Some(sql);
        self.down_hook = Some(hook.clone_box());
        self
    }

    /// Enable an automatic validation of foreign keys before the migration transaction is closed.
    /// This will cause the migration to fail if `PRAGMA foreign_key_check` returns any rows.
    ///
    /// # Example
    ///
    /// ```
    /// use rusqlite_migration::M;
    ///
    /// M::up("CREATE TABLE animals (name TEXT);")
    ///     .foreign_key_check();
    /// ```
    pub const fn foreign_key_check(mut self) -> Self {
        self.foreign_key_check = true;
        self
    }
}

/// Schema version, in the context of Migrations
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
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
            SchemaVersion::Inside(v) => write!(f, "{v} (inside)"),
            SchemaVersion::Outside(v) => write!(f, "{v} (outside)"),
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
#[derive(Debug, PartialEq, Eq, Clone)]
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
    #[must_use]
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
    ///
    /// # Errors
    ///
    /// Returns [`Error::RusqliteError`] in case the user version cannot be queried.
    pub fn current_version(&self, conn: &Connection) -> Result<SchemaVersion> {
        user_version(conn)
            .map(|v| self.db_version_to_schema(v))
            .map_err(std::convert::Into::into)
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

            if m.foreign_key_check {
                validate_foreign_keys(&tx)?;
            }

            if let Some(hook) = &m.up_hook {
                hook(&tx)?;
            }
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
            if let Some(down) = m.down {
                debug!("Running: {}", &down);

                if let Some(hook) = &m.down_hook {
                    hook(&tx)?;
                }

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
                if current_version > self.ms.len() {
                    return Err(Error::MigrationDefinition(
                        MigrationDefinitionError::DatabaseTooFarAhead,
                    ));
                }
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
    /// conn.execute("INSERT INTO animals (name) VALUES (?)", ["dog"]).unwrap();
    /// conn.execute("INSERT INTO food (name) VALUES (?)", ["carrot"]).unwrap();
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`Error::MigrationDefinition`] if no migration is defined.
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
    /// conn.execute("INSERT INTO animals (name) VALUES (?)", ["dog"]).unwrap();
    /// conn.execute("INSERT INTO food (name) VALUES (?)", ["carrot"]).unwrap_err();
    ///
    /// // Go back to an empty database
    /// migrations.to_version(&mut conn, 0);
    /// conn.execute("INSERT INTO animals (name) VALUES (?)", ["cat"]).unwrap_err();
    /// conn.execute("INSERT INTO food (name) VALUES (?)", ["milk"]).unwrap_err();
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
    ///
    /// # Errors
    ///
    /// Returns [`Error::RusqliteError`] if the underlying sqlite database open call fails.
    pub fn validate(&self) -> Result<()> {
        let mut conn = Connection::open_in_memory()?;
        self.to_latest(&mut conn)
    }
}

// Read user version field from the SQLite db
fn user_version(conn: &Connection) -> Result<usize, rusqlite::Error> {
    // To keep compatibility with lower rusqlite versions
    #[allow(deprecated)]
    // We can’t fix this without breaking API compatibility
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    conn.query_row("PRAGMA user_version", NO_PARAMS, |row| row.get(0))
        .map(|v: i64| v as usize)
}

// Set user version field from the SQLite db
fn set_user_version(conn: &Connection, v: usize) -> Result<()> {
    trace!("set user version to: {}", v);
    // We can’t fix this without breaking API compatibility
    #[allow(clippy::cast_possible_truncation)]
    let v = v as u32;
    // To keep compatibility with lower rusqlite versions, allow the needless `&v` borrow
    #[allow(clippy::needless_borrow)]
    conn.pragma_update(None, "user_version", &v)
        .map_err(|e| Error::RusqliteError {
            query: format!("PRAGMA user_version = {v}; -- Approximate query"),
            err: e,
        })
}

// Validate that no foreign keys are violated
fn validate_foreign_keys(conn: &Connection) -> Result<()> {
    #[allow(deprecated)] // To keep compatibility with lower rusqlite versions
    conn.query_row("PRAGMA foreign_key_check", NO_PARAMS, |row| {
        Ok(ForeignKeyCheckError {
            table: row.get(0)?,
            rowid: row.get(1)?,
            parent: row.get(2)?,
            fkid: row.get(3)?,
        })
    })
    .optional()
    .map_err(|e| Error::RusqliteError {
        query: String::from("PRAGMA foreign_key_check"),
        err: e,
    })
    .and_then(|o| match o {
        Some(e) => Err(Error::ForeignKeyCheck(e)),
        None => Ok(()),
    })
}
