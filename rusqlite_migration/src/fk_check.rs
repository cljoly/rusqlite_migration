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

//! Enforce foreign key constraints

use std::cell::OnceCell;

use rusqlite::{Statement, Transaction};

use crate::{Error, ForeignKeyCheckError, Result};

const PRAGMA_FK_CHECK: &str = "SELECT * FROM pragma_foreign_key_check;";

pub(crate) struct FKCheck<'conn> {
    // Store a result here so Self is easier to lazily initialize
    stmt: OnceCell<Statement<'conn>>,
}

impl<'conn> FKCheck<'conn> {
    pub(crate) fn new() -> Self {
        Self {
            stmt: OnceCell::new(),
        }
    }

    /// Validate that no foreign keys are violated
    pub(crate) fn validate(&mut self, conn: &'conn Transaction) -> Result<()> {
        // Not great, but get_mut_or_init is still in nightly
        if self.stmt.get().is_none() {
            self.stmt
                .set(
                    conn.prepare(PRAGMA_FK_CHECK)
                        .map_err(|e| crate::Error::with_sql(e, PRAGMA_FK_CHECK))?,
                )
                .expect("OnceCell was checked above and is empty");
        }
        let stmt = self
            .stmt
            .get_mut()
            .expect("the OnceCell was initialize just above");

        let fk_errors = stmt
            .query_map([], |row| {
                Ok(ForeignKeyCheckError {
                    table: row.get(0)?,
                    rowid: row.get(1)?,
                    parent: row.get(2)?,
                    fkid: row.get(3)?,
                })
            })
            .map_err(|e| Error::with_sql(e, PRAGMA_FK_CHECK))?
            .collect::<Result<Vec<_>, _>>()?;

        if !fk_errors.is_empty() {
            Err(crate::Error::ForeignKeyCheck(fk_errors))
        } else {
            Ok(())
        }
    }
}
