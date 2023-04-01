//! # Immutable Strings
//!
//! Inspired by the [bytes](https://docs.rs/bytes) crate, which offers zero-copy byte slices, this
//! crate does the same but for strings. It is backed by standard library string that is stored by
//! an Arc, and every instance contains a range into that String.  This allows for cloning and
//! creating slices very cheaply.  This is especially useful for parsing operations, where a large
//! string needs to be sliced into a lot of substrings.
//!
//! This crate is heavily inspired by the standard library's [String](std::string::String) type and
//! the `bytes` crate's [Bytes](https://docs.rs/bytes/latest/bytes/struct.Bytes.html) type.
mod string;
pub mod error;
pub mod data;
//mod os_string;

pub use string::{ToImString, ImString};
//pub use os_string::*;
