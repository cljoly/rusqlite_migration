<div align="center">
  
# Rusqlite Migration

[![docs.rs](https://img.shields.io/docsrs/rusqlite_migration?style=flat-square)](https://docs.rs/rusqlite_migration) [![Crates.io](https://img.shields.io/crates/v/rusqlite_migration?style=flat-square)](https://crates.io/crates/rusqlite_migration) [![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg?style=flat-square)](https://github.com/rust-secure-code/safety-dance/)

</div>

<!-- cargo-sync-readme start -->

Rusqlite Migration is a simple schema migration tool for [rusqlite](https://lib.rs/crates/rusqlite) using [user_version](https://sqlite.org/pragma.html#pragma_user_version) instead of an SQL table to maintain the current schema version.

It aims for:
- **simplicity**: there is a set of SQL statements and you just append to it to change the schema,
- **performance**: no need to add a table to be parsed, the `user_version` field is at a fixed offset in the sqlite file format.

## Example

```rust
use anyhow::Result;
use env_logger;
use lazy_static::lazy_static;
use rusqlite::{params, Connection};
use rusqlite_migration::{Migrations, M};

// Define migrations. These are applied atomically.
lazy_static! {
    static ref MIGRATIONS: Migrations<'static> =
        Migrations::new(vec![
            M::up(r#"
                CREATE TABLE friend(
                    friend_id INTEGER PRIMARY KEY,
                    name TEXT NOT NULL,
                    email TEXT UNIQUE,
                    phone TEXT UNIQUE,
                    picture BLOB
                );
   
                CREATE TABLE car(
                    registration_plate TEXT PRIMARY KEY,
                    cost REAL NOT NULL,
                    bought_on TEXT NOT NULL
                );
            "#),
            // PRAGMA are better applied outside of migrations, see below for details.
            M::up(r#"
                      ALTER TABLE friend ADD COLUMN birthday TEXT;
                      ALTER TABLE friend ADD COLUMN comment TEXT;
                  "#),
            // In the future, if the need to change the schema arises, put
            // migrations here, like so:
            // M::up("CREATE INDEX UX_friend_email ON friend(email);"),
            // M::up("CREATE INDEX UX_friend_name ON friend(name);"),
        ]);
}

pub fn init_db() -> Result<Connection> {
    let mut conn = Connection::open("./my_db.db3")?;

    // Update the database schema, atomically
    MIGRATIONS.latest(&mut conn)?;

    Ok(conn)
}

pub fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("trace")).init();

    let conn = init_db().unwrap();

    // Apply some PRAGMA. These are often better applied outside of migrations, as some needs to be
    // executed for each connection (like `foreign_keys`) or to be executed outside transactions
    // (`journal_mode` is a noop in a transaction).
    conn.pragma_update(None, "journal_mode", &"WAL").unwrap();
    conn.pragma_update(None, "foreign_keys", &"ON").unwrap();

    // Use the db 🥳
    conn.execute(
        "INSERT INTO friend (name, birthday) VALUES (?1, ?2)",
        params!["John", "1970-01-01"],
    )
    .unwrap();
}
```

To test that the migrations are working, you can add this to your other tests:

```rust
    #[test]
    fn migrations_test() {
        assert!(MIGRATIONS.validate().is_ok());
    }
```


<!-- cargo-sync-readme end -->

## Acknowledgments

I would like to thank all the contributors, as well as the authors of the
dependancies this crate uses.
