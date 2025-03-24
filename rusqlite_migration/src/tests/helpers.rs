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

use crate::M;

use rusqlite::Connection;

/// Attempt to set the user version in the whole possible range of values (even negative ones or
/// values that don’t fit in the underlying 4 bytes field)
pub fn raw_set_user_version(conn: &mut Connection, version: isize) {
    conn.pragma_update(None, "user_version", version).unwrap()
}

pub fn m_valid0() -> M<'static> {
    M::up("CREATE TABLE m1(a, b); CREATE TABLE m2(a, b, c);")
}
pub fn m_valid10() -> M<'static> {
    M::up("CREATE TABLE t1(a, b);")
}
pub fn m_valid11() -> M<'static> {
    M::up("ALTER TABLE t1 RENAME COLUMN b TO c;")
}
pub fn m_valid20() -> M<'static> {
    M::up("CREATE TABLE t2(b);")
}
pub fn m_valid21() -> M<'static> {
    M::up("ALTER TABLE t2 ADD COLUMN a;")
}

pub fn m_valid_fk() -> M<'static> {
    M::up(
        r#"
        CREATE TABLE fk1(a PRIMARY KEY);
        CREATE TABLE fk2(
            a,
            FOREIGN KEY(a) REFERENCES fk1(a)
        );
        INSERT INTO fk1 (a) VALUES ('foo');
        INSERT INTO fk2 (a) VALUES ('foo');
    "#,
    )
    .foreign_key_check()
}

pub fn m_invalid_down_fk() -> M<'static> {
    M::up(
        r#"
        CREATE TABLE fk1(a PRIMARY KEY);
        CREATE TABLE fk2(
            a,
            FOREIGN KEY(a) REFERENCES fk1(a)
        );
        INSERT INTO fk1 (a) VALUES ('foo');
        INSERT INTO fk2 (a) VALUES ('foo');
    "#,
    )
    .foreign_key_check()
    .down("DROP TABLE fk1;")
}

// All valid Ms in the right order
pub fn all_valid() -> Vec<M<'static>> {
    vec![
        m_valid0(),
        m_valid10(),
        m_valid11(),
        m_valid20(),
        m_valid21(),
        m_valid_fk(),
    ]
}

pub fn m_invalid0() -> M<'static> {
    M::up("CREATE TABLE table3()")
}
pub fn m_invalid1() -> M<'static> {
    M::up("something invalid")
}

pub fn m_invalid_fk() -> M<'static> {
    M::up(
        r#"
        CREATE TABLE fk1(a PRIMARY KEY);
        CREATE TABLE fk2(
            a,
            FOREIGN KEY(a) REFERENCES fk1(a)
        );
        INSERT INTO fk2 (a) VALUES ('foo');
        INSERT INTO fk2 (a) VALUES ('bar');
    "#,
    )
    .foreign_key_check()
}
