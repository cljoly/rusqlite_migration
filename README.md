<!-- insert
---
title: "Rusqlite Migration"
date: 2021-08-21T15:32:05
description: "↕️ Simple database schema migration library for rusqlite, written with performance in mind."
aliases:
- /rusqlite-migration
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

[![docs.rs](https://img.shields.io/docsrs/rusqlite_migration?style=flat-square)](https://docs.rs/rusqlite_migration) [![Crates.io](https://img.shields.io/crates/v/rusqlite_migration?style=flat-square)](https://crates.io/crates/rusqlite_migration) ![](https://img.shields.io/github/languages/code-size/cljoly/rusqlite_migration?style=flat-square) [![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg?style=flat-square)](https://github.com/rust-secure-code/safety-dance/) 

<!-- insert
{{< rawhtml >}}
end_insert -->
</div>
<!-- insert
{{< /rawhtml >}}
end_insert -->

<!-- cargo-sync-readme start -->

Rusqlite Migration is a simple schema migration library for [rusqlite](https://lib.rs/crates/rusqlite) using [user_version](https://sqlite.org/pragma.html#pragma_user_version) instead of an SQL table to maintain the current schema version.

It aims for:
- **simplicity**: define a set of SQL statements. Just add more SQL statement to change the schema. No external CLI, no macro.
- **performance**: no need to add a table to be parsed, the `user_version` field is at a fixed offset in the sqlite file format.

It works especially well with other small libraries complementing rusqlite, like [serde_rusqlite](https://crates.io/crates/serde_rusqlite).

## Example

Here, we define SQL statements to run with [Migrations::new](crate::Migrations::new) and run these (if necessary) with [.to_latest()](crate::Migrations::to_latest).

```rust
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
- migrations with multiple SQL statements (using for instance `r#"…"` or `include_str!(…)`)
- use of lazy_static
- migrations to previous versions (downward migrations)

### Built-in tests

To test that the migrations are working, you can add this in your test module:

```rust
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
