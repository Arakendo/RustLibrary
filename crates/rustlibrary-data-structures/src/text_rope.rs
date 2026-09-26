use crate::BufferError as E;
use std::{fmt, ops::Range};

/// UTF-8 text rope with scalar-safe byte ranges and strict UTF-16 conversions.
/// Clones share Ropey storage; later edits do not change earlier clones.
#[derive(Clone)]
pub struct TextRope {
    rope: ropey::Rope,
    max_bytes: usize,
}

impl fmt::Debug for TextRope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TextRope")
            .field("len_bytes", &self.len_bytes())
            .field("max_bytes", &self.max_bytes)
            .finish()
    }
}

impl TextRope {
    pub fn new(text: &str, max_bytes: usize) -> Result<Self, E> {
        if text.len() > max_bytes {
            return Err(E::LimitExceeded);
        }
        Ok(Self {
            rope: ropey::Rope::from_str(text),
            max_bytes,
        })
    }
    pub fn len_bytes(&self) -> usize {
        self.rope.len_bytes()
    }
    pub fn is_empty(&self) -> bool {
        self.rope.len_bytes() == 0
    }
    pub fn len_utf16(&self) -> usize {
        self.rope.len_utf16_cu()
    }
    /// LF, CRLF (one break), and CR. Empty documents have one line.
    pub fn line_count(&self) -> usize {
        self.rope.len_lines()
    }
    pub fn line_start_byte(&self, line: usize) -> Result<usize, E> {
        if line >= self.line_count() {
            return Err(E::OutOfBounds);
        }
        Ok(self.rope.line_to_byte(line))
    }
    pub fn chunks(&self) -> impl Iterator<Item = &str> {
        self.rope.chunks()
    }
    pub fn to_text(&self) -> String {
        self.rope.to_string()
    }

    fn byte_to_char(&self, byte: usize) -> Result<usize, E> {
        if byte > self.len_bytes() {
            return Err(E::OutOfBounds);
        }
        let index = self.rope.byte_to_char(byte);
        if self.rope.char_to_byte(index) != byte {
            return Err(E::InvalidBoundary);
        }
        Ok(index)
    }
    pub fn byte_to_utf16(&self, byte: usize) -> Result<usize, E> {
        Ok(self.rope.char_to_utf16_cu(self.byte_to_char(byte)?))
    }
    pub fn utf16_to_byte(&self, offset: usize) -> Result<usize, E> {
        if offset > self.len_utf16() {
            return Err(E::OutOfBounds);
        }
        let index = self.rope.utf16_cu_to_char(offset);
        if self.rope.char_to_utf16_cu(index) != offset {
            return Err(E::InvalidBoundary);
        }
        Ok(self.rope.char_to_byte(index))
    }
    /// Range is zero-based UTF-8 bytes, end-exclusive, at scalar boundaries.
    /// Typed failures leave the rope unchanged. Allocator OOM is not recovered.
    pub fn replace(&mut self, range: Range<usize>, text: &str) -> Result<(), E> {
        if range.start > range.end {
            return Err(E::OutOfBounds);
        }
        let start = self.byte_to_char(range.start)?;
        let end = self.byte_to_char(range.end)?;
        let retained = self.len_bytes() - (range.end - range.start);
        let size = retained.checked_add(text.len()).ok_or(E::LimitExceeded)?;
        if size > self.max_bytes {
            return Err(E::LimitExceeded);
        }
        self.rope.remove(start..end);
        self.rope.insert(start, text);
        Ok(())
    }
}
