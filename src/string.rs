use std::sync::Arc;
use std::ops::Range;
use std::ops::Deref;
use std::cmp::Ordering;
use std::string::{String as StdString};
pub use std::string::{FromUtf16Error, FromUtf8Error};

#[derive(Clone, Debug)]
pub struct String {
    /// Underlying string
    string: Arc<StdString>,
    /// Offset, must always point to valid UTF-8 region inside string.
    offset: Range<usize>,
}

impl String {
    fn as_bytes(&self) -> &[u8] {
        self.string.as_bytes()
    }

    fn capacity(&self) -> usize {
        self.string.capacity()
    }

    fn from_std_string(string: StdString) -> Self {
        String {
            offset: 0..string.as_bytes().len(),
            string: Arc::new(string),
        }
    }

    fn into_std_string(self) -> StdString {
        self.as_str().to_string()
    }

    pub fn new() -> Self {
        String::from_std_string(StdString::new())
    }

    pub fn from_utf8(vec: Vec<u8>) -> Result<String, FromUtf8Error> {
        Ok(String::from_std_string(StdString::from_utf8(vec)?))
    }

    pub fn as_str(&self) -> &str {
        let slice = &self.string.as_bytes()[self.offset.start..self.offset.end];
        unsafe { std::str::from_utf8_unchecked(slice) }
    }

    unsafe fn unsafe_modify<F: FnOnce(StdString) -> StdString>(&mut self, f: F) {
        if let Some(mut string_ref) = Arc::get_mut(&mut self.string) {
            let string: StdString = std::mem::take(&mut string_ref);
            *string_ref = f(string);
        } else {
        }
    }

    pub fn push(&mut self, c: char) {
        unsafe {
            self.unsafe_modify(|mut string| {
                string.push(c);
                string
            });
        }
        self.offset.end = self.string.as_bytes().len();
    }

    pub fn push_str(&mut self, slice: &str) {
        unsafe {
            self.unsafe_modify(|mut string| {
                string.push_str(slice);
                string
            });
        }
        self.offset.end = self.string.as_bytes().len();
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

impl ToString for String {
    fn to_string(&self) -> String {
        self.clone()
    }
}

pub trait AnyString {
}

impl AnyString for String {}

impl AnyString for std::string::String {}

const EXAMPLE_STRINGS: &[&str] = &[
    "",
    "text",
    "abcdef",
];

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
