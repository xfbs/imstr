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
#![warn(missing_docs)]

pub mod data;
pub mod error;
pub mod string;

/// Thread-safe immutable string.
///
/// This is a convenient type alias for `ImString<Threadsafe>`. [`ImString`](string::ImString)
/// supports different backing data containers which have unique properties. The
/// [`Threadsafe`](string::Threadsafe) container offers a thread-safe shared storage backed by an
/// [`Arc`](std::sync::Arc).
///
/// If you do not need to use the [`ImString`] across multiple threads, then you can also use
/// [`Local`](string::Local) as the backing store. This does the same but is not threadsafe. It is
/// marginally faster.
///
/// Any type which implements the [Data](data::Data) trait can be used as backing stores.
pub type ImString = string::ImString<string::Threadsafe>;

#[cfg(feature = "peg")]
pub mod peg;
