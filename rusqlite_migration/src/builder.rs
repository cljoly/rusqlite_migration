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

use std::{iter::FromIterator, mem::take};

use include_dir::Dir;

use crate::{loader::from_directory, MigrationHook, Result, M};

/// Allows to build a `Vec<M<'u>>` with additional edits.
#[derive(Default, Debug)]
pub struct MigrationsBuilder<'u> {
    migrations: Vec<Option<M<'u>>>,
}

impl<'u> MigrationsBuilder<'u> {
    /// Creates a set of migrations from a given directory by scanning subdirectories with a specified name pattern.
    /// The migrations are loaded and stored in the binary.
    ///
    /// See the [`crate::Migrations::from_directory`] method for additional information regarding the directory structure.
    ///
    /// # Example
    ///
    /// ```
    /// use rusqlite_migration::{Migrations, MigrationsBuilder};
    /// use include_dir::{Dir, include_dir};
    ///
    /// static MIGRATION_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/../examples/from-directory/migrations");
    /// let migrations: Migrations = MigrationsBuilder::from_directory(&MIGRATION_DIR).unwrap().finalize();
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::FileLoad`] in case the subdirectory names are incorrect,
    /// or don't contain at least a valid `up.sql` file.
    #[cfg_attr(test, mutants::skip)] // Tested at a high level
    pub fn from_directory(dir: &'static Dir<'static>) -> Result<Self> {
        Ok(Self {
            migrations: from_directory(dir)?,
        })
    }

    /// Allows to edit a migration with a given `id`.
    ///
    /// # Panics
    ///
    /// Panics if no migration with the `id` provided exists.
    #[must_use]
    pub fn edit(mut self, id: usize, f: impl Fn(M) -> M) -> Self {
        if id < 1 {
            panic!("id cannot be equal to 0");
        }
        self.migrations[id - 1] = take(&mut self.migrations[id - 1]).map(f);
        self
    }

    /// Finalizes the builder and creates a [`crate::Migrations`].
    pub fn finalize(mut self) -> crate::Migrations<'u> {
        self.migrations.drain(..).flatten().collect()
    }
}

impl<'u> FromIterator<M<'u>> for MigrationsBuilder<'u> {
    fn from_iter<T: IntoIterator<Item = M<'u>>>(iter: T) -> Self {
        Self {
            migrations: Vec::from_iter(iter.into_iter().map(Some)),
        }
    }
}

impl M<'_> {
    /// Replace the `up_hook` in the given migration with the provided one.
    ///
    /// # Warning
    ///
    /// Use [`M::up_with_hook`] instead if you're creating a new migration.
    /// This method is meant for editing existing transactions
    /// when using the [`MigrationsBuilder`].
    pub fn set_up_hook(mut self, hook: impl MigrationHook + 'static) -> Self {
        self.up_hook = Some(hook.clone_box());
        self
    }

    /// Replace the `down_hook` in the given migration with the provided one.
    ///
    /// # Warning
    ///
    /// Use [`M::down_with_hook`] instead if you're creating a new migration.
    /// This method is meant for editing existing transactions
    /// when using the [`MigrationsBuilder`].
    pub fn set_down_hook(mut self, hook: impl MigrationHook + 'static) -> Self {
        self.down_hook = Some(hook.clone_box());
        self
    }
}
