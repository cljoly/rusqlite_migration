<!-- insert
---
title: "Rusqlite Migration"
date: 2021-08-21T15:32:05
description: "↕️ Simple database schema migration library for rusqlite, written with performance in mind."
aliases:
- /rusqlite-migration
tags:
- Rust
- SQLite
- Library
---
end_insert -->

<!-- remove -->
<div align="center">

# Rusqlite Migration
<!-- end_remove -->

<!-- rustdoc start -->

<!-- insert
{{< github_badge >}}

{{< rawhtml >}}
<div class="badges">
{{< /rawhtml >}}
end_insert -->

[![docs.rs](https://img.shields.io/docsrs/rusqlite_migration)][docs]
[![Crates.io](https://img.shields.io/crates/v/rusqlite_migration)][cio]
[![Changelog](https://img.shields.io/badge/-Changelog-purple)][changelog]
[![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)][safety-dance]
[![dependency status](https://deps.rs/crate/rusqlite_migration/latest/status.svg)][deps]
[![Coveralls](https://img.shields.io/coverallsCoverage/github/cljoly/rusqlite_migration)][coveralls]

<!-- insert
{{< rawhtml >}}
end_insert -->
</div>
<!-- insert
{{< /rawhtml >}}
end_insert -->

Rusqlite Migration is a performant and simple schema migration library for [rusqlite](https://crates.io/crates/rusqlite).

* **Performance**:
    * *Fast database opening*: to keep track of the current migration state, most tools create one or more tables in the database. These tables require parsing by SQLite and are queried with SQL statements. This library uses the [`user_version`][uv] value instead. It’s much lighter as it is just an integer at a [fixed offset][uv_offset] in the SQLite file.
    * *Fast compilation*: this crate is very small and does not use macros to define the migrations.
* **Simplicity**: this crate strives for simplicity. Just define a set of SQL statements as strings in your Rust code. Add more SQL statements over time as needed. No external CLI required. Additionally, rusqlite_migration works especially well with other small libraries complementing rusqlite, like [serde_rusqlite][].

## Example

Here, we define SQL statements to run with [`Migrations::new()`][migrations_new] and run these (if necessary) with [`Migrations::to_latest()`][migrations_to_latest].

[migrations_new]: https://docs.rs/rusqlite_migration/latest/rusqlite_migration/struct.Migrations.html#method.new
[migrations_to_latest]: https://docs.rs/rusqlite_migration/latest/rusqlite_migration/struct.Migrations.html#method.to_latest

``` rust
use rusqlite::{params, Connection};
use rusqlite_migration::{Migrations, M};

// 1️⃣ Define migrations
const MIGRATIONS_SLICE: &[M<'_>] = &[
    M::up("CREATE TABLE friend(name TEXT NOT NULL);"),
    // In the future, add more migrations here:
    //M::up("ALTER TABLE friend ADD COLUMN email TEXT;"),
];
const MIGRATIONS: Migrations<'_> = Migrations::from_slice(MIGRATIONS_SLICE);

fn main() {
    let mut conn = Connection::open_in_memory().unwrap();

    // Apply some PRAGMA, often better to do it outside of migrations
    conn.pragma_update_and_check(None, "journal_mode", &"WAL", |_| Ok(()))
        .unwrap();

    // 2️⃣ Update the database schema, atomically
    MIGRATIONS.to_latest(&mut conn).unwrap();

    // 3️⃣ Use the database 🥳
    conn.execute("INSERT INTO friend (name) VALUES (?1)", params!["John"])
        .unwrap();
}
```

Please see the [examples](https://github.com/cljoly/rusqlite_migrate/tree/master/examples) folder for more, in particular:
- migrations with multiple SQL statements (using for instance `r#"…"` or `include_str!(…)`)
- migrations defined [from a directory][from_dir] with SQL files
- migrations to [previous versions (downward migrations)][generic_example]
- migrations [when using `async`][quick_start_async]

I’ve also made a [cheatsheet of SQLite pragma for improved performance and consistency][cheat].

### Built-in tests

To test that the migrations are working, you can add this in your test module:

``` rust
#[test]
fn migrations_test() {
    assert!(MIGRATIONS.validate().is_ok());
}
```

The migrations object is also suitable for serialisation with [insta][], using the `Debug` serialisation. You can store a snapshot of your migrations like this:

```rust
#[test]
fn migrations_insta_snapshot() {
    let migrations = Migrations::new(vec![
        // ...
    ]);
    insta::assert_debug_snapshot!(migrations);
}
```

[insta]: https://insta.rs/

## Optional Features

Rusqlite_migration provides several [Cargo features][cargo_features]. They are:

* `from-directory`: enable loading migrations from *.sql files in a given directory

[cargo_features]: https://doc.rust-lang.org/cargo/reference/manifest.html#the-features-section

## Active Users

<!-- insert
{{< rawhtml >}}
<div class="badges">
{{< /rawhtml >}}
end_insert -->

[![Crates.io Downloads](https://img.shields.io/crates/d/rusqlite_migration?style=social)][cio] [![Crates.io Downloads (recent)](https://img.shields.io/crates/dr/rusqlite_migration?style=social)][cio]

<!-- insert
{{< rawhtml >}}
</div>
{{< /rawhtml >}}
end_insert -->

This crate is actively used in a number of projects. You can find up-to-date list of those on:

* [crates.io][cio_reverse] / [lib.rs][lrs_reverse]
* [GitHub’s list of dependent repositories][gh_reverse]

A number of contributors are also reporting issues as they arise, another indicator of active use.

## Minimum Supported Rust Version (MSRV)

This crate extends rusqlite and as such is tightly integrated with it. Thus, it supports the [same MSRV][msrv] as rusqlite. At the time of writing, this means:

> Latest stable Rust version at the time of release. It might compile with older versions.

## Limits

1. Since this crate uses the [`user_version`][uv_offset] field, if your program or any other library changes it, this library will behave in an unspecified way: it may return an error, apply the wrong set of migrations, do nothing at all...

1. The [`user_version`][uv_offset] field is effectively a i32, so there is a theoretical limit (about two billion) on the number of migrations that can be applied by this library. You are likely to hit memory limits well before that though, so in practice, you can think of the number of migrations as limitless. And you would need to create 10 000 new migrations, every day, for over 5 centuries, before getting close to the limit.

## Contributing

Contributions (documentation or code improvements in particular) are welcome, see [contributing][]!

We use various tools for testing that you may find helpful to install locally (e.g. to fix failing CI checks):
* [cargo-insta][]
* [cargo-mutants][]

## Acknowledgments

I would like to thank all the contributors, as well as the authors of the dependencies this crate uses.

Thanks to [Migadu](https://www.migadu.com/) for offering a discounted service to support this project. It is not an endorsement by Migadu though.

[deps]: https://deps.rs/crate/rusqlite_migration
[changelog]: https://cj.rs/rusqlite_migration/changelog
[coveralls]: https://coveralls.io/github/cljoly/rusqlite_migration
[safety-dance]: https://github.com/rust-secure-code/safety-dance/
[cio]: https://crates.io/crates/rusqlite_migration
[cio_reverse]: https://crates.io/crates/rusqlite_migration/reverse_dependencies
[lrs_reverse]: https://lib.rs/crates/rusqlite_migration/rev
[gh_reverse]: https://github.com/cljoly/rusqlite_migration/network/dependents?dependent_type=REPOSITORY
[contributing]: https://cj.rs/docs/contribute/
[diesel_migrations]: https://crates.io/crates/diesel_migrations
[pgfine]: https://crates.io/crates/pgfine
[movine]: https://crates.io/crates/movine
[uv]: https://sqlite.org/pragma.html#pragma_user_version
[uv_offset]: https://www.sqlite.org/fileformat.html#user_version_number
[serde_rusqlite]: https://crates.io/crates/serde_rusqlite
[cargo-insta]: https://crates.io/crates/cargo-insta
[cargo-mutants]: https://mutants.rs/installation.html
[cheat]: https://cj.rs/blog/sqlite-pragma-cheatsheet-for-performance-and-consistency/
[docs]: https://docs.rs/rusqlite_migration
[msrv]: https://github.com/rusqlite/rusqlite?tab=readme-ov-file#minimum-supported-rust-version-msrv
[from_dir]: https://github.com/cljoly/rusqlite_migration/tree/master/examples/from-directory
[generic_example]: https://github.com/cljoly/rusqlite_migration/blob/master/examples/simple/src/main.rs
[quick_start_async]: https://github.com/cljoly/rusqlite_migration/blob/master/examples/async/src/main.rs
