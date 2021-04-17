# Changelog

## Version 0.4.1

- Update documentation

## Version 0.4.0

### New features

- Add downward migrations, i.e. migrations to go to past schema version of the database. Thanks @MightyPork!
- Unsafe code is now forbidden.

### Breaking changes

- Rename `latest` to `MightyPorkto_latest`. The old symbol is depracated and will be removed eventually.
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


