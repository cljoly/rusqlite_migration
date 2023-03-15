use std::{collections::HashMap, convert::TryFrom, num::NonZeroUsize};

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

impl<'u> From<&MigrationFile> for M<'u> {
    fn from(value: &MigrationFile) -> Self {
        M::up(value.up)
            .comment(value.name)
            .down(value.down.unwrap_or_default())
    }
}

pub(crate) fn from_directory(dir: &'static Dir<'static>) -> Result<Vec<M<'static>>> {
    let mut migrations = dir
        .dirs()
        .map(MigrationFile::try_from)
        .collect::<Result<Vec<MigrationFile>>>()?;

    if migrations.is_empty() {
        return Err(Error::FileLoad(
            "Directory does not contain any migration files".to_string(),
        ));
    }

    migrations.sort_by_key(|m| m.id);

    let ids: HashMap<NonZeroUsize, u32> = migrations.iter().map(|entry| (entry.id, 1)).fold(
        HashMap::new(),
        |mut acc, (id, _count)| {
            acc.entry(id).and_modify(|count| *count += 1).or_insert(1);
            acc
        },
    );

    if let Some(duplicte) = ids.iter().find(|(&_id, &count)| count > 1) {
        return Err(Error::FileLoad(format!(
            "Multiple migrations detected for migration id: {}",
            duplicte.0
        )));
    }

    let has_consecutive_ids = migrations
        .iter()
        .last()
        .map(|m| m.id)
        .map_or(migrations.is_empty(), |k| k.get() == migrations.len());

    if !has_consecutive_ids {
        return Err(Error::FileLoad(
            "Migration ids must be consecutive numbers".to_string(),
        ));
    }

    Ok(migrations.iter().map(M::from).collect())
}
