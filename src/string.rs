use std::borrow::{Borrow, BorrowMut, Cow};
use std::cmp::Ordering;
use std::convert::{AsMut, AsRef, Infallible};
use std::ffi::OsStr;
use std::fmt::{Debug, Display, Error as FmtError, Formatter, Write};
use std::hash::{Hash, Hasher};
use std::iter::{Extend, FromIterator};
use std::net::{SocketAddr, ToSocketAddrs};
use std::ops::{Add, AddAssign, Deref};
use std::ops::{Bound, Range, RangeBounds};
use std::path::Path;
use std::str::FromStr;
pub use std::string::{FromUtf16Error, FromUtf8Error};
use std::string::{String, ToString};
use std::sync::Arc;
use std::vec::IntoIter;

/// Cheaply clonable and slicable UTF-8 string type.
///
/// It uses copy-on-write and reference counting to allow for efficient operations.
#[derive(Clone)]
pub struct ImString {
    /// Underlying string
    string: Arc<String>,
    /// Offset, must always point to valid UTF-8 region inside string.
    offset: Range<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SliceError {
    StartOutOfBounds,
    EndOutOfBounds,
    EndBeforeStart,
    StartNotAligned,
    EndNotAligned,
}

#[test]
fn slice_error() {
    let error = SliceError::StartOutOfBounds;
    error.clone();
    format!("{error:?}");
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
        if self.offset.start != 0 {
            return self.as_str().to_string();
        }

        if let Some(mut string) = Arc::get_mut(&mut self.string) {
            if string.len() != self.offset.end {
                string.truncate(self.offset.end);
            }

            std::mem::take(string)
        } else {
            self.as_str().to_string()
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
    pub fn lines(&self) -> Lines<'_> {
        ImStringIterator {
            string: self.string.clone(),
            iterator: self.as_str().lines(),
        }
    }

    /// Create a subslice of this string.
    ///
    /// This will panic if the specified range is invalid. Use the [try_slice](ImString::try_slice)
    /// method if you want to handle invalid ranges.
    pub fn slice(&self, range: impl RangeBounds<usize>) -> ImString {
        self.try_slice(range).unwrap()
    }

    pub fn try_slice(&self, range: impl RangeBounds<usize>) -> Result<ImString, SliceError> {
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

    pub unsafe fn slice_unchecked(&self, range: impl RangeBounds<usize>) -> ImString {
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
        try_slice_offset(self.string.as_bytes(), slice).map(|range| ImString {
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
}

#[test]
fn can_try_slice() {
    let string = ImString::from("string");

    // get all
    assert_eq!(string.try_slice(..).unwrap(), "string");

    // slice from left
    assert_eq!(string.try_slice(0..).unwrap(), "string");
    assert_eq!(string.try_slice(1..).unwrap(), "tring");
    assert_eq!(string.try_slice(2..).unwrap(), "ring");
    assert_eq!(string.try_slice(3..).unwrap(), "ing");
    assert_eq!(string.try_slice(4..).unwrap(), "ng");
    assert_eq!(string.try_slice(5..).unwrap(), "g");
    assert_eq!(string.try_slice(6..).unwrap(), "");

    // slice from right
    assert_eq!(string.try_slice(..6).unwrap(), "string");
    assert_eq!(string.try_slice(..5).unwrap(), "strin");
    assert_eq!(string.try_slice(..4).unwrap(), "stri");
    assert_eq!(string.try_slice(..3).unwrap(), "str");
    assert_eq!(string.try_slice(..2).unwrap(), "st");
    assert_eq!(string.try_slice(..1).unwrap(), "s");
    assert_eq!(string.try_slice(..0).unwrap(), "");

    // subslice
    let string = string.try_slice(1..5).unwrap();
    assert_eq!(string, "trin");
    assert_eq!(string.try_slice(..).unwrap(), "trin");

    // subslice from left
    assert_eq!(string.try_slice(0..).unwrap(), "trin");
    assert_eq!(string.try_slice(1..).unwrap(), "rin");
    assert_eq!(string.try_slice(2..).unwrap(), "in");
    assert_eq!(string.try_slice(3..).unwrap(), "n");
    assert_eq!(string.try_slice(4..).unwrap(), "");

    // subslice from right
    assert_eq!(string.try_slice(..4).unwrap(), "trin");
    assert_eq!(string.try_slice(..3).unwrap(), "tri");
    assert_eq!(string.try_slice(..2).unwrap(), "tr");
    assert_eq!(string.try_slice(..1).unwrap(), "t");
    assert_eq!(string.try_slice(..0).unwrap(), "");

    assert_eq!(string.try_slice(1..7), Err(SliceError::EndOutOfBounds));
    assert_eq!(string.try_slice(5..7), Err(SliceError::StartOutOfBounds));
    assert_eq!(string.try_slice(3..1), Err(SliceError::EndBeforeStart));

    // a umlaut, o umlaut, u umlaut.
    let string = ImString::from("\u{61}\u{308}\u{6f}\u{308}\u{75}\u{308}");

    assert_eq!(string, string);
    assert_eq!(string.try_slice(..).unwrap(), string);
    assert_eq!(string.try_slice(0..1).unwrap(), &string.as_str()[0..1]);
    assert_eq!(string.try_slice(0..2), Err(SliceError::EndNotAligned));
    assert_eq!(string.try_slice(0..3).unwrap(), &string.as_str()[0..3]);
    assert_eq!(string.try_slice(0..4).unwrap(), &string.as_str()[0..4]);
    assert_eq!(string.try_slice(0..5), Err(SliceError::EndNotAligned));
    assert_eq!(string.try_slice(0..6).unwrap(), &string.as_str()[0..6]);
    assert_eq!(string.try_slice(0..7).unwrap(), &string.as_str()[0..7]);
    assert_eq!(string.try_slice(0..8), Err(SliceError::EndNotAligned));
    assert_eq!(string.try_slice(0..9).unwrap(), &string.as_str()[0..9]);
}

#[test]
fn can_slice_ref() {
    let string = ImString::from("string");
    let slice = string.slice(5..);

    // cannot get slice of non-existing
    assert_eq!(string.try_str_ref("x"), None);
    assert_eq!(slice.try_str_ref("x"), None);

    assert_eq!(
        string.try_str_ref(&string.as_str()).unwrap(),
        string.as_str()
    );
    assert_eq!(
        string.try_str_ref(&string.as_str()[1..]).unwrap(),
        string.as_str()[1..]
    );
    assert_eq!(
        string.try_str_ref(&string.as_str()[2..]).unwrap(),
        string.as_str()[2..]
    );
    assert_eq!(
        string.try_str_ref(&string.as_str()[3..]).unwrap(),
        string.as_str()[3..]
    );
    assert_eq!(
        string.try_str_ref(&string.as_str()[4..]).unwrap(),
        string.as_str()[4..]
    );
    assert_eq!(
        string.try_str_ref(&string.as_str()[5..]).unwrap(),
        string.as_str()[5..]
    );
    assert_eq!(
        string.try_str_ref(&string.as_str()[6..]).unwrap(),
        string.as_str()[6..]
    );
}

#[test]
fn can_into_std_string() {
    let string = ImString::from("long string");
    assert_eq!(string.into_std_string(), "long string");

    let string = ImString::from("long string");
    let string = string.slice(5..);
    assert_eq!(string.into_std_string(), "string");

    let original = ImString::from("long string");
    let string = original.slice(5..);
    drop(original);
    assert_eq!(string.into_std_string(), "string");
}

pub type Lines<'a> = ImStringIterator<'a, std::str::Lines<'a>>;
//pub type Split<'a> = ImStringIterator<'a, std::str::Split<'a>>;

impl PartialEq<str> for ImString {
    fn eq(&self, other: &str) -> bool {
        self.as_str().eq(other)
    }
}

impl<'a> PartialEq<&'a str> for ImString {
    fn eq(&self, other: &&'a str) -> bool {
        self.as_str().eq(*other)
    }
}

impl PartialEq<String> for ImString {
    fn eq(&self, other: &String) -> bool {
        self.as_str().eq(other.as_str())
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

impl Ord for ImString {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_str().cmp(other.as_str())
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

impl<'a> From<Cow<'a, str>> for ImString {
    fn from(string: Cow<'a, str>) -> ImString {
        ImString::from(string.into_owned())
    }
}

pub trait ToImString {
    fn to_im_string(&self) -> ImString;
}

impl Display for ImString {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result<(), FmtError> {
        Display::fmt(self.as_str(), formatter)
    }
}

impl Debug for ImString {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        Debug::fmt(self.as_str(), f)
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

impl ToImString for str {
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

impl<'a> FromIterator<&'a char> for ImString {
    fn from_iter<T: IntoIterator<Item = &'a char>>(iter: T) -> Self {
        let mut string = ImString::new();
        string.extend(iter);
        string
    }
}

impl<'a> FromIterator<&'a str> for ImString {
    fn from_iter<T: IntoIterator<Item = &'a str>>(iter: T) -> Self {
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

pub struct ImStringIterator<'a, I: Iterator<Item = &'a str>> {
    string: Arc<String>,
    iterator: I,
}

impl<'a, I: Iterator<Item = &'a str>> Iterator for ImStringIterator<'a, I> {
    type Item = ImString;
    fn next(&mut self) -> Option<Self::Item> {
        match self.iterator.next() {
            Some(slice) => {
                let offset = try_slice_offset(self.string.as_bytes(), slice.as_bytes()).unwrap();
                Some(ImString {
                    string: self.string.clone(),
                    offset,
                })
            },
            None => None,
        }
    }
}

impl AsRef<str> for ImString {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl AsRef<Path> for ImString {
    fn as_ref(&self) -> &Path {
        self.as_str().as_ref()
    }
}

impl AsRef<OsStr> for ImString {
    fn as_ref(&self) -> &OsStr {
        self.as_str().as_ref()
    }
}

impl AsRef<[u8]> for ImString {
    fn as_ref(&self) -> &[u8] {
        self.as_str().as_ref()
    }
}

impl AsRef<String> for ImString {
    fn as_ref(&self) -> &String {
        self.string.as_ref()
    }
}

impl Borrow<str> for ImString {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl ToSocketAddrs for ImString {
    type Iter = <String as ToSocketAddrs>::Iter;
    fn to_socket_addrs(&self) -> std::io::Result<<String as ToSocketAddrs>::Iter> {
        self.string.to_socket_addrs()
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
