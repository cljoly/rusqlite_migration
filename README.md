<div align="center">
  
# Rusqlite Migration

[![docs.rs](https://img.shields.io/docsrs/rusqlite_migration?style=flat-square)](https://docs.rs/rusqlite_migration) [![Crates.io](https://img.shields.io/crates/v/rusqlite_migration?style=flat-square)](https://crates.io/crates/rusqlite_migration) [![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg?style=flat-square)](https://github.com/rust-secure-code/safety-dance/)

</div>

<!-- cargo-sync-readme start -->

Rusqlite Migration is a simple schema migration tool for [rusqlite](https://lib.rs/crates/rusqlite) using [user_version](https://sqlite.org/pragma.html#pragma_user_version) instead of an SQL table to maintain the current schema version.

It aims for:
- **simplicity**: define a set of SQL statements. Just add more SQL statement to change the schema. No external CLI, no macro.
- **performance**: no need to add a table to be parsed, the `user_version` field is at a fixed offset in the sqlite file format.

## Example

Here, we define SQL statements to run with [Migrations::new](crate::Migrations::new) and run these (if necessary) with [.to_latest()](crate::Migrations::to_latest).

```rust
use lazy_static::lazy_static;
use rusqlite::{params, Connection};
use rusqlite_migration::{Migrations, M};

// 1️⃣ Define migrations
lazy_static! {
    static ref MIGRATIONS: Migrations<'static> =
        Migrations::new(vec![
            M::up(r#"
                CREATE TABLE friend(
                    name TEXT NOT NULL,
                    email TEXT UNIQUE
                );
            "#),
            // In the future, add more migrations here:
            //M::up("ALTER TABLE friend ADD COLUMN birthday TEXT;"),
        ]);
}

fn main() {
    let mut conn = Connection::open_in_memory().unwrap();
    // Apply some PRAGMA, often better to do it outside of migrations
    conn.pragma_update(None, "journal_mode", &"WAL").unwrap();

    // 2️⃣ Update the database schema, atomically
    MIGRATIONS.to_latest(&mut conn).unwrap();

    // Use the database 🥳
    conn.execute(
        "INSERT INTO friend (name, email) \
         VALUES (?1, ?2)",
        params!["John", "john@example.org"],
    )
    .unwrap();
}
```

### Built-in tests

To test that the migrations are working, you can add this in your test module:

```rust
#[test]
fn migrations_test() {
    assert!(MIGRATIONS.validate().is_ok());
}
```

### Migrations to previous versions, more detailed examples…

Please see the [examples](https://github.com/cljoly/rusqlite_migrate/tree/master/examples) folder for more.

<!-- cargo-sync-readme end -->

## Acknowledgments

I would like to thank all the contributors, as well as the authors of the
dependancies this crate uses.
