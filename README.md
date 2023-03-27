# Immutable Strings

Inspired by the [bytes](https://docs.rs/bytes) crate, which offers zero-copy
byte slices, this crate does the same but for strings. It is backed by standard
library string that is stored by an Arc, and every instance contains a range
into that String.  This allows for cloning and creating slices very cheaply.
This is especially useful for parsing operations, where a large string needs to
be sliced into a lot of substrings.

