#[cfg(feature = "std")]
pub use std::string::{FromUtf16Error, FromUtf8Error};

#[cfg(not(feature = "std"))]
pub use {
    alloc::format,
    alloc::string::{FromUtf16Error, FromUtf8Error},
};

/// A possible error when slicing a [`ImString`](crate::ImString).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SliceError {
    StartOutOfBounds,
    EndOutOfBounds,
    EndBeforeStart,
    StartNotAligned,
    EndNotAligned,
}

impl std::fmt::Display for SliceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StartOutOfBounds => write!(f, "start offset out of bounds"),
            Self::StartNotAligned => write!(f, "start offset in multibyte UTF-8 sequence"),
            Self::EndOutOfBounds => write!(f, "end offset out of bounds"),
            Self::EndNotAligned => write!(f, "end offset in multibyte UTF-8 sequence"),
            Self::EndBeforeStart => write!(f, "end offset before start offset"),
        }
    }
}

#[test]
fn slice_error_traits() {
    let error = SliceError::StartOutOfBounds;
    // implements clone
    let new = error.clone();
    // implements partial eq
    assert_eq!(error, new);
    // implements debug
    format!("{error:?}");
    // implements display
    format!("{new}");
}
