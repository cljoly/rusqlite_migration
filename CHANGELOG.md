# Changelog

## Version 1.2.0 Alpha 1

### Minimum Rust Version

Now using edition 2021

## Version 1.1.0

*Same code as version 1.1.0-beta.1*

### Minimum Rust Version

Rust 1.70

### New Features

* Support for tokio-rusqlite behind the feature named `alpha-async-tokio-rusqlite`thanks to [@czocher](https://github.com/czocher). See [the example](https://github.com/cljoly/rusqlite_migration/tree/c54951d22691432fbfd511cc68f1c5b8a2306737/examples/async). This feature is alpha, meaning that compatibility in future minor versions is not guaranteed.
* Create migrations from directories holding SQL files thanks to [@czocher](https://github.com/czocher). See [the example](https://github.com/cljoly/rusqlite_migration/tree/af4da527ff75e3b8c089d2300cab7fbe66096411/examples/from-directory).
* Add up/down hooks to run custom Rust code during migrations ([PR](https://github.com/cljoly/rusqlite_migration/pull/28) thanks to [@matze](https://github.com/matze))
* Add foreign_key_check method to migrations ([PR](https://github.com/cljoly/rusqlite_migration/pull/20) thanks to [@Jokler](https://github.com/Jokler))
* Make `Migration` functions const ([PR](https://github.com/cljoly/rusqlite_migration/pull/19) thanks to [@fkaa](https://github.com/fkaa))
* Make `Migrations` serializable (using the Debug serializer) with [insta](https://insta.rs).

### Depreciation

* Mark `Migrations::from_iter` as deprecated

### Other

* Documentation improvements
    * Repository metadata improvements
* Code quality improvements
    * Introduce cargo mutants & fix bugs found
    * Clippy warning fixes and other linter improvements
    * Report on test coverage & improve test coverage
    * Add benchmarks
* Made errors returned more precise
* Updated dependencies

### See also

Rusqlite was updated from 0.29.0 to 0.30.0. Please see [its release notes](https://github.com/rusqlite/rusqlite/releases/tag/v0.30.0)

## Version 1.1.0 Beta 1

**⚠️ The APIs exposed in this version may be unstable.**

Summing up all the changes from the previous Alpha versions.

### Minimum Rust Version

Rust 1.70

### New Features

* Support for tokio-rusqlite behind the feature named `alpha-async-tokio-rusqlite`thanks to [@czocher](https://github.com/czocher). See [the example](https://github.com/cljoly/rusqlite_migration/tree/c54951d22691432fbfd511cc68f1c5b8a2306737/examples/async). This feature is alpha, meaning that compatibility in future minor versions is not guaranteed.
* Create migrations from directories holding SQL files thanks to [@czocher](https://github.com/czocher). See [the example](https://github.com/cljoly/rusqlite_migration/tree/af4da527ff75e3b8c089d2300cab7fbe66096411/examples/from-directory).
* Add up/down hooks to run custom Rust code during migrations ([PR](https://github.com/cljoly/rusqlite_migration/pull/28) thanks to [@matze](https://github.com/matze))
* Add foreign_key_check method to migrations ([PR](https://github.com/cljoly/rusqlite_migration/pull/20) thanks to [@Jokler](https://github.com/Jokler))
* Make `Migration` functions const ([PR](https://github.com/cljoly/rusqlite_migration/pull/19) thanks to [@fkaa](https://github.com/fkaa))
* Make `Migrations` serializable (using the Debug serializer) with [insta](https://insta.rs).

### Depreciation

* Mark `Migrations::from_iter` as deprecated

### Other

* Documentation improvements
    * Repository metadata improvements
* Code quality improvements
    * Introduce cargo mutants & fix bugs found
    * Clippy warning fixes and other linter improvements
    * Report on test coverage & improve test coverage
    * Add benchmarks
* Made errors returned more precise
* Updated dependencies

### See also

Rusqlite was updated from 0.29.0 to 0.30.0. Please see [its release notes](https://github.com/rusqlite/rusqlite/releases/tag/v0.30.0)

## Version 1.1.0 Alpha 2

**⚠️ The APIs exposed in this version may be unstable.**

### Minimum Rust Version

Rust 1.64

### New Features

* Create migrations from directories holding SQL files. See [the example](https://github.com/cljoly/rusqlite_migration/tree/af4da527ff75e3b8c089d2300cab7fbe66096411/examples/from-directory).

### Depreciation

* Mark `Migrations::from_iter` as deprecated

### Other

* Documentation improvements
* Code quality improvements
    * Introduce cargo mutants & fix bugs found
    * Clippy warning fixes
    * Report on test coverage & improve test coverage
    * Add benchmarks
* Made errors returned more precise
* Update dependencies

## Version 1.1.0 Alpha 1

**⚠️ The APIs exposed in this version may be unstable.**

### Minimum Rust Version

Rust 1.61

### New Features

* Add up/down hooks to run custom Rust code during migrations ([PR](https://github.com/cljoly/rusqlite_migration/pull/28) thanks to [@matze](https://github.com/matze))
  * The purpose of this release is to get feedback on the new API. Please feel free to comment on [this discussion](https://github.com/cljoly/rusqlite_migration/discussions/36)!
* Add foreign_key_check method to migrations ([PR](https://github.com/cljoly/rusqlite_migration/pull/20) thanks to [@Jokler](https://github.com/Jokler))
  * Please beware of the [follow up work needed on this](https://github.com/cljoly/rusqlite_migration/issues/4#issuecomment-1166363260)
* Make `Migration` functions const ([PR](https://github.com/cljoly/rusqlite_migration/pull/19) thanks to [@fkaa](https://github.com/fkaa))

### Other

* CI improvements
* Linter improvements
* Repository metadata improvements
* Documentation improvements
* Dev dependencies update (not dependencies of the library when used in another crate)

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


