//! Reusable collections. Element, byte and UTF-16 offsets are explicitly distinct.
#![doc = include_str!("../README.md")]
use std::fmt;
mod gap_buffer;
mod text_rope;
pub use gap_buffer::GapBuffer;
pub use text_rope::TextRope;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BufferError {
    OutOfBounds,
    InvalidBoundary,
    LimitExceeded,
    AllocationFailed,
}
impl fmt::Display for BufferError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::OutOfBounds => "buffer position or range is out of bounds",
            Self::InvalidBoundary => "position splits a Unicode scalar",
            Self::LimitExceeded => "buffer content exceeds its admitted limit",
            Self::AllocationFailed => "buffer allocation failed",
        })
    }
}
impl std::error::Error for BufferError {}
