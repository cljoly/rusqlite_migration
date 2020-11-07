use rusqlite::Connection;
use rusqlite_migration::{Migrations, M};

pub fn main() {
    let conn = Connection::open_in_memory().unwrap();
    let migrations = Migrations::new(&conn, &[M::up("").down(""), M::up("").down("")]);
}
