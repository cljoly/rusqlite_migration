<!-- insert
---
title: "Rusqlite Changelog"
date: 2025-03-24T20:32:05
tags:
- Rust
- SQLite
- Library
---
end_insert -->

<!-- remove -->
# Changelog
<!-- end_remove -->

<!-- insert
Release notes for the [rusqlite_migration library](https://cj.rs/rusqlite_migration).
end_insert -->

## Version 2.3.0

### Dependencies

Rusqlite was updated from 0.36.0 to 0.37.0.
Please see [the release notes for 0.37.0](https://github.com/rusqlite/rusqlite/releases/tag/v0.37.0).

### Other

- Misc. clippy fixes
- Minor improvements to the example in the Readme

## Version 2.2.0

> [!NOTE]
> The code of this version is identical to [Version 2.2.0 Beta 1](#version-220-beta-1)

### Features

- Implement the `Display` trait for `M`. This makes it easier to print errors pertaining to a particular migration (this feature is planned for the future, in the context of more extensive migration checks)

### Dependencies

Rusqlite was updated from 0.35.0 to 0.36.0.
Please see [the release notes for 0.36.0](https://github.com/rusqlite/rusqlite/releases/tag/v0.36.0).

### Other

- Update development dependencies
- Improve tests to cover more cases, in particular around downward migrations
- Add docs.rs link to Cargo metadata
- Fix clippy warning in rust 1.87.0

## Version 2.2.0 Beta 1

### Features

- Implement the `Display` trait for `M`. This makes it easier to print errors pertaining to a particular migration (this feature is planned for the future, in the context of more extensive migration checks)

### Dependencies

Rusqlite was updated from 0.35.0 to 0.36.0.
Please see [the release notes for 0.36.0](https://github.com/rusqlite/rusqlite/releases/tag/v0.36.0).

### Other

- Update development dependencies
- Improve tests to cover more cases, in particular around downward migrations
- Add docs.rs link to Cargo metadata
- Fix clippy warning in rust 1.87.0

## Version 2.1.0

### Dependencies

Rusqlite was updated from 0.34.0 to 0.34.0.
Please see [the release notes for 0.35.0](https://github.com/rusqlite/rusqlite/releases/tag/v0.35.0).

## Version 2.0.0

### Breaking changes

#### Remove the `alpha-async-tokio-rusqlite` Feature

As the name of the feature suggest, we have had experimental support for async using tokio for a while now. Supporting that feature has been quite a big burden, introducing some duplicated code in the `AsyncMigrations` struct in particular, as well as a whole set of very similar tests. Plus the benefit of async is limited here, because everything gets executed in a blocking fashion in sqlite anyway.

It turns out that we don’t need the async support in rusqlite_migration for folks to use async libraries. For instance, with tokio-rusqlite, you can define migrations like in the sync context and run:
```rust
    async_conn
        .call_unwrap(|conn| MIGRATIONS.to_latest(conn))
        .await?;
```

See [the updated async example](https://github.com/cljoly/rusqlite_migration/blob/master/examples/async/src/main.rs) for details, in particular why it’s fine to call [a method](https://docs.rs/tokio-rusqlite/0.6.0/tokio_rusqlite/struct.Connection.html#method.call_unwrap) with unwrap in its name.

#### Make the Builder `Finalizer` Method Not Generic

On a related note, now that we have removed the `AsyncMigrations` (see the section right above) struct, we only have `Migrations` so there is no need for the `MigrationsBuilder.finalize` method to be generic. Thus we removed the generic argument. To update your code, you can just do this:
```diff
-        .finalize::<Migrations>());
+        .finalize());
```
#### Remove `Migrations::new_iter`

This function has been deprecated for a while now, remove it as a part of the major version bump. You can use the standard `FromIter` trait implementation instead.

### Behavior Change

* When the [user version field](https://www.sqlite.org/fileformat.html#user_version_number) is altered by other code in your application, we are now returning an explicit error (`Error::InvalidUserVersion`) when this can be detected. Previously, the library would silently misbehave.

### Features

- Add the new [`Migrations::from_slice`](https://docs.rs/rusqlite_migration/2.0.0-beta.1/rusqlite_migration/struct.Migrations.html#method.from_slice) constructor, which is `const` and takes a slice, so that it can be constructed in global constant, without using `LazyLock` or similar. Internally, this is possible because we now use a [`Cow`](https://doc.rust-lang.org/std/borrow/enum.Cow.html) structure to hold migrations.
- Add [`Migrations::pending_migrations`](https://docs.rs/rusqlite_migration/2.0.0-beta.1/rusqlite_migration/struct.Migrations.html#method.pending_migrations) which returns the number of migrations that would be applied. This is mostly useful to take a backup of the database prior to applying migrations (and do nothing if no migrations will be applied).


### Dependencies

Rusqlite was updated from 0.32.1 to 0.34.0.
Please see [the release notes for 0.34.0](https://github.com/rusqlite/rusqlite/releases/tag/v0.34.0) and
[the release notes for 0.33.0](https://github.com/rusqlite/rusqlite/releases/tag/v0.33.0).
Tokio Rusqlite was removed as a dependency.

### Minimum Rust Version

Rust 1.84.

Moving forward, we expect to keep this aligned with rusqlite itself, now that it has a [policy](https://github.com/rusqlite/rusqlite?tab=readme-ov-file#minimum-supported-rust-version-msrv) (introduced in [october 2024](https://github.com/rusqlite/rusqlite/pull/1576)).

## Version 2.0.0 Beta 1

### Features

- Add the new [`Migrations::from_slice`](https://docs.rs/rusqlite_migration/2.0.0-beta.1/rusqlite_migration/struct.Migrations.html#method.from_slice) constructor, which is `const` and takes a slice, so that it can be constructed in global constant, without using `LazyLock` or similar. Internally, this is possible because we now use a [`Cow`](https://doc.rust-lang.org/std/borrow/enum.Cow.html) structure to hold migrations.
- Add [`Migrations::pending_migrations`](https://docs.rs/rusqlite_migration/2.0.0-beta.1/rusqlite_migration/struct.Migrations.html#method.pending_migrations) which returns the number of migrations that would be applied. This is mostly useful to take a backup of the database prior to applying migrations (and do nothing if no migrations will be applied).

## Version 2.0.0 Alpha 1

### Breaking changes

#### Remove the `alpha-async-tokio-rusqlite` Feature

As the name of the feature suggest, we have had experimental support for async using tokio for a while now. Supporting that feature has been quite a big burden, introducing some duplicated code in the `AsyncMigrations` struct in particular, as well as a whole set of very similar tests. Plus the benefit of async is limited here, because everything gets executed in a blocking fashion in sqlite anyway.

It turns out that we don’t need the async support in rusqlite_migration for folks to use async libraries. For instance, with tokio-rusqlite, you can define migrations like in the sync context and run:
```rust
    async_conn
        .call_unwrap(|conn| MIGRATIONS.to_latest(conn))
        .await?;
```

See [the updated async example](https://github.com/cljoly/rusqlite_migration/blob/master/examples/async/src/main.rs) for details, in particular why it’s fine to call [a method](https://docs.rs/tokio-rusqlite/0.6.0/tokio_rusqlite/struct.Connection.html#method.call_unwrap) with unwrap in its name.

#### Make the Builder `Finalizer` Method Not Generic

On a related note, now that we have removed the `AsyncMigrations` (see the section right above) struct, we only have `Migrations` so there is no need for the `MigrationsBuilder.finalize` method to be generic. Thus we removed the generic argument. To update your code, you can just do this:
```diff
-        .finalize::<Migrations>());
+        .finalize());
```
#### Remove `Migrations::new_iter`

This function has been deprecated for a while now, remove it as a part of the major version bump. You can use the standard `FromIter` trait implementation instead.

### Behavior Change

* When the [user version field](https://www.sqlite.org/fileformat.html#user_version_number) is altered by other code in your application, we are now returning an explicit error (`Error::InvalidUserVersion`) when this can be detected. Previously, the library would silently misbehave.

### Dependencies

Rusqlite was updated from 0.32.1 to 0.34.0.
Please see [the release notes for 0.34.0](https://github.com/rusqlite/rusqlite/releases/tag/v0.34.0) and
[the release notes for 0.33.0](https://github.com/rusqlite/rusqlite/releases/tag/v0.33.0).
Tokio Rusqlite was removed as a dependency.

### Features

- `Migrations::new` is now `const`

### Minimum Rust Version

Rust 1.84.

Moving forward, we expect to keep this aligned with rusqlite itself, now that it has a [policy](https://github.com/rusqlite/rusqlite?tab=readme-ov-file#minimum-supported-rust-version-msrv) (introduced in [october 2024](https://github.com/rusqlite/rusqlite/pull/1576)).

## Version 1.3.1

The only change is a fix to the deps.rs badge in the documentation.

## Version 1.3.0

> [!NOTE]
> The code of this version is identical to [Version 1.3.0 Beta 1](#version-130-beta-1)

Rusqlite was updated from 0.31.0 to 0.32.1.
Please see [the release notes for 0.32.0](https://github.com/rusqlite/rusqlite/releases/tag/v0.32.0) and
[for 0.32.1](https://github.com/rusqlite/rusqlite/releases/tag/v0.32.1).
Tokio Rusqlite was updated from 0.5.1 to 0.6.0.
Please see the [release notes](https://github.com/programatik29/tokio-rusqlite/releases/tag/v0.6.0).

### Minimum Rust Version

Rust 1.77

### Documentation

Various documentation improvements and clarification. In particular, call out that if a rusqlite error is encountered during a migration, the next migrations in the list are not applied.

### Other

- Apply minor or patch updates to the dependencies
- Update development dependencies
- Make CI testing more reproducible by forcing the use of Cargo.lock

## Version 1.3.0 Beta 1

This reintroduces the async features temporarily removed from [Version 1.3.0 Alpha-Without-Tokio 1](#version-130-alpha-without-tokio-1)

Rusqlite was updated from 0.31.0 to 0.32.1.
Please see [the release notes for 0.32.0](https://github.com/rusqlite/rusqlite/releases/tag/v0.32.0) and
[for 0.32.1](https://github.com/rusqlite/rusqlite/releases/tag/v0.32.1).
Tokio Rusqlite was updated from 0.5.1 to 0.6.0.
Please see the [release notes](https://github.com/programatik29/tokio-rusqlite/releases/tag/v0.6.0).

### Minimum Rust Version

Rust 1.77

### Documentation

Various documentation improvements and clarification. In particular, call out that if a rusqlite error is encountered during a migration, the next migrations in the list are not applied.

### Other

- Apply minor or patch updates to the dependencies
- Update development dependencies
- Make CI testing more reproducible by forcing the use of Cargo.lock


## Version 1.3.0 Alpha-Without-Tokio 1

### Major Changes

This is an alpha version to start integrating rusqlite 0.32.1. Unfortunately, at this time, tokio-rusqlite is did not update to rusqlite 0.32.1. So we are temporarily removing the async features, while we figure out a way to bring them back. **To be clear, we intend to support the async features going forward, this is a temporary change in a specifically tagged version**.

Rusqlite was updated from 0.31.0 to 0.32.1. Please see [the release notes for 0.32.0](https://github.com/rusqlite/rusqlite/releases/tag/v0.32.0) and
[for 0.32.1](https://github.com/rusqlite/rusqlite/releases/tag/v0.32.1)

### Minimum Rust Version

Rust 1.77

### Documentation

Various documentation improvements and clarification. In particular, call out that if a rusqlite error is encountered during a migration, the next migrations in the list are not applied.

### Other

- Apply minor or patch updates to the dependencies
- Update development dependencies
- Make CI testing more reproducible by forcing the use of Cargo.lock

## Version 1.2.0

*Same code as version 1.2.0-beta.1*

### Documentation

- Improved the badges a little bit

## Version 1.2.0 Beta 1

Small release, mainly to update dependencies.

### Minimum Rust Version

Now using edition 2021, but the minimum rust version is still 1.70

### New Features

No new features.

### Other

- Update rusqlite to 0.31
- Update various development dependencies
- Improve CI build time
- Impove documentation
- Fix some broken examples

### See also

Rusqlite was updated from 0.30 to 0.31. Please see [its release notes](https://github.com/rusqlite/rusqlite/releases/tag/v0.31.0)


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


