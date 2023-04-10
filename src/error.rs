//! Error types
use alloc::fmt::{Display, Formatter, Result};
pub use alloc::string::{FromUtf16Error, FromUtf8Error};

/// A possible error when slicing a [`ImString`](crate::ImString).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SliceError {
    /// Start offset out of bounds.
    StartOutOfBounds,
    /// End offset out of bounds.
    EndOutOfBounds,
    /// End index smaller than start index.
    EndBeforeStart,
    /// Start index not on [`char`] boundary.
    StartNotAligned,
    /// End index not on [`char`] boundary.
    EndNotAligned,
}

impl Display for SliceError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
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
    use SliceError::*;
    let errors = [
        StartOutOfBounds,
        EndOutOfBounds,
        EndBeforeStart,
        StartNotAligned,
        EndNotAligned,
    ];

    for error in errors.into_iter() {
        // implements clone
        let new = error.clone();
        // implements partial eq
        assert_eq!(error, new);
        // implements debug
        alloc::format!("{error:?}");
        // implements display
        alloc::format!("{new}");
    }
}
