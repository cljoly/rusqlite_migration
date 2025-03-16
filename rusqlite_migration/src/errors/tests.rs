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
