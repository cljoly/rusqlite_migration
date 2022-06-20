//! Custom error types

use std::fmt;

use crate::SchemaVersion;

/// A typedef of the result returned by many methods.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Enum listing possible errors.
#[derive(Debug, PartialEq)]
#[allow(clippy::enum_variant_names)]
#[non_exhaustive]
pub enum Error {
    /// Rusqlite error, query may indicate the attempted SQL query
    RusqliteError {
        /// SQL query that caused the error
        query: String,
        /// Error returned by rusqlite
        err: rusqlite::Error,
    },
    /// Error with the specified schema version
    SpecifiedSchemaVersion(SchemaVersionError),
    /// Something wrong with migration definitions
    MigrationDefinition(MigrationDefinitionError),
    /// The foreign key check failed
    ForeignKeyCheck(ForeignKeyCheckError),
}

impl Error {
    /// Associtate the SQL request that caused the error
    pub fn with_sql(e: rusqlite::Error, sql: &str) -> Error {
        Error::RusqliteError {
            query: String::from(sql),
            err: e,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // TODO Format the error with fmt instead of debug
        write!(f, "rusqlite_migrate error: {:?}", self)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::RusqliteError { query: _, err } => Some(err),
            Error::SpecifiedSchemaVersion(e) => Some(e),
            Error::MigrationDefinition(e) => Some(e),
            Error::ForeignKeyCheck(e) => Some(e),
        }
    }
}

impl From<rusqlite::Error> for Error {
    fn from(e: rusqlite::Error) -> Error {
        Error::RusqliteError {
            query: String::new(),
            err: e,
        }
    }
}

/// Errors related to schema versions
#[derive(Debug, PartialEq, Clone, Copy)]
#[allow(clippy::enum_variant_names)]
#[non_exhaustive]
pub enum SchemaVersionError {
    /// Attempt to migrate to a version out of range for the supplied migrations
    TargetVersionOutOfRange {
        /// The attempt to migrate to this version caused the error
        specified: SchemaVersion,
        /// Highest version defined in the migration set
        highest: SchemaVersion,
    },
}

impl fmt::Display for SchemaVersionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SchemaVersionError::TargetVersionOutOfRange { specified, highest } => {
                write!(f, "Attempt to migrate to version {}, which is higher than the highest version currently supported, {}.", specified, highest)
            }
        }
    }
}

impl std::error::Error for SchemaVersionError {}

/// Errors related to schema versions
#[derive(Debug, PartialEq, Clone, Copy)]
#[allow(clippy::enum_variant_names)]
#[non_exhaustive]
pub enum MigrationDefinitionError {
    /// Migration has no down version
    DownNotDefined {
        /// Index of the migration that caused the error
        migration_index: usize,
    },
    /// Attempt to migrate when no migrations are defined
    NoMigrationsDefined,
    /// Attempt to migrate when the database is currently at a higher migration level (see <https://github.com/cljoly/rusqlite_migration/issues/17>)
    DatabaseTooFarAhead,
}

impl fmt::Display for MigrationDefinitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MigrationDefinitionError::DownNotDefined { migration_index } => {
                write!(
                    f,
                    "Migration {} (version {} -> {}) cannot be reverted.",
                    migration_index,
                    migration_index,
                    migration_index + 1
                )
            }
            MigrationDefinitionError::NoMigrationsDefined => {
                write!(f, "Attempt to migrate with no migrations defined")
            }
            MigrationDefinitionError::DatabaseTooFarAhead => {
                write!(
                    f,
                    "Attempt to migrate a database with a migration number that is too high"
                )
            }
        }
    }
}

impl std::error::Error for MigrationDefinitionError {}

/// Error caused by a foreign key check
#[derive(Debug, PartialEq, Clone)]
pub struct ForeignKeyCheckError {
    pub(super) table: String,
    pub(super) rowid: i64,
    pub(super) parent: String,
    pub(super) fkid: i64,
}

impl fmt::Display for ForeignKeyCheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Foreign key check found a row {} on table {} missing from {} but required by {}",
            self.rowid, self.table, self.parent, self.fkid
        )
    }
}

impl std::error::Error for ForeignKeyCheckError {}
