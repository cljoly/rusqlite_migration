use rusqlite::Connection;
use rusqlite_migration::{Migrations, M};

pub fn main() {
    let mut conn = Connection::open_in_memory().unwrap();
    let migrations = Migrations::new(vec![M::up("")]);
    migrations.latest(&mut conn).unwrap();
}
