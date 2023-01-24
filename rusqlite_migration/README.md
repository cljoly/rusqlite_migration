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

<!-- insert
{{< github_badge >}}

{{< rawhtml >}}
<div class="badges">
{{< /rawhtml >}}
end_insert -->

[![docs.rs](https://img.shields.io/docsrs/rusqlite_migration?style=flat-square)](https://docs.rs/rusqlite_migration) [![Crates.io](https://img.shields.io/crates/v/rusqlite_migration?style=flat-square)](https://crates.io/crates/rusqlite_migration) ![](https://img.shields.io/github/languages/code-size/cljoly/rusqlite_migration?style=flat-square) [![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg?style=flat-square)](https://github.com/rust-secure-code/safety-dance/) [![dependency status](https://deps.rs/repo/github/cljoly/rusqlite_migration/status.svg)](https://deps.rs/repo/github/cljoly/rusqlite_migration)

<!-- insert
{{< rawhtml >}}
end_insert -->
</div>
<!-- insert
{{< /rawhtml >}}
end_insert -->

<!-- cargo-sync-readme start -->

Rusqlite Migration is a simple and performant schema migration library for [rusqlite](https://lib.rs/crates/rusqlite).

* **Performance**:
    * *Fast database opening*: to keep track of the current migration state, most tools create one or more tables in the database. These tables require parsing by SQLite and are queried with SQL statements. This library uses the [`user_version`][uv] value instead. It’s much lighter as it is just an integer at a [fixed offset][uv_offset] in the SQLite file.
    * *Fast compilation*: this crate is very small and does not use macros to define the migrations.
* **Simplicity**: this crate strives for simplicity. Just define a set of SQL statements as strings in your Rust code. Add more SQL statements over time as needed. No external CLI required. Additionally, rusqlite_migration works especially well with other small libraries complementing rusqlite, like [serde_rusqlite][].

[diesel_migrations]: https://lib.rs/crates/diesel_migrations
[pgfine]: https://crates.io/crates/pgfine
[movine]: https://crates.io/crates/movine
[uv]: https://sqlite.org/pragma.html#pragma_user_version
[uv_offset]: https://www.sqlite.org/fileformat.html#user_version_number
[serde_rusqlite]: https://crates.io/crates/serde_rusqlite

## Example

Here, we define SQL statements to run with [`Migrations::new()`][migrations_new] and run these (if necessary) with [`Migrations::to_latest()`][migrations_to_latest].

[migrations_new]: https://docs.rs/rusqlite_migration/latest/rusqlite_migration/struct.Migrations.html#method.new
[migrations_to_latest]: https://docs.rs/rusqlite_migration/latest/rusqlite_migration/struct.Migrations.html#method.to_latest

``` rust
use rusqlite::{params, Connection};
use rusqlite_migration::{Migrations, M};

// 1️⃣ Define migrations
let migrations = Migrations::new(vec![
    M::up("CREATE TABLE friend(name TEXT NOT NULL);"),
    // In the future, add more migrations here:
    //M::up("ALTER TABLE friend ADD COLUMN email TEXT;"),
]);

let mut conn = Connection::open_in_memory().unwrap();

// Apply some PRAGMA, often better to do it outside of migrations
conn.pragma_update(None, "journal_mode", &"WAL").unwrap();

// 2️⃣ Update the database schema, atomically
migrations.to_latest(&mut conn).unwrap();

// 3️⃣ Use the database 🥳
conn.execute("INSERT INTO friend (name) VALUES (?1)", params!["John"])
    .unwrap();
```

Please see the [examples](https://github.com/cljoly/rusqlite_migrate/tree/master/examples) folder for more, in particular:
- `async` migrations in the [`quick_start_async.rs`][] file
- migrations with multiple SQL statements (using for instance `r#"…"` or `include_str!(…)`)
- use of lazy_static
- migrations to previous versions (downward migrations)

[quick_start_async.rs]: https://github.com/cljoly/rusqlite_migrate/tree/master/examples/quick_start_async.rs

I’ve also made a [cheatsheet of SQLite pragma for improved performance and consistency](https://cj.rs/blog/sqlite-pragma-cheatsheet-for-performance-and-consistency/).

### Built-in tests

To test that the migrations are working, you can add this in your test module:

``` rust
#[test]
fn migrations_test() {
    assert!(MIGRATIONS.validate().is_ok());
}
```

## Contributing

Contributions (documentation or code improvements in particular) are welcome, see [contributing](https://cj.rs/docs/contribute/)!

## Acknowledgments

I would like to thank all the contributors, as well as the authors of the dependencies this crate uses.


<!-- cargo-sync-readme end -->
