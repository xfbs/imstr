use std::cmp::Ordering;
use std::convert::Infallible;
use std::fmt::{Display, Error as FmtError, Formatter, Write};
use std::hash::{Hash, Hasher};
use std::iter::{Extend, FromIterator};
use std::ops::Range;
use std::ops::{Add, AddAssign, Deref};
use std::str::FromStr;
pub use std::string::{FromUtf16Error, FromUtf8Error};
use std::string::{String, ToString as StdToString};
use std::sync::Arc;

/// Cheaply clonable and slicable UTF-8 string type.
///
/// It uses copy-on-write and reference counting to allow for efficient operations.
#[derive(Clone, Debug)]
pub struct ImString {
    /// Underlying string
    string: Arc<String>,
    /// Offset, must always point to valid UTF-8 region inside string.
    offset: Range<usize>,
}

impl ImString {
    /// Returns a byte slice of this string's contents.
    ///
    /// The inverse of this method is [from_utf8](ImString::from_utf8) or
    /// [from_utf8_lossy](ImString::from_utf8_lossy).
    pub fn as_bytes(&self) -> &[u8] {
        self.string.as_bytes()
    }

    /// Get the current capacity of the string.
    pub fn capacity(&self) -> usize {
        self.string.capacity()
    }

    /// Create a new ImString instance from a standard library [String](std::string::String).
    pub fn from_std_string(string: String) -> Self {
        ImString {
            offset: 0..string.as_bytes().len(),
            string: Arc::new(string),
        }
    }

    /// Truncates this string, removing all contents.
    ///
    /// If this is the only reference to the string, it will clear the backing
    /// [String](std::string::String). If it is not, it only sets the offset to an empty slice.
    pub fn clear(&mut self) {
        unsafe {
            self.try_modify_unchecked(|string| string.clear());
        }
        self.offset = 0..0;
    }

    /// Returns the length of the string in bytes.
    ///
    /// This will not return the length in bytes or graphemes.
    pub fn len(&self) -> usize {
        self.offset.len()
    }

    /// Convert this string into a standard library [String](std::string::String).
    pub fn into_std_string(mut self) -> String {
        if let Some(mut string) = Arc::get_mut(&mut self.string) {
            std::mem::take(string)
        } else {
            String::clone(&self.string)
        }
    }

    /// Creates a new, empty ImString.
    pub fn new() -> Self {
        ImString::from_std_string(String::new())
    }

    pub fn with_capacity(capacity: usize) -> Self {
        ImString::from_std_string(String::with_capacity(capacity))
    }

    /// Converts a vector of bytes to a ImString.
    pub fn from_utf8(vec: Vec<u8>) -> Result<ImString, FromUtf8Error> {
        Ok(ImString::from_std_string(String::from_utf8(vec)?))
    }

    /// Converts a slice of bytes to a string, including invalid characters.
    pub fn from_utf8_lossy(bytes: &[u8]) -> ImString {
        let string = String::from_utf8_lossy(bytes).into_owned();
        ImString::from_std_string(string)
    }

    /// Converts a vector of bytes to a ImString.
    pub unsafe fn from_utf8_unchecked(vec: Vec<u8>) -> ImString {
        ImString::from_std_string(String::from_utf8_unchecked(vec))
    }

    /// Extracts a string slice containing the entire string.
    pub fn as_str(&self) -> &str {
        let slice = &self.string.as_bytes()[self.offset.start..self.offset.end];
        unsafe { std::str::from_utf8_unchecked(slice) }
    }

    unsafe fn try_modify_unchecked<F: FnOnce(&mut String)>(&mut self, f: F) -> bool {
        if let Some(mut string) = Arc::get_mut(&mut self.string) {
            f(string);
            true
        } else {
            false
        }
    }

    unsafe fn unchecked_append<F: FnOnce(String) -> String>(&mut self, f: F) {
        if let Some(mut string_ref) = Arc::get_mut(&mut self.string) {
            let string: String = std::mem::take(&mut string_ref);
            *string_ref = f(string);
        } else {
            let string = String::clone(&self.string);
            self.string = Arc::new(f(string));
        }

        self.offset.end = self.string.as_bytes().len();
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

    pub fn insert_str(&mut self, index: usize, s: &str) {
        unsafe {
            self.unchecked_append(|mut string| {
                string.insert_str(index, s);
                string
            });
        }
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

    pub fn is_empty(&self) -> bool {
        self.offset.is_empty()
    }
}

impl PartialEq<str> for ImString {
    fn eq(&self, other: &str) -> bool {
        self.as_str().eq(other)
    }
}

impl Eq for ImString {}

impl PartialEq<ImString> for ImString {
    fn eq(&self, other: &ImString) -> bool {
        self.as_str().eq(other.as_str())
    }
}

impl PartialOrd<ImString> for ImString {
    fn partial_cmp(&self, other: &ImString) -> Option<Ordering> {
        self.as_str().partial_cmp(other.as_str())
    }
}

impl Default for ImString {
    fn default() -> Self {
        ImString::new()
    }
}

impl Deref for ImString {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl From<&str> for ImString {
    fn from(string: &str) -> Self {
        ImString {
            string: Arc::new(String::from(string)),
            offset: 0..string.as_bytes().len(),
        }
    }
}

impl From<char> for ImString {
    fn from(c: char) -> Self {
        String::from(c).into()
    }
}

impl From<String> for ImString {
    fn from(string: String) -> Self {
        ImString {
            offset: 0..string.as_bytes().len(),
            string: Arc::new(string),
        }
    }
}

impl From<ImString> for String {
    fn from(string: ImString) -> Self {
        string.into_std_string()
    }
}

pub trait ToImString {
    fn to_im_string(&self) -> ImString;
}

impl Display for ImString {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result<(), FmtError> {
        self.as_str().fmt(formatter)
    }
}

impl ToImString for ImString {
    fn to_im_string(&self) -> ImString {
        self.clone()
    }
}

impl ToImString for String {
    fn to_im_string(&self) -> ImString {
        self.clone().into()
    }
}

impl ToImString for &str {
    fn to_im_string(&self) -> ImString {
        self.to_string().into()
    }
}

impl Write for ImString {
    fn write_str(&mut self, string: &str) -> Result<(), FmtError> {
        self.push_str(string);
        Ok(())
    }

    fn write_char(&mut self, c: char) -> Result<(), FmtError> {
        self.push(c);
        Ok(())
    }
}

impl FromStr for ImString {
    type Err = Infallible;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(ImString::from(s))
    }
}

#[test]
fn can_from_str() {
    let input = "test";
    let string = ImString::from_str(input).unwrap();
    assert_eq!(&string, input);
}

// Delegate hash to contained string
impl Hash for ImString {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        self.as_str().hash(hasher)
    }
}

impl Extend<char> for ImString {
    fn extend<T: IntoIterator<Item = char>>(&mut self, iter: T) {
        unsafe {
            self.unchecked_append(|mut string| {
                string.extend(iter);
                string
            });
        }
    }
}

impl<'a> Extend<&'a char> for ImString {
    fn extend<T: IntoIterator<Item = &'a char>>(&mut self, iter: T) {
        unsafe {
            self.unchecked_append(|mut string| {
                string.extend(iter);
                string
            });
        }
    }
}

impl<'a> Extend<&'a str> for ImString {
    fn extend<T: IntoIterator<Item = &'a str>>(&mut self, iter: T) {
        unsafe {
            self.unchecked_append(|mut string| {
                string.extend(iter);
                string
            });
        }
    }
}

impl FromIterator<char> for ImString {
    fn from_iter<T: IntoIterator<Item = char>>(iter: T) -> Self {
        let mut string = ImString::new();
        string.extend(iter);
        string
    }
}

impl Add<&str> for ImString {
    type Output = ImString;
    fn add(mut self, string: &str) -> Self::Output {
        self.push_str(string);
        self
    }
}

impl AddAssign<&str> for ImString {
    fn add_assign(&mut self, string: &str) {
        self.push_str(string);
    }
}

#[cfg(test)]
const EXAMPLE_STRINGS: &[&str] = &["", "text", "abcdef"];

#[test]
fn test_default() {
    let string = ImString::default();
    assert_eq!(string.string.len(), 0);
    assert_eq!(string.offset, 0..0);
}

#[test]
fn test_new() {
    let string = ImString::new();
    assert_eq!(string.string.len(), 0);
    assert_eq!(string.offset, 0..0);
}

#[test]
fn can_get_as_bytes() {
    for input in EXAMPLE_STRINGS.into_iter() {
        let string = ImString::from_std_string((*input).into());
        assert_eq!(string.as_bytes(), input.as_bytes());
    }
}

#[test]
fn can_deref() {
    for input in EXAMPLE_STRINGS.into_iter() {
        let string = ImString::from_std_string((*input).into());
        let string_slice: &str = &string;
        assert_eq!(&string_slice, input);
    }
}
