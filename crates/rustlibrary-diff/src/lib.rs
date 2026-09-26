//! Exact, bounded line comparison. No filesystem, patch application or rendering.
use std::{fmt, ops::Range};
mod compare;
pub use compare::diff_lines;

/// Combined input/line limits, DP table cells, conservative work units and output
/// changes. Zero is a finite limit, never unlimited. These do not bound total heap.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub input_bytes: usize,
    pub lines: usize,
    pub cells: usize,
    pub work: usize,
    pub changes: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            input_bytes: 1024 * 1024,
            lines: 4096,
            cells: 1_000_000,
            work: 16_000_000,
            changes: 4096,
        }
    }
}

/// One contiguous changed region. Ranges are zero-based and end-exclusive.
/// An empty old range is insertion; an empty new range is deletion. Both nonempty
/// means replacement. Byte ranges include original CR/LF terminators unchanged.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Change {
    pub old_lines: Range<usize>,
    pub new_lines: Range<usize>,
    pub old_bytes: Range<usize>,
    pub new_bytes: Range<usize>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiffError {
    InputLimit,
    LineLimit,
    CellLimit,
    WorkLimit,
    ChangeLimit,
    AllocationFailed,
}
impl fmt::Display for DiffError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InputLimit => "diff input byte budget exceeded",
            Self::LineLimit => "diff line budget exceeded",
            Self::CellLimit => "diff table cell budget exceeded",
            Self::WorkLimit => "diff work budget exceeded",
            Self::ChangeLimit => "diff change budget exceeded",
            Self::AllocationFailed => "diff allocation failed",
        })
    }
}
impl std::error::Error for DiffError {}
