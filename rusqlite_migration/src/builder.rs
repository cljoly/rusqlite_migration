use std::iter::FromIterator;

use include_dir::Dir;

use crate::{loader::from_directory, MigrationHook, Result, M};

/// Allows to build a `Vec<M<'u>>` with additional edits.
#[derive(Default, Debug)]
pub struct MigrationsBuilder<'u> {
    migrations: Vec<M<'u>>,
}

impl<'u> MigrationsBuilder<'u> {
    /// Creates a set of migrations from a given directory by scanning subdirectories with a specified name pattern.
    /// The migrations are loaded and stored in the binary.
    ///
    /// See the [`Migrations::from_directory`] method for additional information regarding the directory structure.
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
    /// Errors:
    ///
    /// Returns [`Error::FileLoad`] in case the subdirectory names are incorrect,
    /// or don't contain at least a valid `up.sql` file.
    pub fn from_directory(dir: &'static Dir<'static>) -> Result<Self> {
        Ok(Self {
            migrations: from_directory(dir)?,
        })
    }

    /// Allows to edit a migration with a given `id`.
    ///
    /// Errors:
    ///
    /// Panics in case a migration with a given `id` does not exist.
    pub fn edit(mut self, id: usize, f: impl Fn(M) -> M) -> Self {
        if id < 1 {
            panic!("id cannot be equal to 0");
        }
        f(self
            .migrations
            .get_mut(id - 1)
            .expect("No migration with the given index"));
        self
    }

    /// Finalizes the builder and creates a either a [`crate::Migrations`] or [`crate::asynch::AsyncMigration`] instance.
    pub fn finalize<T: FromIterator<M<'u>>>(self) -> T {
        T::from_iter(self.migrations)
    }
}

impl<'u> FromIterator<M<'u>> for MigrationsBuilder<'u> {
    fn from_iter<T: IntoIterator<Item = M<'u>>>(iter: T) -> Self {
        Self {
            migrations: Vec::from_iter(iter),
        }
    }
}

impl<'u> M<'u> {
    /// Replace the `up_hook` in the given migration with the provided one.
    ///
    /// # Warning
    ///
    /// Use [`M::up_with_hook`] instead if you're creating a new migration.
    /// This method is meant for editing existing transactions
    /// when using the [`MigrationsBuilder`].
    pub fn set_up_hook(&mut self, hook: impl MigrationHook + 'static) {
        self.up_hook = Some(hook.clone_box());
    }

    /// Replace the `down_hook` in the given migration with the provided one.
    ///
    /// # Warning
    ///
    /// Use [`M::down_with_hook`] instead if you're creating a new migration.
    /// This method is meant for editing existing transactions
    /// when using the [`MigrationsBuilder`].
    pub fn set_down_hook(&mut self, hook: impl MigrationHook + 'static) {
        self.down_hook = Some(hook.clone_box());
    }
}
