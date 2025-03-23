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

use super::*;

fn all_errors() -> Vec<(&'static str, crate::Error)> {
    use crate::Error::*;
    use crate::ForeignKeyCheckError;
    use crate::MigrationDefinitionError;
    use crate::SchemaVersion;
    use crate::SchemaVersionError;

    vec![
        (
            "rusqlite_error",
            RusqliteError {
                query: "SELECT * FROM table42;".to_owned(),
                err: rusqlite::Error::InvalidQuery,
            },
        ),
        (
            "specified_schema_version",
            SpecifiedSchemaVersion(SchemaVersionError::TargetVersionOutOfRange {
                specified: SchemaVersion::NoneSet,
                highest: SchemaVersion::NoneSet,
            }),
        ),
        (
            "too_high_schema_version",
            SpecifiedSchemaVersion(SchemaVersionError::TooHigh),
        ),
        ("invalid_user_version", InvalidUserVersion),
        (
            "migration_definition",
            MigrationDefinition(MigrationDefinitionError::NoMigrationsDefined),
        ),
        (
            "foreign_key_check",
            ForeignKeyCheck(vec![
                ForeignKeyCheckError {
                    table: "t1".to_owned(),
                    rowid: 1,
                    parent: "t2".to_owned(),
                    fkid: 2,
                },
                ForeignKeyCheckError {
                    table: "t3".to_owned(),
                    rowid: 2,
                    parent: "t4".to_owned(),
                    fkid: 3,
                },
            ]),
        ),
        ("hook", Hook("in hook".to_owned())),
        ("file_load", FileLoad("file causing problem".to_owned())),
        (
            "unrecognized",
            Unrecognized(Box::new(Hook("unknown".to_owned()))),
        ),
    ]
}

// We should be able to convert rusqlite errors transparently
#[test]
fn test_rusqlite_error_conversion() {
    assert!(matches!(
        Error::from(rusqlite::Error::MultipleStatement),
        Error::RusqliteError { query: _, err: _ }
    ));

    let hook_error = HookError::from(rusqlite::Error::MultipleStatement);
    assert!(matches!(&hook_error, &HookError::RusqliteError(_)));
    assert!(matches!(
        Error::from(hook_error),
        Error::RusqliteError { query: _, err: _ },
    ));
}

// Check that Unrecognized errors correctly implement PartialEq, namely that
// > a != b if and only if !(a == b).
// from https://doc.rust-lang.org/std/cmp/trait.PartialEq.html
#[test]
fn test_unrecognized_errors() {
    let u1 = Error::Unrecognized(Box::new(Error::Hook(String::new())));
    let u2 = Error::Unrecognized(Box::new(Error::Hook(String::new())));
    let u3 = Error::Unrecognized(Box::new(Error::Hook("1".to_owned())));
    let u4 = Error::Unrecognized(Box::new(Error::Hook(String::new())));
    let u5 = Error::FileLoad("1".to_owned());
    let u6 = Error::Unrecognized(Box::new(Error::Hook(String::new())));

    for (e1, e2) in &[(u1, u2), (u3, u4), (u5, u6)] {
        assert!(e1 != e2);
        assert!(!(e1 == e2));
    }
}

#[test]
// Errors on specified schema versions should be equal if and only if all versions are
// equal
fn test_specified_schema_version_error() {
    assert_eq!(
        Error::SpecifiedSchemaVersion(SchemaVersionError::TargetVersionOutOfRange {
            specified: SchemaVersion::Outside(NonZeroUsize::new(10).unwrap()),
            highest: SchemaVersion::Inside(NonZeroUsize::new(4).unwrap()),
        }),
        Error::SpecifiedSchemaVersion(SchemaVersionError::TargetVersionOutOfRange {
            specified: SchemaVersion::Outside(NonZeroUsize::new(10).unwrap()),
            highest: SchemaVersion::Inside(NonZeroUsize::new(4).unwrap()),
        }),
    );
    assert_ne!(
        Error::SpecifiedSchemaVersion(SchemaVersionError::TargetVersionOutOfRange {
            specified: SchemaVersion::Outside(NonZeroUsize::new(9).unwrap()),
            highest: SchemaVersion::Inside(NonZeroUsize::new(4).unwrap()),
        }),
        Error::SpecifiedSchemaVersion(SchemaVersionError::TargetVersionOutOfRange {
            specified: SchemaVersion::Outside(NonZeroUsize::new(10).unwrap()),
            highest: SchemaVersion::Inside(NonZeroUsize::new(4).unwrap()),
        }),
    );
}

// Two errors with different queries or errors should be considered different
#[test]
fn test_rusqlite_error_query() {
    assert_ne!(
        Error::RusqliteError {
            query: "SELECTTT".to_owned(),
            err: rusqlite::Error::InvalidQuery
        },
        Error::RusqliteError {
            query: "SSSELECT".to_owned(),
            err: rusqlite::Error::InvalidQuery
        }
    );
    assert_ne!(
        Error::RusqliteError {
            query: "SELECT".to_owned(),
            err: rusqlite::Error::MultipleStatement
        },
        Error::RusqliteError {
            query: "SELECT".to_owned(),
            err: rusqlite::Error::InvalidQuery
        }
    )
}

// Two errors with different file load errors should be considered different
#[test]
fn test_rusqlite_error_file_load() {
    assert_ne!(
        Error::FileLoad("s1".to_owned()),
        Error::FileLoad("s2".to_owned())
    )
}

// Two errors with different foreign key checks should be considered different
#[test]
fn test_rusqlite_error_fkc() {
    assert_ne!(
        Error::ForeignKeyCheck(vec![ForeignKeyCheckError {
            table: "t1".to_owned(),
            rowid: 1,
            parent: "t2".to_owned(),
            fkid: 3
        }]),
        Error::ForeignKeyCheck(vec![ForeignKeyCheckError {
            table: "t1".to_owned(),
            rowid: 3,
            parent: "t2".to_owned(),
            fkid: 3
        }]),
    )
}

// Hook error conversion preserves the message
#[test]
fn test_hook_conversion_msg() {
    let msg = String::from("some error encountered in the hook");
    let hook_error = HookError::Hook(msg.clone());

    assert_eq!(Error::from(hook_error), Error::Hook(msg))
}

#[test]
fn test_schema_version_error_display() {
    let err = SchemaVersionError::TargetVersionOutOfRange {
        specified: SchemaVersion::NoneSet,
        highest: SchemaVersion::NoneSet,
    };
    assert_eq!("Attempt to migrate to version 0 (no version set), which is higher than the highest version currently supported, 0 (no version set).", format!("{err}"))
}

#[test]
fn test_foreign_key_check_error_display() {
    let err = ForeignKeyCheckError {
        table: "a".to_string(),
        rowid: 1,
        parent: "b".to_string(),
        fkid: 2,
    };
    assert_eq!("Foreign key check found row with id 1 in table 'a' missing from table 'b' but required by foreign key with id 2", format!("{err}"))
}

#[test]
fn test_migration_definition_error_display() {
    let err = MigrationDefinitionError::DownNotDefined { migration_index: 1 };
    assert_eq!(
        "Migration 1 (version 1 -> 2) cannot be reverted",
        format!("{err}")
    );

    let err = MigrationDefinitionError::DatabaseTooFarAhead;
    assert_eq!(
        "Attempt to migrate a database with a migration number that is too high",
        format!("{err}")
    );

    let err = MigrationDefinitionError::NoMigrationsDefined;
    assert_eq!(
        "Attempt to migrate with no migrations defined",
        format!("{err}")
    )
}

#[test]
fn test_error_display() {
    for (name, e) in all_errors() {
        insta::assert_snapshot!(format!("error_display__{name}"), e);
    }
}

#[test]
fn test_error_source() {
    use std::error::Error;

    for (name, e) in all_errors() {
        // For API stability reasons (if that changes, we must change the major version)
        insta::assert_debug_snapshot!(format!("error_source_number_{name}"), e.source());
    }
}

#[test]
fn schema_version_partial_display_test() {
    assert_eq!("0 (no version set)", format!("{}", SchemaVersion::NoneSet));
    assert_eq!(
        "1 (inside)",
        format!("{}", SchemaVersion::Inside(NonZeroUsize::new(1).unwrap()))
    );
    assert_eq!(
        "32 (inside)",
        format!("{}", SchemaVersion::Inside(NonZeroUsize::new(32).unwrap()))
    );
    assert_eq!(
        "1 (outside)",
        format!("{}", SchemaVersion::Outside(NonZeroUsize::new(1).unwrap()))
    );
    assert_eq!(
        "32 (outside)",
        format!("{}", SchemaVersion::Outside(NonZeroUsize::new(32).unwrap()))
    );
}

#[test]
fn error_test_source() {
    let err = Error::RusqliteError {
        query: String::new(),
        err: rusqlite::Error::InvalidQuery,
    };
    assert_eq!(
        std::error::Error::source(&err)
            .and_then(|e| e.downcast_ref::<rusqlite::Error>())
            .unwrap(),
        &rusqlite::Error::InvalidQuery
    );

    let err = Error::SpecifiedSchemaVersion(SchemaVersionError::TargetVersionOutOfRange {
        specified: SchemaVersion::NoneSet,
        highest: SchemaVersion::NoneSet,
    });
    assert_eq!(
        std::error::Error::source(&err)
            .and_then(|e| e.downcast_ref::<SchemaVersionError>())
            .unwrap(),
        &SchemaVersionError::TargetVersionOutOfRange {
            specified: SchemaVersion::NoneSet,
            highest: SchemaVersion::NoneSet
        }
    );

    let err = Error::MigrationDefinition(MigrationDefinitionError::NoMigrationsDefined);
    assert_eq!(
        std::error::Error::source(&err)
            .and_then(|e| e.downcast_ref::<MigrationDefinitionError>())
            .unwrap(),
        &MigrationDefinitionError::NoMigrationsDefined
    );

    let err = Error::ForeignKeyCheck(vec![ForeignKeyCheckError {
        table: String::new(),
        rowid: 1i64,
        parent: String::new(),
        fkid: 1i64,
    }]);
    assert_eq!(
        std::error::Error::source(&err)
            .and_then(|e| e.downcast_ref::<ForeignKeyCheckError>())
            .unwrap(),
        &ForeignKeyCheckError {
            table: String::new(),
            rowid: 1i64,
            parent: String::new(),
            fkid: 1i64,
        }
    );

    let err = Error::Hook(String::new());
    assert!(std::error::Error::source(&err).is_none());

    let err = Error::FileLoad(String::new());
    assert!(std::error::Error::source(&err).is_none());
}
