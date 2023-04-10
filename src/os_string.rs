use alloc::sync::Arc;
use core::ops::Range;

pub struct OsString {
    string: Arc<OsString>,
    offset: Range<usize>,
}

pub trait AnyOsString {
}

impl AnyOsString for OsString {}

