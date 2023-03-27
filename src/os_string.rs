use std::sync::Arc;
use std::ops::Range;

pub struct OsString {
    string: Arc<OsString>,
    offset: Range<usize>,
}

pub trait AnyOsString {
}

impl AnyOsString for OsString {}

