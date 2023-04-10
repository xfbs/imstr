# Changelog

## Version 0.2.0

- Adds `ptr_eq()` to `Data` trait
- Adds `trim()`, `trim_start()`, `trim_end()` methods to `ImString`
- Adds `nom` integration
- Adds `pop()`, `into_bytes()`, `as_mut_str()`, `from_utf16()` and `from_utf16_lossy()` methods for ImString.

## Version 0.1.1

- Added integration with [peg](https://crates.io/crates/peg) crate.
- Added benchmarks (using `criterion`).
- Implemented `AsMut<str>`, `BorrowMut<str>` and `DerefMut`.
- Improved crate documentation.

## Version 0.1.0

- Initial release of the crate.
