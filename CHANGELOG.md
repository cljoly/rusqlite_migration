# Changelog

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


