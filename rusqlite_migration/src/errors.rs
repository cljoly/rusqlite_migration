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

//! Custom error types

use std::fmt;

use crate::SchemaVersion;

/// A typedef of the result returned by many methods.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Enum listing possible errors.
#[derive(Debug)]
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
    ForeignKeyCheck(Vec<ForeignKeyCheckError>),
    /// Error returned by the migration hook
    Hook(String),
    /// Error returned when loading migrations from directory
    FileLoad(String),
    /// An unknown error occurred. *Note*: such errors are not comparable between one another,
    /// much like NaN for floats.
    Unrecognized(Box<dyn std::error::Error + Send + Sync + 'static>),
}

impl PartialEq for Error {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::RusqliteError { query: q1, err: e1 },
                Self::RusqliteError { query: q2, err: e2 },
            ) => q1 == q2 && e1 == e2,
            (Self::SpecifiedSchemaVersion(a), Self::SpecifiedSchemaVersion(b)) => a == b,
            (Self::MigrationDefinition(a), Self::MigrationDefinition(b)) => a == b,
            (Self::ForeignKeyCheck(e1), Self::ForeignKeyCheck(e2)) => e1 == e2,
            (Self::Hook(a), Self::Hook(b)) => a == b,
            (Self::FileLoad(a), Self::FileLoad(b)) => a == b,
            // This makes Unrecognized errors behave like NaN (where NaN != NaN)
            (Self::Unrecognized(_), Self::Unrecognized(_)) => false,
            // Fallback to comparing enum variants
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

impl Error {
    /// Associate the SQL request that caused the error
    #[must_use]
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
        write!(f, "rusqlite_migrate error: {self:?}")
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::RusqliteError { query: _, err } => Some(err),
            Error::SpecifiedSchemaVersion(e) => Some(e),
            Error::MigrationDefinition(e) => Some(e),
            Error::ForeignKeyCheck(vec) => Some(vec.first()?),
            Error::Hook(_) | Error::FileLoad(_) => None,
            Error::Unrecognized(ref e) => Some(&**e),
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
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
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
                write!(f, "Attempt to migrate to version {specified}, which is higher than the highest version currently supported, {highest}.")
            }
        }
    }
}

impl std::error::Error for SchemaVersionError {}

/// Errors related to schema versions
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
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
                    "Migration {} (version {} -> {}) cannot be reverted",
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
#[derive(Debug, PartialEq, Eq, Clone)]
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
            "Foreign key check found row with id {} in table '{}' missing from table '{}' \
            but required by foreign key with id {}",
            self.rowid, self.table, self.parent, self.fkid
        )
    }
}

impl std::error::Error for ForeignKeyCheckError {}

/// Error enum with rusqlite or hook-specified errors.
#[derive(Debug, PartialEq)]
#[allow(clippy::enum_variant_names)]
#[non_exhaustive]
pub enum HookError {
    /// Rusqlite error, query may indicate the attempted SQL query
    RusqliteError(rusqlite::Error),
    /// Error returned by the hook
    Hook(String),
}

impl From<rusqlite::Error> for HookError {
    fn from(e: rusqlite::Error) -> HookError {
        HookError::RusqliteError(e)
    }
}

impl From<HookError> for Error {
    fn from(e: HookError) -> Error {
        match e {
            HookError::RusqliteError(err) => Error::with_sql(err, ""),
            HookError::Hook(s) => Error::Hook(s),
        }
    }
}

/// A typedef of the result returned by hooks.
pub type HookResult<E = HookError> = std::result::Result<(), E>;

#[cfg(test)]
mod tests;
