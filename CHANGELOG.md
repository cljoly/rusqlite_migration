# Changelog

## Version 1.0.2

### Bug fix

* fix: adapt to rusqlite 0.29 and tighten dependency requirements for rusqlite (see [this discussion](https://github.com/cljoly/rusqlite_migration/issues/68#issuecomment-1485795284))

## Version 1.0.1

### Bug Fix

* fix: error instead of panicking on higher migration level (see commit ad57d92d1677420eb81c4e25635be1884f9b7ce7)

### Other

* Documentation improvements

## Version 1.0.0

### Breaking changes

* Remove deprecated symbols (`Migrations.latest`, `SchemaVersionError::MigrateToLowerNotSupported`)

### Other

* Documentation improvements

## Version 0.5.1

### Potentially Breaking Changes
- Update the `rusqlite` crate (to protect agaisnt [RUSTSEC-2020-0014](https://rustsec.org/advisories/RUSTSEC-2020-0014.html))

### Other
- Improve the documentation

## Version 0.5.0

- Update the `env_logger` dependency
- Improve the documentation

## Version 0.4.1 / 0.4.2

- Update documentation

## Version 0.4.0

### New features

- Add downward migrations, i.e. migrations to go to past schema version of the database. Thanks @MightyPork!
- Unsafe code is now forbidden.

### Breaking changes

- Rename `latest` to `to_latest`. The old symbol is deprecated and will be removed eventually.
- An error is now returned when a migration is attempted while no migrations exist.

### Other

- Improve general rust API documentation.
- Generate parts of the readme based on rust comments, for increased consistency with the docs.rs content.
- Various refactoring and clean-ups.

## Version 0.3.1

Fix in readme, for crates.io

## Version 0.3

### New features

- Multi line sql statements like:
	```
	M::up(r#"
	CREATE TABLE t1(a, b);
	CREATE TABLE t2(a, b);
	"#)
	```
    are now fully supported

### Other

- Various doc & CI improvements
- Fix a case of failure with silent errors.


