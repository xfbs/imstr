use std::cmp::Ordering;
use std::convert::Infallible;
use std::fmt::{Display, Error as FmtError, Formatter, Write};
use std::hash::{Hash, Hasher};
use std::iter::{Extend, FromIterator};
use std::ops::Range;
use std::ops::{Add, AddAssign, Deref};
use std::str::FromStr;
pub use std::string::{FromUtf16Error, FromUtf8Error};
use std::string::{String as StdString, ToString as StdToString};
use std::sync::Arc;

/// Cheaply clonable and slicable UTF-8 string type.
///
/// It uses copy-on-write and reference counting to allow for efficient operations.
#[derive(Clone, Debug)]
pub struct String {
    /// Underlying string
    string: Arc<StdString>,
    /// Offset, must always point to valid UTF-8 region inside string.
    offset: Range<usize>,
}

impl String {
    /// Returns a byte slice of this string's contents.
    ///
    /// The inverse of this method is [from_utf8](String::from_utf8) or
    /// [from_utf8_lossy](String::from_utf8_lossy).
    pub fn as_bytes(&self) -> &[u8] {
        self.string.as_bytes()
    }

    /// Get the current capacity of the string.
    pub fn capacity(&self) -> usize {
        self.string.capacity()
    }

    /// Create a new String instance from a standard library [String](std::string::String).
    pub fn from_std_string(string: StdString) -> Self {
        String {
            offset: 0..string.as_bytes().len(),
            string: Arc::new(string),
        }
    }

    /// Convert this string into a standard library [String](std::string::String).
    pub fn into_std_string(mut self) -> StdString {
        if let Some(mut string) = Arc::get_mut(&mut self.string) {
            std::mem::take(string)
        } else {
            StdString::clone(&self.string)
        }
    }

    /// Creates a new, empty String.
    pub fn new() -> Self {
        String::from_std_string(StdString::new())
    }

    pub fn with_capacity(capacity: usize) -> Self {
        String::from_std_string(StdString::with_capacity(capacity))
    }

    /// Converts a vector of bytes to a String.
    pub fn from_utf8(vec: Vec<u8>) -> Result<String, FromUtf8Error> {
        Ok(String::from_std_string(StdString::from_utf8(vec)?))
    }

    /// Converts a slice of bytes to a string, including invalid characters.
    pub fn from_utf8_lossy(bytes: &[u8]) -> String {
        let string = StdString::from_utf8_lossy(bytes).into_owned();
        String::from_std_string(string)
    }

    /// Converts a vector of bytes to a String.
    pub unsafe fn from_utf8_unchecked(vec: Vec<u8>) -> String {
        String::from_std_string(StdString::from_utf8_unchecked(vec))
    }

    /// Extracts a string slice containing the entire string.
    pub fn as_str(&self) -> &str {
        let slice = &self.string.as_bytes()[self.offset.start..self.offset.end];
        unsafe { std::str::from_utf8_unchecked(slice) }
    }

    unsafe fn unchecked_append<F: FnOnce(StdString) -> StdString>(&mut self, f: F) {
        if let Some(mut string_ref) = Arc::get_mut(&mut self.string) {
            let string: StdString = std::mem::take(&mut string_ref);
            *string_ref = f(string);
        } else {
            let string = StdString::clone(&self.string);
            self.string = Arc::new(f(string));
        }

        self.offset.end = self.string.as_bytes().len();
    }

    pub fn truncate(&mut self, length: usize) {
        if let Some(mut string) = Arc::get_mut(&mut self.string) {
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
}

impl PartialEq<str> for String {
    fn eq(&self, other: &str) -> bool {
        self.as_str().eq(other)
    }
}

impl Eq for String {}

impl PartialEq<String> for String {
    fn eq(&self, other: &String) -> bool {
        self.as_str().eq(other.as_str())
    }
}

impl PartialOrd<String> for String {
    fn partial_cmp(&self, other: &String) -> Option<Ordering> {
        self.as_str().partial_cmp(other.as_str())
    }
}

impl Default for String {
    fn default() -> Self {
        String::new()
    }
}

impl Deref for String {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl From<&str> for String {
    fn from(string: &str) -> Self {
        String {
            string: Arc::new(StdString::from(string)),
            offset: 0..string.as_bytes().len(),
        }
    }
}

impl From<char> for String {
    fn from(c: char) -> Self {
        StdString::from(c).into()
    }
}

impl From<StdString> for String {
    fn from(string: StdString) -> Self {
        String {
            offset: 0..string.as_bytes().len(),
            string: Arc::new(string),
        }
    }
}

pub trait ToString {
    fn to_string(&self) -> String;
}

impl Display for String {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result<(), FmtError> {
        self.as_str().fmt(formatter)
    }
}

impl<T: std::string::ToString> ToString for T {
    fn to_string(&self) -> String {
        std::string::ToString::to_string(self).into()
    }
}

impl Write for String {
    fn write_str(&mut self, string: &str) -> Result<(), FmtError> {
        self.push_str(string);
        Ok(())
    }

    fn write_char(&mut self, c: char) -> Result<(), FmtError> {
        self.push(c);
        Ok(())
    }
}

impl FromStr for String {
    type Err = Infallible;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(String::from(s))
    }
}

#[test]
fn can_from_str() {
    let input = "test";
    let string = String::from_str(input).unwrap();
    assert_eq!(&string, input);
}

// Delegate hash to contained string
impl Hash for String {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        self.as_str().hash(hasher)
    }
}

impl Extend<char> for String {
    fn extend<T: IntoIterator<Item = char>>(&mut self, iter: T) {
        unsafe {
            self.unchecked_append(|mut string| {
                string.extend(iter);
                string
            });
        }
    }
}

impl<'a> Extend<&'a char> for String {
    fn extend<T: IntoIterator<Item = &'a char>>(&mut self, iter: T) {
        unsafe {
            self.unchecked_append(|mut string| {
                string.extend(iter);
                string
            });
        }
    }
}

impl<'a> Extend<&'a str> for String {
    fn extend<T: IntoIterator<Item = &'a str>>(&mut self, iter: T) {
        unsafe {
            self.unchecked_append(|mut string| {
                string.extend(iter);
                string
            });
        }
    }
}

impl FromIterator<char> for String {
    fn from_iter<T: IntoIterator<Item = char>>(iter: T) -> Self {
        let mut string = String::new();
        string.extend(iter);
        string
    }
}

impl Add<&str> for String {
    type Output = String;
    fn add(mut self, string: &str) -> Self::Output {
        self.push_str(string);
        self
    }
}

impl AddAssign<&str> for String {
    fn add_assign(&mut self, string: &str) {
        self.push_str(string);
    }
}

#[cfg(test)]
const EXAMPLE_STRINGS: &[&str] = &["", "text", "abcdef"];

#[test]
fn test_default() {
    let string = String::default();
    assert_eq!(string.string.len(), 0);
    assert_eq!(string.offset, 0..0);
}

#[test]
fn test_new() {
    let string = String::new();
    assert_eq!(string.string.len(), 0);
    assert_eq!(string.offset, 0..0);
}

#[test]
fn can_get_as_bytes() {
    for input in EXAMPLE_STRINGS.into_iter() {
        let string = String::from_std_string((*input).into());
        assert_eq!(string.as_bytes(), input.as_bytes());
    }
}

#[test]
fn can_deref() {
    for input in EXAMPLE_STRINGS.into_iter() {
        let string = String::from_std_string((*input).into());
        let string_slice: &str = &string;
        assert_eq!(&string_slice, input);
    }
}
