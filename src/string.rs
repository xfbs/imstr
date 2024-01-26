//! [`ImString`] type and associated data store types.
use crate::data::Data;
use crate::error::*;
use alloc::{
    borrow::Cow,
    rc::Rc,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use core::{
    borrow::{Borrow, BorrowMut},
    cmp::Ordering,
    convert::{AsMut, AsRef, Infallible},
    fmt::{Debug, Display, Error as FmtError, Formatter, Write},
    hash::{Hash, Hasher},
    iter::{Extend, FromIterator},
    ops::{
        Add, AddAssign, Bound, Deref, DerefMut, Index, IndexMut, Range, RangeBounds, RangeFrom,
        RangeFull, RangeInclusive, RangeTo,
    },
    str::FromStr,
};
#[cfg(feature = "std")]
use std::{ffi::OsStr, net::ToSocketAddrs, path::Path};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Threadsafe shared storage for string.
pub type Threadsafe = Arc<String>;

/// Shared storage for string (not threadsafe).
pub type Local = Rc<String>;

/// Cheaply cloneable and sliceable UTF-8 string type.
///
/// An `ImString` is a cheaply cloneable and sliceable UTF-8 string type,
/// designed to provide efficient operations for working with text data.
///
/// `ImString` is backed by a reference-counted shared
/// [`String`](std::string::String), which allows it to provide efficient
/// cloning and slicing operations. When an `ImString` is cloned or sliced,
/// it creates a new view into the underlying `String`, without copying the
/// text data. This makes working with large strings and substrings more
/// memory-efficient.
///
/// Due to its design, `ImString` is especially suitable for use cases where
/// strings are frequently cloned or sliced, but modifications to the text data
/// are less common.
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// use imstr::ImString;
///
/// // Create new ImString from a string literal
/// let string = ImString::from("hello world");
///
/// // Clone the ImString without copying the text data.
/// let string_clone = string.clone();
///
/// // Create a slice (substring) without copying the text data.
/// let string_slice = string.slice(0..5);
/// assert_eq!(string_slice, "hello");
/// ```
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ImString<S: Data<String>> {
    /// Underlying string
    string: S,
    /// Offset, must always point to valid UTF-8 region inside string.
    offset: Range<usize>,
}

fn slice_ptr_range(slice: &[u8]) -> Range<*const u8> {
    let start = slice.as_ptr();
    let end = unsafe { start.add(slice.len()) };
    start..end
}

fn try_slice_offset(current: &[u8], candidate: &[u8]) -> Option<Range<usize>> {
    let current_slice = slice_ptr_range(current);
    let candidate_slice = slice_ptr_range(candidate);
    let contains_start = current_slice.start <= candidate_slice.start;
    let contains_end = current_slice.end >= candidate_slice.end;
    if !contains_start || !contains_end {
        return None;
    }
    let offset_start = unsafe { candidate_slice.start.offset_from(current_slice.start) } as usize;
    let offset_end = unsafe { candidate_slice.end.offset_from(current_slice.start) } as usize;
    Some(offset_start..offset_end)
}

impl<S: Data<String>> ImString<S> {
    /// Returns a byte slice of this string's contents.
    ///
    /// The inverse of this method is [`from_utf8`](ImString::from_utf8) or
    /// [`from_utf8_lossy`](ImString::from_utf8_lossy).
    ///
    /// # Example
    ///
    /// ```rust
    /// # use imstr::ImString;
    /// let string = ImString::from("hello");
    /// assert_eq!(string.as_bytes(), &[104, 101, 108, 108, 111]);
    /// ```
    pub fn as_bytes(&self) -> &[u8] {
        &self.string.get().as_bytes()[self.offset.clone()]
    }

    /// Return the backing [String](std::string::String)'s capacity, in bytes.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use imstr::ImString;
    /// let string = ImString::with_capacity(10);
    /// assert_eq!(string.capacity(), 10);
    /// ```
    pub fn capacity(&self) -> usize {
        self.string.get().capacity()
    }

    /// Create a new `ImString` instance from a standard library [`String`](std::string::String).
    ///
    /// This method will construct the [`ImString`] without needing to clone the [`String`] instance.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use imstr::ImString;
    /// let string = String::from("hello");
    /// let string = ImString::from_std_string(string);
    /// ```
    pub fn from_std_string(string: String) -> Self {
        ImString {
            offset: 0..string.as_bytes().len(),
            string: S::new(string),
        }
    }

    /// Truncates this string, removing all contents.
    ///
    /// If this is the only reference to the string, it will clear the backing
    /// [String](std::string::String). If it is not, it only sets the offset to an empty slice.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use imstr::ImString;
    /// let mut string = ImString::from("hello");
    /// assert_eq!(string, "hello");
    /// string.clear();
    /// assert_eq!(string, "");
    /// ```
    pub fn clear(&mut self) {
        unsafe {
            self.try_modify_unchecked(|string| string.clear());
        }
        self.offset = 0..0;
    }

    /// Returns a mutable string slice of the contents of this string.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```rust
    /// # use imstr::ImString;
    /// let mut string = ImString::from("foobar");
    /// let string_slice = string.as_mut_str();
    /// string_slice.make_ascii_uppercase();
    /// assert_eq!(string, "FOOBAR");
    /// ```
    pub fn as_mut_str(&mut self) -> &mut str {
        if self.string.get_mut().is_none() {
            let string = self.as_str().to_string();
            self.offset = 0..string.len();
            self.string = S::new(string);
        }

        let string = self.string.get_mut().unwrap();
        return &mut string[self.offset.clone()];
    }

    unsafe fn try_modify_unchecked<F: FnOnce(&mut String)>(&mut self, f: F) -> bool {
        if let Some(string) = self.string.get_mut() {
            f(string);
            true
        } else {
            false
        }
    }

    /// Creates a new [`ImString`] with the given capacity.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use imstr::ImString;
    /// let mut string = ImString::with_capacity(10);
    /// assert_eq!(string.capacity(), 10);
    /// ```
    pub fn with_capacity(capacity: usize) -> Self {
        ImString::from_std_string(String::with_capacity(capacity))
    }

    /// Returns the length of the string in bytes.
    ///
    /// This will not return the length in `char`s or graphemes.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use imstr::ImString;
    /// let string = ImString::from("hello");
    /// assert_eq!(string.len(), "hello".len());
    /// ```
    pub fn len(&self) -> usize {
        self.offset.len()
    }

    /// Convert this string into a standard library [`String`](std::string::String).
    ///
    /// If this string has no other clones, it will return the `String` without needing to clone
    /// it.
    ///
    /// ```rust
    /// # use imstr::ImString;
    /// let string = ImString::from("hello");
    /// let string = string.into_std_string();
    /// assert_eq!(string, "hello");
    /// ```
    pub fn into_std_string(mut self) -> String {
        if self.offset.start == 0 {
            if let Some(string) = self.string.get_mut() {
                string.truncate(self.offset.end);
                return core::mem::take(string);
            }
        }

        self.as_str().to_string()
    }

    /// Creates a new, empty `ImString`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use imstr::ImString;
    /// let string = ImString::new();
    /// assert_eq!(string, "");
    /// ```
    pub fn new() -> Self {
        ImString::from_std_string(String::new())
    }

    /// Extracts a string slice containing the entire string.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use imstr::ImString;
    /// let string = ImString::from("hello");
    /// assert_eq!(string.as_str(), "hello");
    /// ```
    pub fn as_str(&self) -> &str {
        let slice = &self.string.get().as_bytes()[self.offset.start..self.offset.end];
        unsafe { core::str::from_utf8_unchecked(slice) }
    }

    /// Decode a UTF-16-encoded string into an [`ImString`], returning a [`FromUtf16Error`] if
    /// `string` contains any invalid data.
    ///
    /// This method is useful for interfacing with legacy systems that still use UTF-16 as their
    /// primary encoding.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use imstr::ImString;
    /// // 𝄞music
    /// let v = &[0xD834, 0xDD1E, 0x006d, 0x0075, 0x0073, 0x0069, 0x0063];
    /// assert_eq!(ImString::from("𝄞music"), ImString::from_utf16(v).unwrap());
    ///
    /// // 𝄞mu<invalid>ic
    /// let v = &[0xD834, 0xDD1E, 0x006d, 0x0075, 0xD800, 0x0069, 0x0063];
    /// assert!(ImString::from_utf16(v).is_err());
    /// ```
    pub fn from_utf16(string: &[u16]) -> Result<Self, FromUtf16Error> {
        Ok(ImString::from_std_string(String::from_utf16(string)?))
    }

    /// Decode a UTF-16-encoded string into an [`ImString`], replacing invalid data with the
    /// [replacement character (`U+FFD`)](std::char::REPLACEMENT_CHARACTER).
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```rust
    /// # use imstr::ImString;
    /// // 𝄞mus<invalid>ic<invalid>
    /// let v = &[0xD834, 0xDD1E, 0x006d, 0x0075, 0x0073, 0xDD1E, 0x0069, 0x0063, 0xD834];
    /// assert_eq!(ImString::from("𝄞mus\u{FFFD}ic\u{FFFD}"), ImString::from_utf16_lossy(v));
    /// ```
    pub fn from_utf16_lossy(string: &[u16]) -> Self {
        ImString::from_std_string(String::from_utf16_lossy(string))
    }

    /// Converts a vector of bytes to an [`ImString`].
    ///
    /// See [`String::from_utf8()`] for more details on this function.
    ///
    /// # Example
    ///
    /// Basic usage:
    ///
    /// ```rust
    /// # use imstr::ImString;
    /// // some bytes, in a vector
    /// let sparkle_heart = vec![240, 159, 146, 150];
    ///
    /// // we know this is valid UTF-8, so we use unwrap()
    /// let string = ImString::from_utf8(sparkle_heart).unwrap();
    ///
    /// assert_eq!(string, "💖");
    /// ```
    pub fn from_utf8(vec: Vec<u8>) -> Result<Self, FromUtf8Error> {
        Ok(ImString::from_std_string(String::from_utf8(vec)?))
    }

    /// Converts a slice of bytes to a string, including invalid characters.
    ///
    /// See [`String::from_utf8_lossy()`] for more details on this function.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// # use imstr::ImString;
    /// // some bytes, in a vector
    /// let sparkle_heart = vec![240, 159, 146, 150];
    ///
    /// let sparkle_heart = ImString::from_utf8_lossy(&sparkle_heart);
    ///
    /// assert_eq!(sparkle_heart, "💖");
    /// ```
    ///
    /// Incorrect bytes:
    ///
    /// ```
    /// # use imstr::ImString;
    /// // some invalid bytes
    /// let input = b"Hello \xF0\x90\x80World";
    /// let output = ImString::from_utf8_lossy(input);
    ///
    /// assert_eq!(output, "Hello �World");
    /// ```
    pub fn from_utf8_lossy(bytes: &[u8]) -> Self {
        let string = String::from_utf8_lossy(bytes).into_owned();
        ImString::from_std_string(string)
    }

    /// Converts a vector of bytes to a [`ImString`], without checking if the data is valid UTF-8.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it does not check that the bytes passed to it are valid
    /// UTF-8. If this constraint is violated, it may cause memory unsafety issues with future
    /// users of the [`ImString`], as the library assumes that all strings are valid UTF-8.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```rust
    /// # use imstr::ImString;
    /// // some bytes, in a vector
    /// let sparkle_heart = vec![240, 159, 146, 150];
    ///
    /// let sparkle_heart = unsafe {
    ///     ImString::from_utf8_unchecked(sparkle_heart)
    /// };
    ///
    /// assert_eq!(sparkle_heart, "💖");
    /// ```
    pub unsafe fn from_utf8_unchecked(vec: Vec<u8>) -> Self {
        ImString::from_std_string(String::from_utf8_unchecked(vec))
    }

    /// Converts an [`ImString`] into a byte vector.
    ///
    /// This consumes the [`ImString`], so that in some circumstances the contents do not need to
    /// be copied.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```rust
    /// # use imstr::ImString;
    /// let string = ImString::from("hello");
    /// let bytes = string.into_bytes();
    /// assert_eq!(bytes, &[104, 101, 108, 108, 111]);
    /// ```
    pub fn into_bytes(self) -> Vec<u8> {
        self.into_std_string().into_bytes()
    }

    unsafe fn unchecked_append<F: FnOnce(String) -> String>(&mut self, f: F) {
        match self.string.get_mut() {
            Some(mut string_ref) if self.offset.start == 0 => {
                let mut string: String = core::mem::take(&mut string_ref);
                string.truncate(self.offset.end);
                *string_ref = f(string);
            }
            _ => {
                self.string = S::new(f(self.as_str().to_string()));
                self.offset.start = 0;
            }
        }

        self.offset.end = self.string.get().as_bytes().len();
    }

    /// Inserts a character into this string at the specified index.
    ///
    /// This is an *O(n)* operation as it requires copying every element in the buffer.
    ///
    /// # Panics
    ///
    /// Panics if `index` is larger than the [`ImString`]'s length, of if it does not lie on a
    /// [`char`] boundary. You can use [`is_char_boundary()`](str::is_char_boundary) to check if a
    /// given index is such a boundary.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```rust
    /// # use imstr::ImString;
    /// let mut string = ImString::with_capacity(3);
    /// string.insert(0, 'f');
    /// string.insert(1, 'o');
    /// string.insert(2, 'o');
    /// assert_eq!(string, "foo");
    /// ```
    pub fn insert(&mut self, index: usize, c: char) {
        unsafe {
            self.unchecked_append(|mut string| {
                string.insert(index, c);
                string
            });
        }
    }

    /// Inserts a string into this string at the specified index.
    ///
    /// This is an *O(n)* operation as it requires copying every element in the buffer.
    ///
    /// # Panics
    ///
    /// Panics if `index` is larger than the [`ImString`]'s length, of if it does not lie on a
    /// [`char`] boundary. You can use [`is_char_boundary()`](str::is_char_boundary) to check if an
    /// index lies on a char boundary.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use imstr::ImString;
    /// let mut string = ImString::from("Hello!");
    /// string.insert_str(5, ", World");
    /// assert_eq!(string, "Hello, World!");
    /// ```
    pub fn insert_str(&mut self, index: usize, s: &str) {
        unsafe {
            self.unchecked_append(|mut string| {
                string.insert_str(index, s);
                string
            });
        }
    }

    /// Shortens this [`ImString`] to the specified length.
    ///
    /// If `length` is greater than the string's current length, this has no effect.
    ///
    /// # Panics
    ///
    /// Panics if `length` does not lie on a char boundary. You can use the
    /// [`is_char_boundary()`](str::is_char_boundary) method to determine if an offset lies on
    /// a char boundary.
    ///
    /// # Example
    ///
    /// Basic usage:
    ///
    /// ```rust
    /// # use imstr::ImString;
    /// let mut string = ImString::from("hello");
    /// string.truncate(2);
    /// assert_eq!(string, "he");
    /// ```
    pub fn truncate(&mut self, length: usize) {
        // actual new length
        let length = self.offset.start + length;

        // truncate backing string if possible
        if let Some(string) = self.string.get_mut() {
            string.truncate(length);
        }

        self.offset.end = self.offset.end.min(length);
    }

    /// Removes the last character from the string and returns it.
    ///
    /// If the string is empty, this returns `None`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use imstr::ImString;
    /// let mut string = ImString::from("foo");
    ///
    /// assert_eq!(string.pop(), Some('o'));
    /// assert_eq!(string.pop(), Some('o'));
    /// assert_eq!(string.pop(), Some('f'));
    /// assert_eq!(string.pop(), None);
    /// ```
    pub fn pop(&mut self) -> Option<char> {
        let last_char = self.as_str().chars().rev().next()?;
        self.offset.end -= last_char.len_utf8();
        Some(last_char)
    }

    /// Appends the given [`char`] to the end of this [`ImString`].
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```rust
    /// # use imstr::ImString;
    /// let mut string = ImString::from("abc");
    ///
    /// string.push('1');
    /// string.push('2');
    /// string.push('3');
    ///
    /// assert_eq!(string, "abc123");
    /// ```
    pub fn push(&mut self, c: char) {
        unsafe {
            self.unchecked_append(|mut string| {
                string.push(c);
                string
            });
        }
    }

    /// Appends the given string slice onto to the end of this [`ImString`].
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```rust
    /// # use imstr::ImString;
    /// let mut string = ImString::from("foo");
    ///
    /// string.push_str("bar");
    ///
    /// assert_eq!(string, "foobar");
    /// ```
    pub fn push_str(&mut self, slice: &str) {
        unsafe {
            self.unchecked_append(|mut string| {
                string.push_str(slice);
                string
            });
        }
    }

    /// Returns `true` if this string has a length of zero, and `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use imstr::ImString;
    /// let string = ImString::from("");
    /// assert_eq!(string.is_empty(), true);
    ///
    /// let string = ImString::from("hello");
    /// assert_eq!(string.is_empty(), false);
    /// ```
    pub fn is_empty(&self) -> bool {
        self.offset.is_empty()
    }

    /// Create a new [`ImString`] containing a slice of this string.
    ///
    /// This will not copy the underlying string, only create another reference to it.
    ///
    /// # Panics
    ///
    /// This will panic if the specified range is invalid. In order to be valid, the lower and
    /// upper bounds must be within this string, and must lie on a [`char`] boundary.  Use the
    /// [try_slice](ImString::try_slice) method if you want to handle invalid ranges.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use imstr::ImString;
    /// let string = ImString::from("Hello, World!");
    ///
    /// let hello = string.slice(0..5);
    /// assert_eq!(hello, "Hello");
    ///
    /// let world = string.slice(7..12);
    /// assert_eq!(world, "World");
    /// ```
    pub fn slice(&self, range: impl RangeBounds<usize>) -> Self {
        self.try_slice(range).unwrap()
    }

    /// Try to create a new [`ImString`] containing a slice of this string.
    ///
    /// This will not copy the underlying string, only create another reference to it.
    ///
    /// If the specified range is not invalid, for example because it points outside of this string
    /// or because the lower or upper bound do not lie on a [`char`] boundary, this method will
    /// return [`SliceError`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use imstr::ImString;
    /// let string = ImString::from("Hello, World!");
    ///
    /// let hello = string.try_slice(0..5).unwrap();
    /// assert_eq!(hello, "Hello");
    ///
    /// let world = string.try_slice(7..12).unwrap();
    /// assert_eq!(world, "World");
    /// ```
    pub fn try_slice(&self, range: impl RangeBounds<usize>) -> Result<Self, SliceError> {
        let start = match range.start_bound() {
            Bound::Included(value) => *value,
            Bound::Excluded(value) => *value + 1,
            Bound::Unbounded => 0,
        };
        if start > self.offset.len() {
            return Err(SliceError::StartOutOfBounds);
        }
        let end = match range.end_bound() {
            Bound::Included(value) => *value - 1,
            Bound::Excluded(value) => *value,
            Bound::Unbounded => self.offset.len(),
        };
        if end < start {
            return Err(SliceError::EndBeforeStart);
        }
        if end > self.offset.len() {
            return Err(SliceError::EndOutOfBounds);
        }
        if !self.as_str().is_char_boundary(start) {
            return Err(SliceError::StartNotAligned);
        }
        if !self.as_str().is_char_boundary(end) {
            return Err(SliceError::EndNotAligned);
        }
        let slice = unsafe { self.slice_unchecked(range) };
        Ok(slice)
    }

    /// Create a new [`ImString`] containing a slice of this string without checking the bounds.
    ///
    /// # Safety
    ///
    /// This method is unsafe because it does not check the bounds.  If the specified range is not
    /// invalid, for example because it points outside of this string or because the lower or upper
    /// bound do not lie on a [`char`] boundary, this method will return an invalid [`ImString`],
    /// which can lead to memory unsafety errors.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use imstr::ImString;
    /// let string = ImString::from("Hello, World!");
    ///
    /// let hello = string.try_slice(0..5).unwrap();
    /// assert_eq!(hello, "Hello");
    ///
    /// let world = string.try_slice(7..12).unwrap();
    /// assert_eq!(world, "World");
    /// ```
    pub unsafe fn slice_unchecked(&self, range: impl RangeBounds<usize>) -> Self {
        let start = match range.start_bound() {
            Bound::Included(value) => *value,
            Bound::Excluded(value) => *value + 1,
            Bound::Unbounded => 0,
        };
        let end = match range.end_bound() {
            Bound::Included(value) => *value - 1,
            Bound::Excluded(value) => *value,
            Bound::Unbounded => self.offset.len(),
        };
        let offset = self.offset.start + start..self.offset.start + end;
        ImString {
            string: self.string.clone(),
            offset,
        }
    }

    /// Try to promote a [`str`] slice of this [`ImString`] into an [`ImString`].
    ///
    /// If the given [`str`] slice is not from this [`ImString`], this method will return `None`.
    ///
    /// This method is useful when interfacing with algorithms that only work on string slices,
    /// but you want to store the output strings as [`ImString`] values.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```rust
    /// # use imstr::ImString;
    /// let string = ImString::from("Hello, world!");
    /// let slice = &string[7..12];
    /// let slice = string.try_str_ref(slice).unwrap();
    /// assert_eq!(slice, "world");
    /// assert_eq!(string.try_str_ref("other"), None);
    /// ```
    pub fn try_str_ref(&self, string: &str) -> Option<Self> {
        self.try_slice_ref(string.as_bytes())
    }

    /// Promote a [`str`] slice of this [`ImString`] into an [`ImString`].
    ///
    /// If the given [`str`] slice is not from this [`ImString`], this method will create a new
    /// [`ImString`]. If you do not want this behavior, use
    /// [`try_str_ref()`](ImString::try_str_ref).
    ///
    /// This method is useful when interfacing with algorithms that only work on string slices,
    /// but you want to store the output strings as [`ImString`] values.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```rust
    /// # use imstr::ImString;
    /// let string = ImString::from("Hello, world!");
    /// let slice = &string[7..12];
    /// let slice = string.str_ref(slice);
    /// assert_eq!(slice, "world");
    /// assert_eq!(string.str_ref("other"), "other");
    /// ```
    pub fn str_ref(&self, string: &str) -> Self {
        self.try_str_ref(string)
            .unwrap_or_else(|| Self::from(string))
    }

    /// Try to promote a [`u8`] slice of this [`ImString`] into an [`ImString`].
    ///
    /// If the given [`u8`] slice is not from this [`ImString`], this method will return `None`.
    ///
    /// This method is useful when interfacing with algorithms that only work on byte slices,
    /// but you want to store the output strings as [`ImString`] values.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```rust
    /// # use imstr::ImString;
    /// let string = ImString::from("Hello, world!");
    /// let slice = &string.as_bytes()[7..12];
    /// let slice = string.try_slice_ref(slice).unwrap();
    /// assert_eq!(slice, "world");
    /// assert_eq!(string.try_slice_ref(b"other"), None);
    /// ```
    pub fn try_slice_ref(&self, slice: &[u8]) -> Option<Self> {
        try_slice_offset(self.string.get().as_bytes(), slice).map(|range| ImString {
            offset: range,
            ..self.clone()
        })
    }

    /// Promote a [`u8`] slice of this [`ImString`] into an [`ImString`].
    ///
    /// This method is useful when interfacing with algorithms that only work on byte slices,
    /// but you want to store the output strings as [`ImString`] values.
    ///
    /// # Panics
    ///
    /// If the given [`u8`] slice is not from this [`ImString`], or if the slice does not contain
    /// valid UTF-8, this method will panic.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```rust
    /// # use imstr::ImString;
    /// let string = ImString::from("Hello, world!");
    /// let slice = &string.as_bytes()[7..12];
    /// let slice = string.slice_ref(slice);
    /// assert_eq!(slice, "world");
    /// ```
    pub fn slice_ref(&self, slice: &[u8]) -> Self {
        self.try_slice_ref(slice).unwrap()
    }

    /// Try splitting the string into two at the given byte index.
    ///
    /// Returns a new [`ImString`] containing bytes `position..` and keeps bytes `0..position` in
    /// `self`. If `position` is not on a [`char`] boundary, or if it is beyond the last code point
    /// of the string, returns `None`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use imstr::ImString;
    /// let mut hello = ImString::from("Hello, World!");
    /// let world = hello.split_off(7);
    /// assert_eq!(hello, "Hello, ");
    /// assert_eq!(world, "World!");
    /// ```
    pub fn try_split_off(&mut self, position: usize) -> Option<Self> {
        if position > self.offset.end {
            return None;
        }

        if !self.as_str().is_char_boundary(position) {
            return None;
        }

        let new = ImString {
            offset: position..self.offset.end,
            ..self.clone()
        };

        self.offset.end = position;
        Some(new)
    }

    /// Split the string into two at the given byte index.
    ///
    /// Returns a new [`ImString`] containing bytes `position..` and keeps bytes `0..position` in
    /// `self`.
    ///
    /// # Panics
    ///
    /// Panics if `position` is not on a [`char`] boundary, or if it is beyond the last code point
    /// of the string. Use [`is_char_boundary()`](str::is_char_boundary) to check if an index is
    /// on a char boundary.
    ///
    /// Use [`try_split_off()`](ImString::try_split_off) if you want to handle invalid positions.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use imstr::ImString;
    /// let mut hello = ImString::from("Hello, World!");
    /// let world = hello.split_off(7);
    /// assert_eq!(hello, "Hello, ");
    /// assert_eq!(world, "World!");
    /// ```
    pub fn split_off(&mut self, position: usize) -> Self {
        self.try_split_off(position).unwrap()
    }

    /// Returns a clone of the underlying reference-counted shared `String`.
    ///
    /// This method provides access to the raw `Arc<String>` that backs the `ImString`.
    /// Note that the returned `Arc<String>` may contain more data than the `ImString` itself,
    /// depending on the `ImString`'s `offset`. To access the string contents represented
    /// by the `ImString`, consider using `as_str()` instead.
    ///
    /// # Examples
    ///
    /// ```
    /// use imstr::{ImString, data::Arc};
    ///
    /// let string: ImString = ImString::from("hello world");
    /// let raw_string: Arc<String> = string.raw_string();
    /// assert_eq!(&*raw_string, "hello world");
    /// ```
    pub fn raw_string(&self) -> S {
        self.string.clone()
    }

    /// Returns a clone of the `ImString`'s `offset` as a `Range<usize>`.
    ///
    /// The `offset` represents the start and end positions of the `ImString`'s view
    /// into the underlying `String`. This method is useful when you need to work with
    /// the raw offset values, for example, when creating a new `ImString` from a slice
    /// of the current one.
    ///
    /// # Examples
    ///
    /// ```
    /// use imstr::ImString;
    /// use std::ops::Range;
    ///
    /// let string: ImString = ImString::from("hello world");
    /// let raw_offset: Range<usize> = string.raw_offset();
    /// assert_eq!(raw_offset, 0..11);
    /// ```
    pub fn raw_offset(&self) -> Range<usize> {
        self.offset.clone()
    }

    /// Returns a reference of the `ImString`'s `offset` as a `Range<usize>`.
    ///
    /// The `offset` represents the start and end positions of the `ImString`'s view
    /// into the underlying `String`. This method is useful when you need to work with
    /// the raw offset values, for example, when creating a new `ImString` from a slice
    /// of the current one.
    ///
    /// # Examples
    ///
    /// ```
    /// use imstr::ImString;
    /// use std::ops::Range;
    ///
    /// let string: ImString = ImString::from("hello world");
    /// let raw_offset: &Range<usize> = string.raw_offset_ref();
    /// assert_eq!(raw_offset, &(0..11));
    /// ```
    pub fn raw_offset_ref(&self) -> &Range<usize> {
        &self.offset
    }

    /// Sets the `ImString`'s `offset` to the given `Range<usize>`.
    ///
    /// The `offset` represents the start and end positions of the `ImString`'s view
    /// into the underlying `String`. This method is useful when you need to work with
    /// the raw offset values, for example, when creating a new `ImString` from a slice
    /// of the current one.
    ///
    /// # Returns
    ///
    /// Returns an error if the given `offset` is not a valid range within the underlying `String`.
    ///
    /// # Examples
    ///
    /// ```
    /// use imstr::ImString;
    /// use std::ops::Range;
    ///
    /// let mut string: ImString = ImString::from("hello world");
    /// string.try_set_offset(0..5).unwrap();
    /// assert_eq!(string, "hello");
    /// ```
    pub fn try_set_offset(&mut self, range: impl RangeBounds<usize>) -> Result<(), SliceError> {
        let start = match range.start_bound() {
            Bound::Included(value) => *value,
            Bound::Excluded(value) => *value + 1,
            Bound::Unbounded => 0,
        };
        let end = match range.end_bound() {
            Bound::Included(value) => *value - 1,
            Bound::Excluded(value) => *value,
            Bound::Unbounded => self.offset.len(),
        };
        if end < start {
            return Err(SliceError::EndBeforeStart);
        }
        if !self.string.get().is_char_boundary(start) {
            return Err(SliceError::StartNotAligned);
        }
        if !self.string.get().is_char_boundary(end) {
            return Err(SliceError::EndNotAligned);
        }

        self.offset = start..end;
        Ok(())
    }

    /// An iterator over the lines of a string.
    ///
    /// Lines are split at line endings that are either newlines (`\n`) or sequences of a carriage
    /// return followed by a line feed (`\r\n`).
    ///
    /// Line terminators are not included in the lines returned by the iterator.
    ///
    /// The final line ending is optional. A string that ends with a final line ending will return
    /// the same lines as an otherwise identical string without a final line ending.
    ///
    /// This works the same way as [str::lines](str::lines), except that it
    /// returns ImString instances.
    pub fn lines(&self) -> Lines<'_, S> {
        ImStringIterator::new(self.string.clone(), self.as_str().lines())
    }

    /// Iterator over chars in an ImString.
    pub fn chars(&self) -> Chars<S> {
        Chars {
            string: self.clone(),
        }
    }

    /// Iterators over `char`s with their corresponding index in an `ImString`.
    pub fn char_indices(&self) -> CharIndices<S> {
        CharIndices {
            offset: 0,
            string: self.clone(),
        }
    }

    /// Returns a slice of this string with leading and trailing whitespace removed.
    ///
    /// *Whitespace* is defined according to the terms of the Unicode Derived Core Property
    /// `White_Space`, which includes newlines.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```rust
    /// # use imstr::ImString;
    /// let string = ImString::from("\n Hello\tWorld\t\n");
    /// assert_eq!(string.trim(), "Hello\tWorld");
    /// ```
    pub fn trim(&self) -> Self {
        self.str_ref(self.as_str().trim())
    }

    /// Returns a slice of this string with leading whitespace removed.
    ///
    /// *Whitespace* is defined according to the terms of the Unicode Derived Core Property
    /// `White_Space`, which includes newlines.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```rust
    /// # use imstr::ImString;
    /// let string = ImString::from("\n Hello\tworld\t\n");
    /// assert_eq!(string.trim_start(), "Hello\tworld\t\n");
    /// ```
    pub fn trim_start(&self) -> Self {
        self.str_ref(self.as_str().trim_start())
    }

    /// Returns a slice of this string with trailing whitespace removed.
    ///
    /// *Whitespace* is defined according to the terms of the Unicode Derived Core Property
    /// `White_Space`, which includes newlines.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```rust
    /// # use imstr::ImString;
    /// let string = ImString::from("\n Hello\tworld\t\n");
    /// assert_eq!(string.trim_end(), "\n Hello\tworld");
    /// ```
    pub fn trim_end(&self) -> Self {
        self.str_ref(self.as_str().trim_end())
    }
}

impl<S: Data<String>> Default for ImString<S> {
    fn default() -> Self {
        ImString::new()
    }
}

impl<S: Data<String>> From<&str> for ImString<S> {
    fn from(string: &str) -> Self {
        ImString::from_std_string(string.to_string())
    }
}

impl<S: Data<String>> From<char> for ImString<S> {
    fn from(c: char) -> Self {
        String::from(c).into()
    }
}

impl<S: Data<String>> From<String> for ImString<S> {
    fn from(string: String) -> Self {
        ImString::from_std_string(string)
    }
}

impl<'a, S: Data<String>> From<Cow<'a, str>> for ImString<S> {
    fn from(string: Cow<'a, str>) -> Self {
        ImString::from(string.into_owned())
    }
}

impl<S: Data<String>> From<ImString<S>> for String {
    fn from(string: ImString<S>) -> Self {
        string.into_std_string()
    }
}

impl<S: Data<String>> PartialEq<str> for ImString<S> {
    fn eq(&self, other: &str) -> bool {
        self.as_str().eq(other)
    }
}

impl<'a, S: Data<String>> PartialEq<&'a str> for ImString<S> {
    fn eq(&self, other: &&'a str) -> bool {
        self.as_str().eq(*other)
    }
}

impl<S: Data<String>> PartialEq<String> for ImString<S> {
    fn eq(&self, other: &String) -> bool {
        self.as_str().eq(other.as_str())
    }
}

impl<S: Data<String>, O: Data<String>> PartialEq<ImString<O>> for ImString<S> {
    fn eq(&self, other: &ImString<O>) -> bool {
        self.as_str().eq(other.as_str())
    }
}

impl<S: Data<String>> Eq for ImString<S> {}

impl<S: Data<String>> PartialOrd<ImString<S>> for ImString<S> {
    fn partial_cmp(&self, other: &ImString<S>) -> Option<Ordering> {
        self.as_str().partial_cmp(other.as_str())
    }
}

impl<S: Data<String>> Ord for ImString<S> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl<S: Data<String>> Debug for ImString<S> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        Debug::fmt(self.as_str(), f)
    }
}

impl<S: Data<String>> Display for ImString<S> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result<(), FmtError> {
        Display::fmt(self.as_str(), formatter)
    }
}

impl<S: Data<String>> FromStr for ImString<S> {
    type Err = Infallible;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(ImString::from(s))
    }
}

// Delegate hash to contained str. This is important!
impl<S: Data<String>> Hash for ImString<S> {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        self.as_str().hash(hasher)
    }
}

impl<S: Data<String>> Write for ImString<S> {
    fn write_str(&mut self, string: &str) -> Result<(), FmtError> {
        self.push_str(string);
        Ok(())
    }

    fn write_char(&mut self, c: char) -> Result<(), FmtError> {
        self.push(c);
        Ok(())
    }
}

impl<S: Data<String>> Index<Range<usize>> for ImString<S> {
    type Output = str;
    fn index(&self, index: Range<usize>) -> &str {
        &self.as_str()[index]
    }
}

impl<S: Data<String>> Index<RangeFrom<usize>> for ImString<S> {
    type Output = str;
    fn index(&self, index: RangeFrom<usize>) -> &str {
        &self.as_str()[index]
    }
}

impl<S: Data<String>> Index<RangeFull> for ImString<S> {
    type Output = str;
    fn index(&self, index: RangeFull) -> &str {
        &self.as_str()[index]
    }
}

impl<S: Data<String>> Index<RangeInclusive<usize>> for ImString<S> {
    type Output = str;
    fn index(&self, index: RangeInclusive<usize>) -> &str {
        &self.as_str()[index]
    }
}

impl<S: Data<String>> Index<RangeTo<usize>> for ImString<S> {
    type Output = str;
    fn index(&self, index: RangeTo<usize>) -> &str {
        &self.as_str()[index]
    }
}

impl<S: Data<String>> IndexMut<Range<usize>> for ImString<S> {
    fn index_mut(&mut self, index: Range<usize>) -> &mut str {
        &mut self.as_mut_str()[index]
    }
}

impl<S: Data<String>> IndexMut<RangeFrom<usize>> for ImString<S> {
    fn index_mut(&mut self, index: RangeFrom<usize>) -> &mut str {
        &mut self.as_mut_str()[index]
    }
}

impl<S: Data<String>> IndexMut<RangeFull> for ImString<S> {
    fn index_mut(&mut self, index: RangeFull) -> &mut str {
        &mut self.as_mut_str()[index]
    }
}

impl<S: Data<String>> IndexMut<RangeInclusive<usize>> for ImString<S> {
    fn index_mut(&mut self, index: RangeInclusive<usize>) -> &mut str {
        &mut self.as_mut_str()[index]
    }
}

impl<S: Data<String>> IndexMut<RangeTo<usize>> for ImString<S> {
    fn index_mut(&mut self, index: RangeTo<usize>) -> &mut str {
        &mut self.as_mut_str()[index]
    }
}

/// Iterator over lines of an [`ImString`].
///
/// Unlike the [`Lines`](std::str::Lines) iterator of [`str`], this iterator returns instances of
/// [`ImString`].
///
/// # Example
///
/// ```rust
/// # use imstr::ImString;
/// let string = ImString::from("multi\nline\ninput");
/// let lines: Vec<ImString> = string.lines().collect();
/// assert_eq!(lines[0], "multi");
/// assert_eq!(lines[1], "line");
/// assert_eq!(lines[2], "input");
/// ```
pub type Lines<'a, S> = ImStringIterator<'a, S, core::str::Lines<'a>>;

/// Iterator wrapper over string slices of an [`ImString`].
///
/// This iterator wrapper turns string slices of an [`ImString`] into [`ImString`]s.
pub struct ImStringIterator<'a, S: Data<String>, I: Iterator<Item = &'a str>> {
    string: S,
    iterator: I,
}

impl<'a, S: Data<String>, I: Iterator<Item = &'a str>> Iterator for ImStringIterator<'a, S, I> {
    type Item = ImString<S>;
    fn next(&mut self) -> Option<Self::Item> {
        match self.iterator.next() {
            Some(slice) => {
                let offset =
                    try_slice_offset(self.string.get().as_bytes(), slice.as_bytes()).unwrap();
                Some(ImString {
                    string: self.string.clone(),
                    offset,
                })
            }
            None => None,
        }
    }
}

impl<'a, S: Data<String>, I: Iterator<Item = &'a str>> ImStringIterator<'a, S, I> {
    fn new(string: S, iterator: I) -> Self {
        ImStringIterator { string, iterator }
    }
}

/// Iterator over `char`s with their corresponding byte index inside an `ImString`.
#[derive(Clone, Debug)]
pub struct CharIndices<S: Data<String>> {
    offset: usize,
    string: ImString<S>,
}

impl<S: Data<String>> Iterator for CharIndices<S> {
    type Item = (usize, char);
    fn next(&mut self) -> Option<Self::Item> {
        match self.string.as_str().chars().next() {
            Some(c) => {
                let len = c.len_utf8();
                self.string = self.string.slice(len..);
                let offset = self.offset;
                self.offset += len;
                Some((offset, c))
            }
            None => None,
        }
    }
}

/// Iterator over `char`s inside an `ImString`.
#[derive(Clone, Debug)]
pub struct Chars<S: Data<String>> {
    string: ImString<S>,
}

impl<S: Data<String>> Iterator for Chars<S> {
    type Item = char;
    fn next(&mut self) -> Option<Self::Item> {
        match self.string.as_str().chars().next() {
            Some(c) => {
                self.string = self.string.slice(c.len_utf8()..);
                Some(c)
            }
            None => None,
        }
    }
}

impl<S: Data<String>> Deref for ImString<S> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl<S: Data<String>> DerefMut for ImString<S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_str()
    }
}

impl<S: Data<String>> Borrow<str> for ImString<S> {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl<S: Data<String>> BorrowMut<str> for ImString<S> {
    fn borrow_mut(&mut self) -> &mut str {
        self.as_mut_str()
    }
}

impl<S: Data<String>> AsRef<str> for ImString<S> {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

#[cfg(feature = "std")]
impl<S: Data<String>> AsRef<Path> for ImString<S> {
    fn as_ref(&self) -> &Path {
        self.as_str().as_ref()
    }
}

#[cfg(feature = "std")]
impl<S: Data<String>> AsRef<OsStr> for ImString<S> {
    fn as_ref(&self) -> &OsStr {
        self.as_str().as_ref()
    }
}

impl<S: Data<String>> AsRef<[u8]> for ImString<S> {
    fn as_ref(&self) -> &[u8] {
        self.as_str().as_ref()
    }
}

impl<S: Data<String>> AsMut<str> for ImString<S> {
    fn as_mut(&mut self) -> &mut str {
        self.as_mut_str()
    }
}

#[cfg(feature = "std")]
impl<S: Data<String>> ToSocketAddrs for ImString<S> {
    type Iter = <String as ToSocketAddrs>::Iter;
    fn to_socket_addrs(&self) -> std::io::Result<<String as ToSocketAddrs>::Iter> {
        self.as_str().to_socket_addrs()
    }
}

impl<S: Data<String>> Add<&str> for ImString<S> {
    type Output = ImString<S>;
    fn add(mut self, string: &str) -> Self::Output {
        self.push_str(string);
        self
    }
}

impl<S: Data<String>> AddAssign<&str> for ImString<S> {
    fn add_assign(&mut self, string: &str) {
        self.push_str(string);
    }
}

impl<S: Data<String>> Extend<char> for ImString<S> {
    fn extend<T: IntoIterator<Item = char>>(&mut self, iter: T) {
        unsafe {
            self.unchecked_append(|mut string| {
                string.extend(iter);
                string
            });
        }
    }
}

impl<'a, S: Data<String>> Extend<&'a char> for ImString<S> {
    fn extend<T: IntoIterator<Item = &'a char>>(&mut self, iter: T) {
        unsafe {
            self.unchecked_append(|mut string| {
                string.extend(iter);
                string
            });
        }
    }
}

impl<'a, S: Data<String>> Extend<&'a str> for ImString<S> {
    fn extend<T: IntoIterator<Item = &'a str>>(&mut self, iter: T) {
        unsafe {
            self.unchecked_append(|mut string| {
                string.extend(iter);
                string
            });
        }
    }
}

impl<S: Data<String>> FromIterator<char> for ImString<S> {
    fn from_iter<T: IntoIterator<Item = char>>(iter: T) -> Self {
        let mut string = ImString::new();
        string.extend(iter);
        string
    }
}

impl<'a, S: Data<String>> FromIterator<&'a char> for ImString<S> {
    fn from_iter<T: IntoIterator<Item = &'a char>>(iter: T) -> Self {
        let mut string = ImString::new();
        string.extend(iter);
        string
    }
}

impl<'a, S: Data<String>> FromIterator<&'a str> for ImString<S> {
    fn from_iter<T: IntoIterator<Item = &'a str>>(iter: T) -> Self {
        let mut string = ImString::new();
        string.extend(iter);
        string
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::Cloned;
    use alloc::boxed::Box;
    use alloc::format;
    use alloc::vec;

    fn test_strings<S: Data<String>>() -> Vec<ImString<S>> {
        let long = ImString::from("long string here");
        let world = ImString::from("world");
        let some = ImString::from("some");
        let multiline = ImString::from("some\nmulti\nline\nstring\nthat\nis\nlong");
        let large: ImString<S> = (0..100).map(|_| "hello\n").collect();
        vec![
            ImString::new(),
            ImString::default(),
            ImString::from(""),
            ImString::from("a"),
            ImString::from("ü"),
            ImString::from("hello"),
            ImString::from("0.0.0.0:800"),
            ImString::from("localhost:1234"),
            ImString::from("0.0.0.0:1234"),
            large.slice(0..6),
            large.slice(6..12),
            large.slice(..),
            long.clone(),
            long.slice(4..10),
            long.slice(0..4),
            long.slice(4..4),
            long.slice(5..),
            long.slice(..),
            world.clone(),
            world.clone(),
            some.slice(4..),
            some,
            multiline.slice(5..15),
            multiline,
            ImString::from("\u{e4}\u{fc}\u{f6}\u{f8}\u{3a9}"),
            ImString::from("\u{1f600}\u{1f603}\u{1f604}"),
            ImString::from("o\u{308}u\u{308}a\u{308}"),
        ]
    }

    macro_rules! tests {
        () => {};
        (#[test] fn $name:ident <S: Data<String>>() $body:tt $($rest:tt)*) => {
            #[test]
            fn $name() {
                fn $name <S: Data<String>>() $body
                $name::<Threadsafe>();
                $name::<Local>();
                $name::<Cloned<String>>();
                $name::<Box<String>>();
            }
            tests!{$($rest)*}
        };
        (#[test] fn $name:ident <S: Data<String>>($string:ident: ImString<S>) $body:tt $($rest:tt)*) => {
            #[test]
            fn $name() {
                fn $name <S: Data<String>>() {
                    fn $name <S: Data<String>>($string: ImString<S>) $body
                    for string in test_strings::<S>().into_iter() {
                        $name(string);
                    }
                }
                $name::<Threadsafe>();
                $name::<Local>();
                $name::<Cloned<String>>();
                $name::<Box<String>>();
            }
            tests!{$($rest)*}
        }
    }

    tests! {
        #[test]
        fn test_new<S: Data<String>>() {
            let string: ImString<S> = ImString::new();
            assert_eq!(string.string.get().len(), 0);
            assert_eq!(string.offset, 0..0);
        }

        #[test]
        fn test_default<S: Data<String>>() {
            let string: ImString<S> = ImString::new();
            assert_eq!(string.string.get().len(), 0);
            assert_eq!(string.offset, 0..0);
        }

        #[test]
        fn test_with_capacity<S: Data<String>>() {
            for capacity in [10, 100, 256] {
                let string: ImString<S> = ImString::with_capacity(capacity);
                assert!(string.capacity() >= capacity);
                assert_eq!(string.string.get().len(), 0);
                assert_eq!(string.offset, 0..0);
            }
        }

        #[test]
        fn test_offset<S: Data<String>>(string: ImString<S>) {
            assert!(string.offset.start <= string.string.get().len());
            assert!(string.offset.start <= string.offset.end);
            assert!(string.offset.end <= string.string.get().len());
        }

        #[test]
        fn test_as_str<S: Data<String>>(string: ImString<S>) {
            assert_eq!(string.as_str(), &string.string.get()[string.offset.clone()]);
            assert_eq!(string.as_str().len(), string.len());
        }

        #[test]
        fn test_as_mut_str<S: Data<String>>(string: ImString<S>) {
            // ascii uppercase copy of string
            let string_uppercase = string.as_str().to_ascii_uppercase();

            // uppercase in-place
            let mut string = string;
            let string_slice = string.as_mut_str();
            string_slice.make_ascii_uppercase();

            // make sure both versions are identical
            assert_eq!(string_uppercase, &*string_slice);
            drop(string_slice);
            assert_eq!(string, string_uppercase);
        }

        #[test]
        fn test_as_bytes<S: Data<String>>(string: ImString<S>) {
            assert_eq!(string.as_bytes(), &string.string.get().as_bytes()[string.offset.clone()]);
            assert_eq!(string.as_bytes().len(), string.len());
        }

        #[test]
        fn test_len<S: Data<String>>(string: ImString<S>) {
            assert_eq!(string.len(), string.offset.len());
            assert_eq!(string.len(), string.as_str().len());
            assert_eq!(string.len(), string.as_bytes().len());
        }

        #[test]
        fn test_clear<S: Data<String>>(string: ImString<S>) {
            let mut string = string;
            string.clear();
            assert_eq!(string.as_str(), "");
            assert_eq!(string.len(), 0);
        }

        #[test]
        fn test_debug<S: Data<String>>(string: ImString<S>) {
            let debug_string = format!("{string:?}");
            let debug_str = format!("{:?}", string.as_str());
            assert_eq!(debug_string, debug_str);
        }

        #[test]
        fn test_deref<S: Data<String>>(string: ImString<S>) {
            assert_eq!(string.deref(), string.as_str());
        }

        #[test]
        fn test_clone<S: Data<String>>(string: ImString<S>) {
            assert_eq!(string, string.clone());
        }

        #[test]
        fn test_display<S: Data<String>>(string: ImString<S>) {
            let display_string = format!("{string}");
            let display_str = format!("{}", string.as_str());
            assert_eq!(display_string, display_str);
        }

        #[test]
        fn test_insert_start<S: Data<String>>(string: ImString<S>) {
            let mut string = string;
            let length = string.len();
            string.insert(0, 'h');
            assert_eq!(string.len(), length + 1);
            assert_eq!(string.chars().nth(0), Some('h'));
        }

        #[test]
        fn test_insert_one<S: Data<String>>(string: ImString<S>) {
            if !string.is_empty() && string.is_char_boundary(1) {
                let mut string = string;
                let length = string.len();
                string.insert(1, 'h');
                assert_eq!(string.len(), length + 1);
                assert_eq!(string.chars().nth(1), Some('h'));
            }
        }

        #[test]
        fn test_insert_end<S: Data<String>>(string: ImString<S>) {
            let mut string = string;
            let length = string.len();
            string.insert(length, 'h');
            assert_eq!(string.len(), length + 1);
            // FIXME
            //assert_eq!(string.chars().nth(length), Some('h'));
        }

        #[test]
        fn test_insert_str_start<S: Data<String>>(string: ImString<S>) {
            // test to make sure we can insert_str at the start
            let original = string.as_str().to_string();
            let mut string = string;
            let inserted = "hello";
            string.insert_str(0, inserted);
            assert_eq!(string.len(), original.len() + inserted.len());
            assert_eq!(&string[0..inserted.len()], inserted);
            assert_eq!(&string[inserted.len()..], original);
        }

        #[test]
        fn test_insert_str_end<S: Data<String>>(string: ImString<S>) {
            // test to make sure we can insert_str at the end
            let original = string.as_str().to_string();
            let mut string = string;
            let inserted = "hello";
            string.insert_str(string.len(), inserted);
            assert_eq!(string.len(), original.len() + inserted.len());
            assert_eq!(&string[0..original.len()], original);
            assert_eq!(&string[original.len()..], inserted);
        }

        #[test]
        fn test_is_empty<S: Data<String>>(string: ImString<S>) {
            assert_eq!(string.is_empty(), string.len() == 0);
            assert_eq!(string.is_empty(), string.as_str().is_empty());
        }

        #[test]
        fn test_push<S: Data<String>>(string: ImString<S>) {
            let mut string = string;
            let mut std_string = string.as_str().to_string();
            let c = 'c';
            std_string.push(c);
            string.push(c);
            assert_eq!(string, std_string);
        }

        #[test]
        fn test_push_str<S: Data<String>>(string: ImString<S>) {
            let mut string = string;
            let mut std_string = string.as_str().to_string();
            let s = "string";
            std_string.push_str(s);
            string.push_str(s);
            assert_eq!(string, std_string);
        }

        #[test]
        fn test_pop<S: Data<String>>(string: ImString<S>) {
            let mut characters: Vec<char> = string.chars().collect();
            let mut string = string;
            loop {
                let c1 = characters.pop();
                let c2 = string.pop();
                assert_eq!(c1, c2);
                if c1.is_none() {
                    break;
                }
            }
        }

        #[test]
        fn test_index_range_full<S: Data<String>>(string: ImString<S>) {
            assert_eq!(&string[..], &string.as_str()[..]);
        }

        #[test]
        fn test_index_range_from<S: Data<String>>(string: ImString<S>) {
            for i in (0..string.len()).filter(|i| string.is_char_boundary(*i)) {
                assert_eq!(&string[i..], &string.as_str()[i..]);
            }
        }

        #[test]
        fn test_index_range_to<S: Data<String>>(string: ImString<S>) {
            for i in (0..string.len()).filter(|i| string.is_char_boundary(*i)) {
                assert_eq!(&string[..i], &string.as_str()[..i]);
            }
        }

        #[test]
        fn test_index_range_exclusive<S: Data<String>>(string: ImString<S>) {
            for start in (0..string.len()).filter(|i| string.is_char_boundary(*i)) {
                for end in (start..string.len()).filter(|i| string.is_char_boundary(*i)) {
                    assert_eq!(&string[start..end], &string.as_str()[start..end]);
                }
            }
        }

        #[test]
        fn test_index_range_inclusive<S: Data<String>>(string: ImString<S>) {
            if !string.is_empty() {
                for start in (0..string.len()-1).filter(|i| string.is_char_boundary(*i)) {
                    for end in (start..string.len()-1).filter(|i| string.is_char_boundary(*i + 1)) {
                        assert_eq!(&string[start..=end], &string.as_str()[start..=end]);
                    }
                }
            }
        }

        #[test]
        fn test_into_bytes<S: Data<String>>(string: ImString<S>) {
            let std_bytes = string.as_str().to_string().into_bytes();
            let bytes = string.into_bytes();
            assert_eq!(bytes, std_bytes);
        }

        #[test]
        fn test_slice_all<S: Data<String>>(string: ImString<S>) {
            assert_eq!(string.slice(..), string);
        }

        #[test]
        fn test_slice_start<S: Data<String>>(string: ImString<S>) {
            for end in 0..string.len() {
                if string.is_char_boundary(end) {
                    assert_eq!(string.slice(..end), string.as_str()[..end]);
                }
            }
        }

        #[test]
        fn test_slice_end<S: Data<String>>(string: ImString<S>) {
            for start in 0..string.len() {
                if string.is_char_boundary(start) {
                    assert_eq!(string.slice(start..), string.as_str()[start..]);
                }
            }
        }

        #[test]
        fn test_slice_middle<S: Data<String>>(string: ImString<S>) {
            for start in 0..string.len() {
                if string.is_char_boundary(start) {
                    for end in start..string.len() {
                        if string.is_char_boundary(end) {
                            assert_eq!(string.slice(start..end), string.as_str()[start..end]);
                        }
                    }
                }
            }
        }

        #[test]
        fn test_try_slice_all<S: Data<String>>(string: ImString<S>) {
            assert_eq!(string.try_slice(..).unwrap(), string);
        }

        #[test]
        fn test_try_slice_start<S: Data<String>>(string: ImString<S>) {
            for end in 0..string.len() {
                if string.is_char_boundary(end) {
                    assert_eq!(string.try_slice(..end).unwrap(), string.as_str()[..end]);
                } else {
                    // cannot get slice with end in middle of UTF-8 multibyte sequence.
                    assert_eq!(string.try_slice(..end), Err(SliceError::EndNotAligned));
                }
            }

            // cannot get slice with end pointing past the end of the string.
            assert_eq!(string.try_slice(..string.len()+1), Err(SliceError::EndOutOfBounds));
        }

        #[test]
        fn test_try_slice_end<S: Data<String>>(string: ImString<S>) {
            for start in 0..string.len() {
                if string.is_char_boundary(start) {
                    assert_eq!(string.try_slice(start..).unwrap(), string.as_str()[start..]);
                } else {
                    // cannot get slice with end in middle of UTF-8 multibyte sequence.
                    assert_eq!(string.try_slice(start..), Err(SliceError::StartNotAligned));
                }
            }

            // cannot get slice with end pointing past the end of the string.
            assert_eq!(string.try_slice(string.len()+1..), Err(SliceError::StartOutOfBounds));
        }

        #[test]
        fn test_write<S: Data<String>>() {
            let mut string: ImString<S> = ImString::new();
            string.write_str("Hello").unwrap();
            string.write_char(',').unwrap();
            string.write_char(' ').unwrap();
            string.write_str("World").unwrap();
            string.write_char('!').unwrap();
            assert_eq!(string, "Hello, World!");

        }

        #[test]
        fn test_add_assign<S: Data<String>>(string: ImString<S>) {
            let mut std_string = string.as_str().to_string();
            let mut string = string;
            string += "hello";
            std_string += "hello";
            assert_eq!(string, std_string);
        }

        #[test]
        fn test_add<S: Data<String>>(string: ImString<S>) {
            let std_string = string.as_str().to_string();
            let std_string = std_string + "hello";
            let string = string + "hello";
            assert_eq!(string, std_string);
        }

        #[test]
        fn test_to_socket_addrs<S: Data<String>>(_string: ImString<S>) {
            #[cfg(all(feature = "std", not(miri)))]
            {
                let addrs = _string.to_socket_addrs().map(|s| s.collect::<Vec<_>>());
                let str_addrs = _string.as_str().to_socket_addrs().map(|s| s.collect::<Vec<_>>());
                match addrs {
                    Ok(addrs) => assert_eq!(addrs, str_addrs.unwrap()),
                    Err(_err) => assert!(str_addrs.is_err()),
                }
            }
        }

        #[test]
        fn test_from_iterator_char<S: Data<String>>() {
            let input = ['h', 'e', 'l', 'l', 'o'];
            let string: ImString<S> = input.into_iter().collect();
            assert_eq!(string, "hello");
        }

        #[test]
        fn test_from_iterator_char_ref<S: Data<String>>() {
            let input = ['h', 'e', 'l', 'l', 'o'];
            let string: ImString<S> = input.iter().collect();
            assert_eq!(string, "hello");
        }

        #[test]
        fn test_from_iterator_str<S: Data<String>>() {
            let input = ["hello", "world", "!"];
            let string: ImString<S> = input.into_iter().collect();
            assert_eq!(string, "helloworld!");
        }

        #[test]
        fn test_extend_char<S: Data<String>>() {
            let input = ['h', 'e', 'l', 'l', 'o'];
            let mut string: ImString<S> = ImString::new();
            string.extend(input.into_iter());
            assert_eq!(string, "hello");
        }

        #[test]
        fn test_extend_char_ref<S: Data<String>>() {
            let input = ['h', 'e', 'l', 'l', 'o'];
            let mut string: ImString<S> = ImString::new();
            string.extend(input.into_iter());
            assert_eq!(string, "hello");
        }

        #[test]
        fn test_extend_str<S: Data<String>>() {
            let input = ["hello", "world", "!"];
            let mut string: ImString<S> = ImString::new();
            string.extend(input.into_iter());
            assert_eq!(string, "helloworld!");
        }

        #[test]
        fn test_from_utf8_lossy<S: Data<String>>() {
            let string: ImString<S> = ImString::from_utf8_lossy(b"hello");
            assert_eq!(string, "hello");
        }

        #[test]
        fn test_from_utf8_unchecked<S: Data<String>>() {
            let string: ImString<S> = unsafe {
                ImString::from_utf8_unchecked(b"hello".to_vec())
            };
            assert_eq!(string, "hello");
        }

        #[test]
        fn test_borrow<S: Data<String>>(string: ImString<S>) {
            let s: &str = string.borrow();
            assert_eq!(s, string.as_str());
        }

        #[test]
        fn test_as_ref_str<S: Data<String>>(string: ImString<S>) {
            let s: &str = string.as_ref();
            assert_eq!(s, string.as_str());
        }

        #[test]
        fn test_as_ref_bytes<S: Data<String>>(string: ImString<S>) {
            let s: &[u8] = string.as_ref();
            assert_eq!(s, string.as_bytes());
        }

        #[test]
        fn test_as_ref_path<S: Data<String>>(_string: ImString<S>) {
            #[cfg(feature = "std")]
            {
                let s: &Path = _string.as_ref();
                assert_eq!(s, _string.as_str().as_ref() as &Path);
            }
        }

        #[test]
        fn test_as_ref_os_str<S: Data<String>>(_string: ImString<S>) {
            #[cfg(feature = "std")]
            {
                let s: &OsStr = _string.as_ref();
                assert_eq!(s, _string.as_str().as_ref() as &OsStr);
            }
        }

        #[test]
        fn test_deref_mut<S: Data<String>>(string: ImString<S>) {
            let mut string = string;
            let data = string.as_str().to_string();
            let mutable: &mut str = string.deref_mut();
            assert_eq!(&*mutable, &data);
        }

        #[test]
        fn test_as_mut<S: Data<String>>(string: ImString<S>) {
            let mut string = string;
            let data = string.as_str().to_string();
            let mutable: &mut str = string.as_mut();
            assert_eq!(&*mutable, &data);
        }

        #[test]
        fn test_borrow_mut<S: Data<String>>(string: ImString<S>) {
            let mut string = string;
            let data = string.as_str().to_string();
            let mutable: &mut str = string.borrow_mut();
            assert_eq!(&*mutable, &data);
        }

        #[test]
        fn test_partial_eq<S: Data<String>>(string: ImString<S>) {
            assert_eq!(string, string.as_str());
            assert_eq!(string, string.to_string());
            assert_eq!(string, string);
        }

        #[test]
        fn test_partial_ord<S: Data<String>>(string: ImString<S>) {
            let other = ImString::from("test");
            assert_eq!(string.as_str().partial_cmp(other.as_str()), string.partial_cmp(&other));
            assert_eq!(other.as_str().partial_cmp(string.as_str()), other.partial_cmp(&string));
        }

        #[test]
        fn test_ord<S: Data<String>>(string: ImString<S>) {
            let other = ImString::from("test");
            assert_eq!(string.as_str().cmp(other.as_str()), string.cmp(&other));
            assert_eq!(other.as_str().cmp(string.as_str()), other.cmp(&string));
        }

        #[test]
        fn test_from<S: Data<String>>(string: ImString<S>) {
            let std_string: String = string.clone().into();
            assert_eq!(string, std_string);
        }

        #[test]
        fn test_raw_offset<S: Data<String>>(string: ImString<S>) {
            assert_eq!(string.offset, string.raw_offset());
        }

        #[test]
        fn test_raw_string<S: Data<String>>(string: ImString<S>) {
            assert_eq!(string.string.get(), string.raw_string().get());
        }

        #[test]
        fn into_std_string<S: Data<String>>(string: ImString<S>) {
            let std_clone = string.as_str().to_string();
            let std_string = string.into_std_string();
            assert_eq!(std_clone, std_string);
        }

        #[test]
        fn test_truncate<S: Data<String>>(string: ImString<S>) {
            let mut clone = string.as_str().to_string();
            let mut string = string;

            for length in (0..string.len()).rev() {
                if string.is_char_boundary(length) {
                    string.truncate(length);
                    clone.truncate(length);
                    assert_eq!(string, clone);
                }
            }
        }

        #[test]
        fn test_str_ref<S: Data<String>>(string: ImString<S>) {
            assert_eq!(string, string.str_ref(string.as_str()));
        }

        #[test]
        fn test_try_str_ref<S: Data<String>>(string: ImString<S>) {
            assert_eq!(string, string.try_str_ref(string.as_str()).unwrap());
            assert_eq!(string.try_str_ref("test"), None);
        }

        #[test]
        fn test_slice_ref<S: Data<String>>(string: ImString<S>) {
            assert_eq!(string, string.slice_ref(string.as_bytes()));
        }

        #[test]
        fn test_try_slice_ref<S: Data<String>>(string: ImString<S>) {
            assert_eq!(string, string.try_slice_ref(string.as_bytes()).unwrap());
            assert_eq!(string.try_slice_ref(b"test"), None);
        }
    }
}
