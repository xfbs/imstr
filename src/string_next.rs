use crate::data::Data;
use crate::error::*;
use std::borrow::{Borrow, BorrowMut, Cow};
use std::cmp::Ordering;
use std::convert::{AsMut, AsRef, Infallible};
use std::ffi::OsStr;
use std::fmt::{Debug, Display, Error as FmtError, Formatter, Write};
use std::hash::{Hash, Hasher};
use std::iter::{Extend, FromIterator};
use std::net::{SocketAddr, ToSocketAddrs};
use std::ops::{
    Add, AddAssign, Bound, Deref, Index, IndexMut, Range, RangeBounds, RangeFrom, RangeFull,
    RangeInclusive, RangeTo,
};
use std::path::Path;
use std::rc::Rc;
use std::str::FromStr;
use std::string::{String, ToString};
use std::sync::Arc;
use std::vec::IntoIter;

/// Threadsafe shared storage for string.
pub type Threadsafe = Arc<String>;

/// Non-threadsafe shared storage for string.
pub type Local = Rc<String>;

/// Non-shared storage for string.
pub type Cloned = crate::data::Cloned<String>;

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
/// The `ImString` struct contains two fields:
///
/// - `string`: An [`Arc`](std::sync::Arc) wrapping a `String`, which ensures
///   that the underlying `String` data is shared and reference-counted.
///
/// - `offset`: A [`Range`](std::ops::Range) that defines the start and end
///   positions of the `ImString`'s view into the underlying `String`. The
///   `offset` must always point to a valid UTF-8 region inside the `string`.
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
/// //let string_slice = string.slice(0..5);
/// //assert_eq!(string_slice, "hello");
/// ```
#[derive(Clone)]
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

    /// Return the backing [String](std::string::String)'s contents, in bytes.
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
    /// This method will construct the `ImString` without needing to clone the `String` instance.
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

    unsafe fn try_modify_unchecked<F: FnOnce(&mut String)>(&mut self, f: F) -> bool {
        if let Some(mut string) = self.string.get_mut() {
            f(string);
            true
        } else {
            false
        }
    }

    /// Creates a new string with the given capacity.
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

    /// Convert this string into a standard library [String](std::string::String).
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
        if self.offset.start != 0 {
            return self.as_str().to_string();
        }

        if let Some(mut string) = self.string.get_mut() {
            if string.len() != self.offset.end {
                string.truncate(self.offset.end);
            }

            std::mem::take(string)
        } else {
            self.as_str().to_string()
        }
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
        unsafe { std::str::from_utf8_unchecked(slice) }
    }

    /// Converts a vector of bytes to a ImString.
    pub fn from_utf8(vec: Vec<u8>) -> Result<Self, FromUtf8Error> {
        Ok(ImString::from_std_string(String::from_utf8(vec)?))
    }

    /// Converts a slice of bytes to a string, including invalid characters.
    pub fn from_utf8_lossy(bytes: &[u8]) -> Self {
        let string = String::from_utf8_lossy(bytes).into_owned();
        ImString::from_std_string(string)
    }

    /// Converts a vector of bytes to a ImString.
    pub unsafe fn from_utf8_unchecked(vec: Vec<u8>) -> Self {
        ImString::from_std_string(String::from_utf8_unchecked(vec))
    }

    unsafe fn unchecked_append<F: FnOnce(String) -> String>(&mut self, f: F) {
        if let Some(mut string_ref) = self.string.get_mut() {
            let string: String = std::mem::take(&mut string_ref);
            *string_ref = f(string);
        } else {
            self.string = S::new(f(self.as_str().to_string()));
            self.offset.start = 0;
        }

        self.offset.end = self.string.get().as_bytes().len();
    }

    /// Inserts a character into this string at the specified index.
    ///
    /// This is an *O(n)$ operation as it requires copying every element in the buffer.
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
    /// This is an *O(n)$ operation as it requires copying every element in the buffer.
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

    pub fn truncate(&mut self, length: usize) {
        if let Some(mut string) = self.string.get_mut() {
            string.truncate(length);
        } else {
            self.offset.end = self.offset.end.min(length);
        }
    }

    pub fn push(&mut self, c: char) {
        unsafe {
            self.unchecked_append(|mut string| {
                string.push(c);
                string
            });
        }
    }

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

    /// Create a subslice of this string.
    ///
    /// This will panic if the specified range is invalid. Use the [try_slice](ImString::try_slice)
    /// method if you want to handle invalid ranges.
    pub fn slice(&self, range: impl RangeBounds<usize>) -> Self {
        self.try_slice(range).unwrap()
    }

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

    pub fn try_str_ref(&self, string: &str) -> Option<Self> {
        self.try_slice_ref(string.as_bytes())
    }

    pub fn str_ref(&self, string: &str) -> Self {
        self.try_str_ref(string).unwrap()
    }

    pub fn try_slice_ref(&self, slice: &[u8]) -> Option<Self> {
        try_slice_offset(self.string.get().as_bytes(), slice).map(|range| ImString {
            offset: range,
            ..self.clone()
        })
    }

    pub fn slice_ref(&self, slice: &[u8]) -> Self {
        self.try_slice_ref(slice).unwrap()
    }

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
    /// use imstr::ImString;
    /// use std::sync::Arc;
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
    /// This works the same way as [String::lines](std::string::String::lines), except that it
    /// returns ImString instances.
    pub fn lines(&self) -> Lines<'_, S> {
        ImStringIterator::new(self.string.clone(), self.as_str().lines())
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

impl<S1: Data<String>, S2: Data<String>> PartialEq<ImString<S1>> for ImString<S2> {
    fn eq(&self, other: &ImString<S1>) -> bool {
        self.as_str().eq(other.as_str())
    }
}

impl<S: Data<String>> Eq for ImString<S> {}

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

pub type Lines<'a, S> = ImStringIterator<'a, S, std::str::Lines<'a>>;

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

impl<S: Data<String>> Deref for ImString<S> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

#[cfg(test)]
fn test_strings<S: Data<String>>() -> Vec<ImString<S>> {
    let empty = ImString::from("");
    let new = ImString::new();
    let hello = ImString::from("hello");
    let long = ImString::from("long string here");
    let sliced = long.slice(4..11);
    let world = ImString::from("world");
    let cloned = world.clone();
    let some = ImString::from("some");
    let some = some.slice(4..);
    vec![empty, new, hello, long, sliced, world, cloned, some]
}

macro_rules! tests {
    () => {};
    (#[test] fn $name:ident <S: Data<String>>() $body:tt $($rest:tt)*) => {
        #[test]
        fn $name() {
            fn $name <S: Data<String>>() $body
            $name::<Threadsafe>();
            $name::<Local>();
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
        }
        tests!{$($rest)*}
    }
}

tests! {
    #[test]
    fn test_test<S: Data<String>>() {
        for string in test_strings::<S>().into_iter() {
            assert_eq!(string.as_str(), &string.string.get()[string.offset.clone()]);
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
    fn test_insert_start<S: Data<String>>(string: ImString<S>) {
        let mut string = string;
        let length = string.len();
        string.insert(0, 'h');
        // FIXME
        //assert_eq!(string.len(), length + 1);
        //assert_eq!(string.chars().nth(0), Some('h'));
    }

    #[test]
    fn test_is_empty<S: Data<String>>(string: ImString<S>) {
        assert_eq!(string.is_empty(), string.len() == 0);
    }
}
