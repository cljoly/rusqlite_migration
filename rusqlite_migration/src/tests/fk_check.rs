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

use rusqlite::Connection;

use crate::tests::helpers::{m_invalid_fk, m_invalid_fk_down, m_valid0_up, m_valid_fk_up};
use crate::{Error, Migrations};

#[test]
fn valid_fk_check_test() {
    assert_eq!(Ok(()), Migrations::new(vec![m_valid_fk_up()]).validate())
}

#[test]
fn invalid_fk_check_test() {
    let migrations = Migrations::new(vec![m_invalid_fk()]);
    insta::assert_debug_snapshot!(migrations.validate());

    let migrations = Migrations::new(vec![m_valid0_up(), m_invalid_fk()]);
    insta::assert_debug_snapshot!(migrations.validate());
}

#[test]
fn invalid_down_fk_check_test() {
    let migrations = Migrations::new(vec![m_invalid_fk_down()]);

    let mut conn = Connection::open_in_memory().unwrap();
    migrations.to_latest(&mut conn).unwrap();

    assert!(matches!(
        migrations.to_version(&mut conn, 0),
        Err(Error::ForeignKeyCheck(_))
    ));
}
