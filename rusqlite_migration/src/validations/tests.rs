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

use super::*;
use crate::Migrations;

use crate::tests::helpers::{
    all_valid_down, m_valid0_up, m_valid11_up, m_valid20_up, m_valid_fk_up,
};

#[test]
fn test_empty_migrations() {
    let migrations = Migrations::from_slice(&[]);
    assert_eq!(Validations::upward().validate(&migrations), Ok(()));
    assert_eq!(Validations::everything().validate(&migrations), Ok(()));
}

#[test]
fn test_missing_downward_start() {
    let mut missing_start = all_valid_down();
    missing_start[0] = m_valid0_up();

    let migrations = Migrations::new(missing_start);
    insta::assert_debug_snapshot!(Validations::everything().validate(&migrations));

    // Additional checks for consistency
    assert_eq!(Validations::upward().validate(&migrations), Ok(()));
    {
        use std::error::Error;

        let v = Validations::everything().validate(&migrations).unwrap_err();
        insta::assert_snapshot!("error_missing_downward_pretty_print", v);
        assert!(v.source().is_none());
    }
}

#[test]
fn test_missing_downward_middle() {
    let mut missing_middle = all_valid_down();
    missing_middle[3] = m_valid20_up();

    let migrations = Migrations::new(missing_middle);
    insta::assert_debug_snapshot!(Validations::everything().validate(&migrations));
}

#[test]
fn test_missing_downward_end() {
    let mut missing_end = all_valid_down();
    let len = missing_end.len();
    missing_end[len - 1] = m_valid_fk_up();

    let migrations = Migrations::new(missing_end);
    insta::assert_debug_snapshot!(Validations::everything().validate(&migrations));
}

#[test]
fn test_missing_downward_multiple() {
    let mut missing_multiple = all_valid_down();
    let len = missing_multiple.len();
    missing_multiple[2] = m_valid11_up();
    missing_multiple[3] = m_valid20_up();
    missing_multiple[len - 1] = m_valid_fk_up();

    let migrations = Migrations::new(missing_multiple);
    insta::assert_debug_snapshot!(
        "multiple_missing_multiple_errors",
        Validations::everything().validate(&migrations)
    );
}

#[test]
fn test_invalid_down_migration() {
    use std::error::Error;

    let mut invalid_start = all_valid_down();
    invalid_start[0] = m_valid0_up().down("Invalid sql");

    let migrations = Migrations::new(invalid_start);
    let v = Validations::everything().validate(&migrations);

    insta::assert_debug_snapshot!("full_error", v);
    assert_eq!(
        Validations::upward()
            .require_downward()
            .validate(&migrations),
        v
    );
    assert_eq!(
        Validations::upward()
            .check_downward_if_present()
            .validate(&migrations),
        v
    );
    assert!(Validations::upward().validate(&migrations).is_ok());

    insta::assert_debug_snapshot!("source", v.unwrap_err().source());
}

#[test]
fn test_rusqlite_error_conversion() {
    let rusqlite_e = rusqlite::Error::InvalidQuery;
    let e = Error::from(rusqlite_e);
    assert!(matches!(
        e,
        Error::RusqliteMigration(crate::Error::RusqliteError { .. })
    ));
    insta::assert_snapshot!(e)
}
