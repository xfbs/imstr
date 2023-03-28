# Immutable Strings

Inspired by the [bytes](https://docs.rs/bytes) crate, which offers zero-copy
byte slices, this crate does the same but for strings. It is backed by standard
library string that is stored by an Arc, and every instance contains a range
into that String.  This allows for cloning and creating slices very cheaply.
This is especially useful for parsing operations, where a large string needs to
be sliced into a lot of substrings.

## Similar

| Crate | Zero-Copy | Slicing | Modify | String Compatible | Notes |
| --- | --- | --- | --- | --- | --- |
| [Tendril](https://crates.io/crates/tendril) | Yes | Yes | Yes | No | Complex implementation |
| [Immut String](https://crates.io/crates/immut_string) | Yes | No | No |  | Simple |
| [Immutable String](https://crates.io/crates/immutable_string) | No | No | No | | |
| [ArcCStr](https://crates.io/crates/arccstr) | Yes | No | No | | Not UTF-8 |
| [Implicit Clone](https://crates.io/crates/implicit-clone) | Yes | No | No | | |


https://crates.io/crates/semistr
https://crates.io/crates/quetta
https://crates.io/crates/bytesstr
https://crates.io/crates/fast-str
https://crates.io/crates/flexstr
https://crates.io/crates/sstable
https://crates.io/crates/bytestring
https://crates.io/crates/arcstr
https://crates.io/crates/cowstr
https://crates.io/crates/strck

