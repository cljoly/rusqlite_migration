use crate::M;

pub fn m_valid0() -> M<'static> {
    M::up("CREATE TABLE m1(a, b); CREATE TABLE m2(a, b, c);")
}
pub fn m_valid10() -> M<'static> {
    M::up("CREATE TABLE t1(a, b);")
}
pub fn m_valid11() -> M<'static> {
    M::up("ALTER TABLE t1 RENAME COLUMN b TO c;")
}
pub fn m_valid20() -> M<'static> {
    M::up("CREATE TABLE t2(b);")
}
pub fn m_valid21() -> M<'static> {
    M::up("ALTER TABLE t2 ADD COLUMN a;")
}

pub fn m_valid_fk() -> M<'static> {
    M::up(
        r#"
        CREATE TABLE fk1(a PRIMARY KEY);
        CREATE TABLE fk2(
            a,
            FOREIGN KEY(a) REFERENCES fk1(a)
        );
        INSERT INTO fk1 (a) VALUES ('foo');
        INSERT INTO fk2 (a) VALUES ('foo');
    "#,
    )
    .foreign_key_check()
}

pub fn m_invalid_down_fk() -> M<'static> {
    M::up(
        r#"
        CREATE TABLE fk1(a PRIMARY KEY);
        CREATE TABLE fk2(
            a,
            FOREIGN KEY(a) REFERENCES fk1(a)
        );
        INSERT INTO fk1 (a) VALUES ('foo');
        INSERT INTO fk2 (a) VALUES ('foo');
    "#,
    )
    .foreign_key_check()
    .down("DROP TABLE fk1;")
}

// All valid Ms in the right order
pub fn all_valid() -> Vec<M<'static>> {
    vec![
        m_valid0(),
        m_valid10(),
        m_valid11(),
        m_valid20(),
        m_valid21(),
        m_valid_fk(),
    ]
}

pub fn m_invalid0() -> M<'static> {
    M::up("CREATE TABLE table3()")
}
pub fn m_invalid1() -> M<'static> {
    M::up("something invalid")
}

pub fn m_invalid_fk() -> M<'static> {
    M::up(
        r#"
        CREATE TABLE fk1(a PRIMARY KEY);
        CREATE TABLE fk2(
            a,
            FOREIGN KEY(a) REFERENCES fk1(a)
        );
        INSERT INTO fk2 (a) VALUES ('foo');
        INSERT INTO fk2 (a) VALUES ('bar');
    "#,
    )
    .foreign_key_check()
}

pub fn all_errors() -> Vec<(&'static str, crate::Error)> {
    use crate::Error::*;
    use crate::ForeignKeyCheckError;
    use crate::MigrationDefinitionError;
    use crate::SchemaVersion;
    use crate::SchemaVersionError;

    vec![
        (
            "rusqlite_error",
            RusqliteError {
                query: "SELECT * FROM table42;".to_owned(),
                err: rusqlite::Error::InvalidQuery,
            },
        ),
        (
            "specified_schema_version",
            SpecifiedSchemaVersion(SchemaVersionError::TargetVersionOutOfRange {
                specified: SchemaVersion::NoneSet,
                highest: SchemaVersion::NoneSet,
            }),
        ),
        (
            "migration_definition",
            MigrationDefinition(MigrationDefinitionError::NoMigrationsDefined),
        ),
        (
            "foreign_key_check",
            ForeignKeyCheck(vec![
                ForeignKeyCheckError {
                    table: "t1".to_owned(),
                    rowid: 1,
                    parent: "t2".to_owned(),
                    fkid: 2,
                },
                ForeignKeyCheckError {
                    table: "t3".to_owned(),
                    rowid: 2,
                    parent: "t4".to_owned(),
                    fkid: 3,
                },
            ]),
        ),
        ("hook", Hook("in hook".to_owned())),
        ("file_load", FileLoad("file causing problem".to_owned())),
        (
            "unrecognized",
            Unrecognized(Box::new(Hook("unknown".to_owned()))),
        ),
        #[cfg(feature = "alpha-async-tokio-rusqlite")]
        ("connection_closed", ConnectionClosed),
    ]
}
