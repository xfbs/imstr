pub use std::string::{FromUtf16Error, FromUtf8Error};

/// A possible error when slicing a [`ImString`](crate::ImString).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SliceError {
    StartOutOfBounds,
    EndOutOfBounds,
    EndBeforeStart,
    StartNotAligned,
    EndNotAligned,
}

#[test]
fn slice_error_debug() {
    let error = SliceError::StartOutOfBounds;
    error.clone();
    format!("{error:?}");
}
