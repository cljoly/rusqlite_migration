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

use std::result;

use rusqlite::Connection;
use rusqlite::NO_PARAMS;

#[cfg(test)]
mod tests {

    use super::*;

    fn m_valid0() -> M<'static> {
        M::up("PRAGMA journal_mode = WAL")
    }
    fn m_valid10() -> M<'static> {
        M::up("CREATE TABLE t1(a, b);")
    }
    fn m_valid11() -> M<'static> {
        M::up("ALTER TABLE t1 RENAME COLUMN b TO c;")
    }
    fn m_valid20() -> M<'static> {
        M::up("CREATE TABLE t2(b);")
    }
    fn m_valid21() -> M<'static> {
        M::up("ALTER TABLE t2 ADD COLUMN a;")
    }

    // All valid Ms in the right order
    fn all_valid() -> Vec<M<'static>> {
        vec![
            m_valid0(),
            m_valid10(),
            m_valid11(),
            m_valid20(),
            m_valid21(),
        ]
    }

    fn m_invalid0() -> M<'static> {
        M::up("CREATE TABLE table3()")
    }
    fn m_invalid1() -> M<'static> {
        M::up("something invalid")
    }

    #[test]
    fn empty_migrations_test() {
        let _ = Migrations::new(vec![]);
    }
    // TODO
    // Error in the middle of  a migration (right version number + proper errro)
    // Test function for SQL statements that panics & don’t panic

    #[test]
    fn user_version_start_0_test() {
        let conn = Connection::open_in_memory().unwrap();

        {
            let migrations = Migrations::new(vec![]);
            assert_eq!(Ok(0), migrations.user_version(&conn))
        }

        {
            let migrations = Migrations::new(vec![M::up("something valid")]);
            assert_eq!(Ok(0), migrations.user_version(&conn))
        }
    }

    #[test]
    fn invalid_migration_statement_test() {
        for m in &[m_invalid0(), m_invalid1(), m_valid11(), m_valid21()] {
            let migrations = Migrations::new(vec![m.clone()]);
            assert_ne!(Ok(()), migrations.validate())
        }
    }

    #[test]
    #[should_panic]
    fn invalid_migration_multiple_statement_test() {
        let migrations = Migrations::new(vec![m_valid0(), m_invalid1()]);
        migrations.validate().unwrap()
    }

    #[test]
    fn valid_migration_multiple_statement_test() {
        for m in &[m_valid0(), m_valid10(), m_valid20()] {
            let migrations = Migrations::new(vec![m.clone()]);
            assert_eq!(Ok(()), migrations.validate())
        }
    }

    #[test]
    fn all_valid_test() {
        assert_eq!(Ok(()), Migrations::new(all_valid()).validate())
    }
}

/// Enum listing possible errors.
#[derive(Debug, PartialEq)]
#[allow(clippy::enum_variant_names)]
#[non_exhaustive]
pub enum Error {
    /// Rusqlite error
    RusqliteError(rusqlite::Error),
    /// Error with the specified schema version
    SchemaVersion(SchemaVersionError),
}

impl From<rusqlite::Error> for Error {
    fn from(e: rusqlite::Error) -> Error {
        Error::RusqliteError(e)
    }
}

/// Errors related to schema versions
#[derive(Debug, PartialEq)]
#[allow(clippy::enum_variant_names)]
#[non_exhaustive]
pub enum SchemaVersionError {
    /// Attempts to migrate to a version lower than the version currently in
    /// the database. This is currently not supported
    MigrateToLowerNotSupported,
}
/// A typedef of the result returned by many methods.
pub type Result<T, E = Error> = result::Result<T, E>;

/// One migration
#[derive(Debug, PartialEq, Clone)]
pub struct M<'u> {
    up: &'u str,
}

impl<'u> M<'u> {
    /// Create a schema update. The SQL command must end with a “;”
    pub fn up(sql: &'u str) -> Self {
        Self { up: sql }
    }
}

/// Set of migrations
#[derive(Debug, PartialEq, Clone)]
pub struct Migrations<'m> {
    ms: Vec<M<'m>>,
}

impl<'m> Migrations<'m> {
    pub fn new(ms: Vec<M<'m>>) -> Self {
        Self { ms }
    }

    /// Performs allocations transparently
    pub fn new_iter<I: IntoIterator<Item = M<'m>>>(ms: I) -> Self {
        use std::iter::FromIterator;
        Self::new(Vec::from_iter(ms))
    }

    // Read user version field from the SQLite db
    fn user_version(&self, conn: &Connection) -> Result<usize, rusqlite::Error> {
        conn.query_row("PRAGMA user_version", NO_PARAMS, |row| row.get(0))
            .map(|v: i64| v as usize)
    }

    // Set user version field from the SQLite db
    fn set_user_version(&self, conn: &Connection, v: usize) -> Result<()> {
        let v = v as u32;
        conn.pragma_update(None, "user_version", &v)?;
        Ok(())
    }

    /// Get current schema version
    pub fn current_version(&self, conn: &Connection) -> Result<usize> {
        self.user_version(conn).map_err(|e| e.into())
    }

    /// Migrate upward methods. This is rolled back on error.
    /// On success, returns the number of update performed
    fn goto_up(
        &self,
        conn: &mut Connection,
        current_version: usize,
        target_version: usize,
    ) -> Result<usize> {
        debug_assert!(current_version < target_version);
        debug_assert!(target_version <= self.ms.len());
        debug_assert!(0 < target_version);

        let ms_to_apply = &self.ms[current_version..target_version];
        let tx = conn.transaction()?;
        for v in ms_to_apply {
            let mut stmt = tx.prepare(v.up)?;
            let mut row = stmt.query(NO_PARAMS)?;
            // XXX Forces execution of the statement. We can’t use execute, as
            // this requires no row to be returned and some pragma do.
            let _ = row.next();
        }
        self.set_user_version(&tx, target_version)?;
        tx.commit()?;
        return Ok(ms_to_apply.len());
    }

    /// Migrate downward methods (not implemented at the moment)
    fn goto_down(&self) -> Result<()> {
        Err(Error::SchemaVersion(
            SchemaVersionError::MigrateToLowerNotSupported,
        ))
    }

    /// Go to a given schema version
    pub fn goto(&self, conn: &mut Connection, version: usize) -> Result<()> {
        let current_version = self.current_version(conn)?;
        if version == current_version {
            return Ok(());
        }
        if version > current_version {
            return self.goto_up(conn, current_version, version).map(|_| ());
        }
        // version < current_version
        return self.goto_down();
    }

    /// Maximum version defined in the migration set
    pub fn max_schema_version(&self) -> usize {
        return self.ms.len();
    }

    /// Migrate the database to latest schema version.
    pub fn latest(&self, conn: &mut Connection) -> Result<()> {
        let max_schema_version = self.max_schema_version();
        self.goto(conn, max_schema_version)
    }

    /// Run migrations from first to last, one by one. Convenience method for testing.
    pub fn validate(&self) -> Result<()> {
        let mut conn = Connection::open_in_memory()?;
        self.latest(&mut conn)
    }
}
