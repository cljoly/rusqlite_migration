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

#![cfg_attr(docsrs, feature(doc_auto_cfg))]
// The doc is extracted from the README.md file at build time
#![doc = include_str!(concat!(env!("OUT_DIR"), "/readme_for_rustdoc.md"))]

use log::{debug, info, trace, warn};
use rusqlite::{Connection, Transaction};

#[cfg(feature = "from-directory")]
use include_dir::Dir;

#[cfg(feature = "from-directory")]
mod loader;
#[cfg(feature = "from-directory")]
use loader::from_directory;

#[cfg(feature = "from-directory")]
mod builder;
#[cfg(feature = "from-directory")]
pub use builder::MigrationsBuilder;

#[cfg(feature = "alpha-async-tokio-rusqlite")]
mod asynch;
mod errors;

#[cfg(test)]
mod tests;

#[cfg(feature = "alpha-async-tokio-rusqlite")]
pub use asynch::AsyncMigrations;
pub use errors::{
    Error, ForeignKeyCheckError, HookError, HookResult, MigrationDefinitionError, Result,
    SchemaVersionError,
};
use std::{
    cmp::{self, Ordering},
    fmt::{self, Debug},
    iter::FromIterator,
    num::NonZeroUsize,
    ptr::addr_of,
};

/// Helper trait to make hook functions cloneable.
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

impl Debug for Box<dyn MigrationHook> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Don’t print the closure address as it changes between runs
        write!(f, "MigrationHook(<closure>)")
    }
}

impl Clone for Box<dyn MigrationHook> {
    fn clone(&self) -> Self {
        (**self).clone_box()
    }
}

/// One migration.
///
/// A migration can contain up- and down-hooks, which are incomparable closures.
/// To signify `M` equality we compare if two migrations either don't have hooks defined (they are set to `None`)
/// or if the closure memory addresses are the same.
#[derive(Debug, Clone)]
#[must_use]
pub struct M<'u> {
    up: &'u str,
    up_hook: Option<Box<dyn MigrationHook>>,
    down: Option<&'u str>,
    down_hook: Option<Box<dyn MigrationHook>>,
    foreign_key_check: bool,
    comment: Option<&'u str>,
}

impl<'u> PartialEq for M<'u> {
    fn eq(&self, other: &Self) -> bool {
        let equal_up_hooks = match (self.up_hook.as_ref(), other.up_hook.as_ref()) {
            (None, None) => true,
            (Some(a), Some(b)) => addr_of!(*a) as usize == addr_of!(*b) as usize,
            _ => false,
        };

        let equal_down_hooks = match (self.down_hook.as_ref(), other.down_hook.as_ref()) {
            (None, None) => true,
            (Some(a), Some(b)) => addr_of!(*a) as usize == addr_of!(*b) as usize,
            _ => false,
        };

        self.up == other.up
            && self.down == other.down
            && equal_up_hooks
            && equal_down_hooks
            && self.foreign_key_check == other.foreign_key_check
    }
}

impl<'u> Eq for M<'u> {}

impl<'u> M<'u> {
    /// Create a schema update. The SQL command will be executed only when the migration has not been
    /// executed on the underlying database.
    ///
    /// # Please note
    ///
    /// ## PRAGMA statements
    ///
    /// PRAGMA statements are discouraged here. They are often better applied outside of
    /// migrations, because:
    ///   * a PRAGMA executed this way may not be applied consistently. For instance:
    ///     * [`foreign_keys`](https://sqlite.org/pragma.html#pragma_foreign_keys) needs to be
    ///       executed for each sqlite connection, not just once per database as a migration. Please
    ///       see the [`Self::foreign_key_check()`] method to maintain foreign key constraints during
    ///       migrations instead.
    ///     * [`journal_mode`][jm] has no effect when executed inside transactions (that will be
    ///       the case for the SQL written in `up`).
    ///   * Multiple SQL commands containing `PRAGMA` are [not working][ru794] with the
    ///     `extra_check` feature of rusqlite.
    ///
    /// ## Misc.
    ///
    /// * SQL commands should end with a “;”.
    /// * You can use the `include_str!` macro to include whole files or opt for the
    ///   `from-directory` feature of this crate.
    ///
    /// # Example
    ///
    /// ```
    /// use rusqlite_migration::M;
    ///
    /// M::up("CREATE TABLE animals (name TEXT);");
    /// ```
    ///
    /// [ru794]: https://github.com/rusqlite/rusqlite/pull/794
    /// [jm]: https://sqlite.org/pragma.html#pragma_journal_mode
    pub const fn up(sql: &'u str) -> Self {
        Self {
            up: sql,
            up_hook: None,
            down: None,
            down_hook: None,
            foreign_key_check: false,
            comment: None,
        }
    }

    /// Add a comment to the schema update
    pub const fn comment(mut self, comment: &'u str) -> Self {
        self.comment = Some(comment);
        self
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
    /// This works both for upward and downward migrations.
    ///
    /// This will cause the migration to fail if [`PRAGMA foreign_key_check`][fkc] returns any
    /// foreign key check violations.
    ///
    /// # Turning `PRAGMA foreign_keys` ON and OFF
    ///
    /// By default with SQLite, foreign key constraints are not checked (that [may change in the
    /// future][fk]). If you wish to check this, you need to manually turn [`PRAGMA
    /// foreign_keys`][fk] ON. However, the [documentation for “Making Other Kinds Of Table Schema
    /// Changes”][doc_other_migration] suggests turning this OFF before running the migrations.
    ///
    /// This if you want to enforce foreign key checks, it seems best to disable it first (in case
    /// future versions of SQLite enable it by default), then run the migrations, then enable it,
    /// as in the example below.
    ///
    /// Please make sure you **do not** call `PRAGMA foreign_keys` from inside the migrations, as
    /// it would be a no-op (each migration is run inside a transaction).
    ///
    /// # Example
    ///
    /// ```
    /// use rusqlite::{params, Connection};
    /// use rusqlite_migration::{Migrations, M};
    ///
    /// let migrations = Migrations::new(vec![
    ///     M::up("CREATE TABLE animals (name TEXT);")
    ///         .foreign_key_check(), // Let’s pretend this is necessary here
    /// ]);
    ///
    /// let mut conn = Connection::open_in_memory().unwrap();
    ///
    /// // Turn foreign key constraints off for the duration of the migration
    /// conn.pragma_update(None, "foreign_keys", &"OFF").unwrap();
    ///
    /// migrations.to_latest(&mut conn).unwrap();
    ///
    /// // Restore foreign key constraints checks
    /// conn.pragma_update(None, "foreign_keys", &"ON").unwrap();
    ///
    /// conn.execute("INSERT INTO animals (name) VALUES (?1)", params!["dog"])
    ///     .unwrap();
    /// ```
    ///
    /// [fk]: https://www.sqlite.org/pragma.html#pragma_foreign_keys
    /// [fkc]: https://www.sqlite.org/pragma.html#pragma_foreign_key_check
    /// [doc_other_migration]: https://www.sqlite.org/lang_altertable.html#making_other_kinds_of_table_schema_changes
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

    /// Creates a set of migrations from a given directory by scanning subdirectories with a specified name pattern.
    /// The migrations are loaded and stored in the binary.
    ///
    /// # Directory Structure Requirements
    ///
    /// The migration directory pointed to by `include_dir!()` must contain
    /// subdirectories in accordance with the given pattern:
    /// `{usize id indicating the order}-{convenient migration name}`
    ///
    /// Those directories must contain at lest an `up.sql` file containing a valid upward
    /// migration. They can also contain a `down.sql` file containing a downward migration.
    ///
    /// ## Example structure
    ///
    /// ```no_test
    /// migrations
    /// ├── 01-friend_car
    /// │  └── up.sql
    /// ├── 02-add_birthday_column
    /// │  └── up.sql
    /// └── 03-add_animal_table
    ///    ├── down.sql
    ///    └── up.sql
    /// ```
    ///
    /// # Example
    ///
    /// ```
    /// use rusqlite_migration::Migrations;
    /// use include_dir::{Dir, include_dir};
    ///
    /// static MIGRATION_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/../examples/from-directory/migrations");
    /// let migrations = Migrations::from_directory(&MIGRATION_DIR).unwrap();
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`Error::FileLoad`] in case the subdirectory names are incorrect,
    /// or don't contain at least a valid `up.sql` file.
    #[cfg(feature = "from-directory")]
    pub fn from_directory(dir: &'static Dir<'static>) -> Result<Self> {
        let migrations = from_directory(dir)?
            .into_iter()
            .collect::<Option<Vec<_>>>()
            .ok_or(Error::FileLoad("Could not load migrations".to_string()))?;

        Ok(Self { ms: migrations })
    }

    /// **Deprecated**: [`Migrations`] now implements [`FromIterator`], so use [`Migrations::from_iter`] instead.
    ///
    /// Performs allocations transparently.
    #[deprecated = "Use the `FromIterator` trait implementation instead. For instance, you can call Migrations::from_iter."]
    pub fn new_iter<I: IntoIterator<Item = M<'m>>>(ms: I) -> Self {
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
        Ok(user_version(conn).map(|v| self.db_version_to_schema(v))?)
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
        trace!("committed migration transaction");

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

                if m.foreign_key_check {
                    validate_foreign_keys(&tx)?;
                }
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
                    "some migrations to run, target: {target_db_version}, current: {current_version}"
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
    #[allow(clippy::missing_panics_doc)]
    pub fn max_schema_version(&self) -> SchemaVersion {
        match self.ms.len() {
            0 => SchemaVersion::NoneSet,
            v => SchemaVersion::Inside(
                NonZeroUsize::new(v).expect("Already checked for 0 in previous match arm")),
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
    ///
    /// Returns [`Error::RusqliteError`] if rusqlite returns an error when executing a migration
    /// statement. Note that this immediatley stops applying migrations.
    /// ```rust
    /// # use rusqlite_migration::{Migrations, M};
    /// let mut conn = rusqlite::Connection::open_in_memory().unwrap();
    ///
    /// let migrations = Migrations::new(vec![
    ///     M::up("CREATE TABLE t1 (c);"),
    ///     M::up("SYNTAX ERROR"),         // This won’t be applied
    ///     M::up("CREATE TABLE t2 (c);"), // This won’t be applied either because the migration above
    ///                                    // failed
    /// ]);
    ///
    /// assert!(matches!(
    ///     migrations.to_latest(&mut conn),
    ///     Err(rusqlite_migration::Error::RusqliteError {
    ///         query: _,
    ///         err: rusqlite::Error::SqliteFailure(_, _),
    ///     })
    /// ));
    /// ```
    /// If rusqlite `extra_check` feature is enabled, any migration that returns a value will error
    /// and no further migrations will be applied.
    ///
    /// # Transaction Behavior
    ///
    /// Since rusqlite 0.33, a [default transaction behavior][default_behavior] can be set. For
    /// now, when applying migrations, this setting will be respected.
    ///
    /// Please note that future minor versions of rusqlite_migration might decide to ignore the
    /// setting and to instead use any transaction behavior deemed most appropriate.  You can read
    /// more in the [corresponding page of the SQLite documentation][sqlite_doc].
    ///
    ///
    /// [default_behavior]: https://github.com/rusqlite/rusqlite/pull/1532
    /// [sqlite_doc]: https://sqlite.org/lang_transaction.html
    pub fn to_latest(&self, conn: &mut Connection) -> Result<()> {
        let v_max = self.max_schema_version();
        match v_max {
            SchemaVersion::NoneSet => {
                warn!("no migration defined");
                Err(Error::MigrationDefinition(
                    MigrationDefinitionError::NoMigrationsDefined,
                ))
            }
            SchemaVersion::Inside(v) => {
                debug!("some migrations defined (version: {v}), try to migrate");
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
            SchemaVersion::Inside(v) => {
                debug!("some migrations defined (version: {v}), try to migrate");
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

    /// Run upward migrations on a temporary in-memory database from first to last, one by one.
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
    ///         migrations.validate().unwrap();
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
    // We can’t fix this without breaking API compatibility
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    conn.query_row("PRAGMA user_version", [], |row| row.get(0))
        .map(|v: i64| v as usize)
}

// Set user version field from the SQLite db
fn set_user_version(conn: &Connection, v: usize) -> Result<()> {
    trace!("set user version to: {}", v);
    // We can’t fix this without breaking API compatibility
    #[allow(clippy::cast_possible_truncation)]
    let v = v as u32;
    conn.pragma_update(None, "user_version", v)
        .map_err(|e| Error::RusqliteError {
            query: format!("PRAGMA user_version = {v}; -- Approximate query"),
            err: e,
        })
}

// Validate that no foreign keys are violated
fn validate_foreign_keys(conn: &Connection) -> Result<()> {
    let pragma_fk_check = "PRAGMA foreign_key_check";
    let mut stmt = conn
        .prepare_cached(pragma_fk_check)
        .map_err(|e| Error::with_sql(e, pragma_fk_check))?;

    let fk_errors = stmt
        .query_map([], |row| {
            Ok(ForeignKeyCheckError {
                table: row.get(0)?,
                rowid: row.get(1)?,
                parent: row.get(2)?,
                fkid: row.get(3)?,
            })
        })
        .map_err(|e| Error::with_sql(e, pragma_fk_check))?
        .collect::<Result<Vec<_>, _>>()?;
    if !fk_errors.is_empty() {
        Err(crate::Error::ForeignKeyCheck(fk_errors))
    } else {
        Ok(())
    }
}

impl<'u> FromIterator<M<'u>> for Migrations<'u> {
    fn from_iter<T: IntoIterator<Item = M<'u>>>(iter: T) -> Self {
        Self {
            ms: Vec::from_iter(iter),
        }
    }
}
