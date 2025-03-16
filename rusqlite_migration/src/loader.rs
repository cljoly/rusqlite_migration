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

use std::{convert::TryFrom, num::NonZeroUsize};

use crate::{Error, Result, M};
use include_dir::Dir;

#[derive(Debug, Clone)]
struct MigrationFile {
    id: NonZeroUsize,
    name: &'static str,
    up: &'static str,
    down: Option<&'static str>,
}

fn get_name(value: &'static Dir<'static>) -> Result<&'static str> {
    value
        .path()
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or(Error::FileLoad(format!(
            "Could not extract file name from {:?}",
            value.path()
        )))
}

#[cfg_attr(test, mutants::skip)] // Tested at a high level
fn get_migrations(
    name: &'static str,
    value: &'static Dir<'static>,
) -> Result<(&'static str, Option<&'static str>)> {
    let up = value
        .files()
        .find(|f| f.path().ends_with("up.sql"))
        .ok_or(Error::FileLoad(format!(
            "Missing upward migration file for migration {name}"
        )))?
        .contents_utf8()
        .ok_or(Error::FileLoad(format!(
            "Could not load contents from {name}/up.sql"
        )))?;

    let down = value
        .files()
        .find(|f| f.path().ends_with("down.sql"))
        .map(|down| {
            down.contents_utf8().ok_or(Error::FileLoad(format!(
                "Could not load contents from {name}/down.sql"
            )))
        })
        .transpose()?;

    Ok((up, down))
}

fn get_id(file_name: &'static str) -> Result<NonZeroUsize> {
    file_name
        .split_once('-')
        .ok_or(Error::FileLoad(format!(
            "Could not extract migration id from file name {file_name}"
        )))?
        .0
        .parse::<usize>()
        .map_err(|e| {
            Error::FileLoad(format!(
                "Could not parse migration id from file name {file_name} as usize: {e}"
            ))
        })
        .and_then(|v| {
            NonZeroUsize::new(v).ok_or(Error::FileLoad(format!(
                "{file_name} has an incorrect migration id: migration id cannot be 0"
            )))
        })
}

impl TryFrom<&'static Dir<'static>> for MigrationFile {
    type Error = Error;

    fn try_from(value: &'static Dir<'static>) -> std::result::Result<Self, Self::Error> {
        let name = get_name(value)?;
        let (up, down) = get_migrations(name, value)?;
        let id = get_id(name)?;

        Ok(MigrationFile { id, name, up, down })
    }
}

impl From<&MigrationFile> for M<'_> {
    fn from(value: &MigrationFile) -> Self {
        M::up(value.up)
            .comment(value.name)
            .down(value.down.unwrap_or_default())
    }
}

#[cfg_attr(test, mutants::skip)] // Tested at a high level
pub(crate) fn from_directory(dir: &'static Dir<'static>) -> Result<Vec<Option<M<'static>>>> {
    let mut migrations: Vec<Option<M>> = vec![None; dir.dirs().count()];

    for dir in dir.dirs() {
        let migration_file = MigrationFile::try_from(dir)?;

        let id = usize::from(migration_file.id) - 1;

        if migrations.len() <= id {
            return Err(Error::FileLoad(
                "Migration ids must be consecutive numbers".to_string(),
            ));
        }

        if migrations[id].is_some() {
            return Err(Error::FileLoad(format!(
                "Multiple migrations detected for migration id: {}",
                migration_file.id
            )));
        }

        migrations[id] = Some((&migration_file).into());
    }

    if migrations.iter().all(|m| m.is_none()) {
        return Err(Error::FileLoad(
            "Directory does not contain any migration files".to_string(),
        ));
    }

    if migrations.iter().any(|m| m.is_none()) {
        return Err(Error::FileLoad(
            "Migration ids must be consecutive numbers".to_string(),
        ));
    }

    // The values are returned in the order of the keys, i.e. of IDs
    Ok(migrations)
}
