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

use log::{debug, info, trace, warn};
use rusqlite::Connection;
use rusqlite::NO_PARAMS;

mod errors;

#[cfg(test)]
mod tests;
pub use errors::{Error, MigrationDefinitionError, Result, SchemaVersionError};

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
            warn!(
                "Migration to version {} has no down variant: {:?}",
                i + 1,
                bad_m
            );
            return Err(Error::MigrationDefinition(
                MigrationDefinitionError::DownNotDefined { to_version: i + 1 },
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
        let res = if target_db_version == current_version {
            debug!("no migration to run, db already up to date");
            return Ok(()); // return directly, so the migration message is not printed
        } else if target_db_version > current_version {
            debug!(
                "some migrations to run, target_db_version: {}, current_version: {}",
                target_db_version, current_version
            );
            self.goto_up(conn, current_version, target_db_version)
        } else {
            debug!(
                "rollback to older version requested, target_db_version: {}, current_version: {}",
                target_db_version, current_version
            );
            self.goto_down(conn, current_version, target_db_version)
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

    /// Migrate the database to latest schema version. The migrations are applied atomically.
    pub fn latest(&self, conn: &mut Connection) -> Result<()> {
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
        self.latest(&mut conn)
    }
}

// Read user version field from the SQLite db
fn user_version(conn: &Connection) -> Result<usize, rusqlite::Error> {
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
