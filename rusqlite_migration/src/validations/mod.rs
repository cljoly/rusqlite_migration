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

//! Run a more complete set of validations (like requiring a downward migration to be present and
//! to apply cleanly). This is useful in a unit test, to validate the migrations.
//!
//! See also [`Migrations::validate`] for simple cases.
//!
//! # Example
//!
//! ```
//! #[cfg(test)]
//! mod tests {
//!
//!     // … Other tests …
//!
//!     #[test]
//!     fn migrations_test() -> Result<(), dyn Error> {
//!         Validations::everything().validate(migrations)?;
//!     }
//! }
//! ```

use std::fmt::Display;

use rusqlite::Connection;

use super::Migrations;

#[cfg(test)]
mod tests;

/// Result for validations
pub type Result<'m, T, E = Error> = std::result::Result<T, E>;

/// Enum of possible validation errors.
#[derive(Debug, PartialEq)]
#[non_exhaustive]
pub enum Error {
    /// Downward migrations were required for every upward migrations, but some are missing.
    MissingDownwardMigrations(Vec<(usize, String)>),
    /// Underlying rusqlite_migration error.
    RusqliteMigration(crate::Error),
}

impl From<crate::Error> for Error {
    fn from(value: crate::Error) -> Self {
        Error::RusqliteMigration(value)
    }
}

impl From<rusqlite::Error> for Error {
    fn from(value: rusqlite::Error) -> Self {
        Error::from(crate::Error::from(value))
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::MissingDownwardMigrations(_) => None,
            Error::RusqliteMigration(error) => Some(error),
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::RusqliteMigration(e) => write!(f, "underlying rusqlite migration error: {e}"),
            Error::MissingDownwardMigrations(vs) => {
                write!(
                    f,
                    "the following migrations do not have a corresponding downward migration: "
                )?;
                for (i, v) in vs {
                    write!(f, "{i}: {v}, ")?
                }
                Ok(())
            }
        }
    }
}

#[derive(PartialEq, Debug)]
enum DownwardCheck {
    No,
    IfPresent,
    Required,
}

/// Opt-in checks to validate migrations
pub struct Validations {
    downward: DownwardCheck,
}

impl Validations {
    /// Apply all possible checks, in their strictest setting. Please note that future versions of
    /// the library will add more checks and so this might cause tests to fail when you upgrade the
    /// library.
    pub fn everything() -> Self {
        Self {
            downward: DownwardCheck::Required,
        }
    }

    /// Always validate upward migrations
    pub fn upward() -> Self {
        Self {
            downward: DownwardCheck::No,
        }
    }

    /// Validate all downwards migrations found. Allow a downward migration to be missing.
    pub fn check_downward_if_present(mut self) -> Self {
        self.downward = DownwardCheck::IfPresent;
        self
    }

    /// Validate all downwards migrations found. Error if a downward migration is missing.
    pub fn require_downward(mut self) -> Self {
        self.downward = DownwardCheck::Required;
        self
    }

    /// Run the validations
    pub fn validate(&self, migrations: &Migrations) -> Result<()> {
        // Let’s have all fields in scope, to ensure we don’t forgot to use any flags (or any
        // future flags)
        let Self { downward } = self;
        let mut conn = Connection::open_in_memory()?;
        let nbr_migrations = migrations.pending_migrations(&conn)? as usize;
        if nbr_migrations == 0 {
            log::debug!("no migrations defined, they are deemed valid");
            return Ok(());
        }

        // https://mutants.rs/skip_calls.html#with_capacity
        let mut missing_downward_migrations =
            Vec::with_capacity(if *downward == DownwardCheck::Required {
                nbr_migrations
            } else {
                0
            });

        // Always check upward migrations and check downward ones depending on flags
        for i in 1..=nbr_migrations {
            log::debug!("Checking migration number {i}");
            migrations.to_version(&mut conn, i)?;
            match downward {
                DownwardCheck::No => (),
                DownwardCheck::Required | DownwardCheck::IfPresent => {
                    if migrations.ms[i - 1].down.is_some() {
                        // Revert and reapply, to see if the revert applies cleanly
                        migrations.to_version(&mut conn, i - 1)?;
                        migrations.to_version(&mut conn, i)?;
                    } else if *downward == DownwardCheck::Required {
                        let m = &migrations.ms[i - 1];
                        missing_downward_migrations.push((i, format!("{m:?}")))
                    }
                }
            };
        }

        if missing_downward_migrations.is_empty() {
            Ok(())
        } else {
            Err(Error::MissingDownwardMigrations(
                missing_downward_migrations,
            ))
        }
    }
}
