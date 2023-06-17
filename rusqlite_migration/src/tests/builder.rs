use std::iter::FromIterator;

use crate::{MigrationsBuilder, M};

#[test]
#[should_panic]
fn test_non_existing_index() {
    let ms = vec![M::up("CREATE TABLE t(a);")];

    let _ = MigrationsBuilder::from_iter(ms.clone()).edit(100, move |t| t);
}

#[test]
#[should_panic]
fn test_0_index() {
    let ms = vec![M::up("CREATE TABLE t(a);")];

    let _ = MigrationsBuilder::from_iter(ms).edit(0, move |t| t);
}
