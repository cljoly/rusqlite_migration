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

use std::num::NonZeroUsize;

use include_dir::{include_dir, Dir};

use rusqlite::{params, Connection};
use rusqlite_migration::{Error, Migrations, SchemaVersion};

static MIGRATIONS_DIR: Dir =
    include_dir!("$CARGO_MANIFEST_DIR/../examples/from-directory/migrations");
static JUST_DOWN: Dir = include_dir!("$CARGO_MANIFEST_DIR/tests/migrations/just_down");
static EMPTY_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/tests/migrations/empty_dir");
static NON_CONSECUTIVE: Dir = include_dir!("$CARGO_MANIFEST_DIR/tests/migrations/non_consecutive");
static MULTIPLE: Dir = include_dir!("$CARGO_MANIFEST_DIR/tests/migrations/multiple");
static ZERO_AS_ID: Dir = include_dir!("$CARGO_MANIFEST_DIR/tests/migrations/zero_as_id");
static INVALID_UTF8: Dir = include_dir!("$CARGO_MANIFEST_DIR/tests/migrations/invalid_utf8");
static TOO_LARGE_MIGRATION_ID: Dir =
    include_dir!("$CARGO_MANIFEST_DIR/tests/migrations/too_large_migration_id");
static BAD_MIGRATION_ID: Dir =
    include_dir!("$CARGO_MANIFEST_DIR/tests/migrations/bad_migration_id");
static MISSING_MIGRATION_ID: Dir =
    include_dir!("$CARGO_MANIFEST_DIR/tests/migrations/missing_migration_id");

#[test]
fn main_test() {
    let mut conn = Connection::open_in_memory().unwrap();
    {
        let migrations = Migrations::from_directory(&MIGRATIONS_DIR).unwrap();

        migrations.to_latest(&mut conn).unwrap();

        assert_eq!(
            Ok(SchemaVersion::Inside(NonZeroUsize::new(3).unwrap())),
            migrations.current_version(&conn)
        );

        conn.execute(
            "INSERT INTO friend (name, birthday) VALUES (?1, ?2)",
            params!["John", "1970-01-01"],
        )
        .unwrap();
    }
}

#[test]
fn utf8_test() {
    let migrations = Migrations::from_directory(&INVALID_UTF8);
    assert_eq!(
        Error::FileLoad("Could not load contents from 01-invalid_utf8/up.sql".to_string()),
        migrations.unwrap_err()
    )
}

#[test]
fn missing_id_test() {
    let migrations = Migrations::from_directory(&MISSING_MIGRATION_ID);
    assert_eq!(
        Error::FileLoad("Could not extract migration id from file name friend_car".to_string()),
        migrations.unwrap_err()
    )
}

#[test]
fn bad_migration_id() {
    let migrations = Migrations::from_directory(&BAD_MIGRATION_ID);
    assert_eq!(
        Error::FileLoad("Could not parse migration id from file name a-friend_car as usize: invalid digit found in string".to_string()),
        migrations.unwrap_err()
    )
}

#[test]
fn too_large_migration_id() {
    let migrations = Migrations::from_directory(&TOO_LARGE_MIGRATION_ID);
    assert_eq!(
        Error::FileLoad("Could not parse migration id from file name 18446744073709551616-friend_car as usize: number too large to fit in target type".to_string()),
        migrations.unwrap_err()
    )
}

#[test]
fn zero_as_id() {
    let migrations = Migrations::from_directory(&ZERO_AS_ID);
    assert_eq!(
        Error::FileLoad(
            "00-friend_car has an incorrect migration id: migration id cannot be 0".to_string()
        ),
        migrations.unwrap_err()
    )
}

#[test]
fn just_down() {
    let migrations = Migrations::from_directory(&JUST_DOWN);
    assert_eq!(
        Error::FileLoad("Missing upward migration file for migration 01-friend_car".to_string()),
        migrations.unwrap_err()
    )
}

#[test]
fn multiple_up() {
    let migrations = Migrations::from_directory(&MULTIPLE);
    assert_eq!(
        Error::FileLoad("Multiple migrations detected for migration id: 1".to_string()),
        migrations.unwrap_err()
    )
}

#[test]
fn empty_dir() {
    let migrations = Migrations::from_directory(&EMPTY_DIR);
    assert_eq!(
        Error::FileLoad("Directory does not contain any migration files".to_string()),
        migrations.unwrap_err()
    )
}

#[test]
fn non_consecutive() {
    let migrations = Migrations::from_directory(&NON_CONSECUTIVE);
    assert_eq!(
        Error::FileLoad("Migration ids must be consecutive numbers".to_string()),
        migrations.unwrap_err()
    )
}

#[test]
// Ensure that we have a healthy mix of files with an end of line (EOL) at the end and of files
// without.
fn eol_does_not_matter_test() {
    let no_eol =
        include_str!("../../examples/from-directory/migrations/02-add_birthday_column/up.sql");
    let eol = include_str!("../../examples/from-directory/migrations/01-friend_car/up.sql");

    assert_ne!(no_eol.chars().last().unwrap(), '\n');
    assert_eq!(eol.chars().last().unwrap(), '\n');
}
