use crate::BufferError as E;
use std::fmt;

/// A generic element gap buffer. Offsets count T elements, not text units.
/// Safe slots own each value once; no Clone, Copy or Default bound is required.
pub struct GapBuffer<T> {
    slots: Vec<Option<T>>,
    start: usize,
    end: usize,
    max_elements: usize,
}

impl<T> fmt::Debug for GapBuffer<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GapBuffer")
            .field("len", &self.len())
            .field("gap_position", &self.start)
            .field("max_elements", &self.max_elements)
            .finish()
    }
}

impl<T> GapBuffer<T> {
    pub fn new(max_elements: usize) -> Self {
        Self {
            slots: Vec::new(),
            start: 0,
            end: 0,
            max_elements,
        }
    }
    pub fn len(&self) -> usize {
        self.slots.len() - (self.end - self.start)
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub fn gap_position(&self) -> usize {
        self.start
    }
    pub fn get(&self, index: usize) -> Option<&T> {
        if index >= self.len() {
            return None;
        }
        self.slots[if index < self.start {
            index
        } else {
            index + self.end - self.start
        }]
        .as_ref()
    }
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.slots[..self.start]
            .iter()
            .chain(self.slots[self.end..].iter())
            .filter_map(Option::as_ref)
    }
    pub fn move_gap(&mut self, index: usize) -> Result<(), E> {
        if index > self.len() {
            return Err(E::OutOfBounds);
        }
        while self.start > index {
            self.start -= 1;
            self.end -= 1;
            self.slots.swap(self.start, self.end);
        }
        while self.start < index {
            self.slots.swap(self.start, self.end);
            self.start += 1;
            self.end += 1;
        }
        Ok(())
    }
    /// Inserts at the cursor. On failure the supplied value is returned to caller.
    pub fn insert(&mut self, value: T) -> Result<(), (E, T)> {
        if self.len() == self.max_elements {
            return Err((E::LimitExceeded, value));
        }
        if self.start == self.end {
            let old_len = self.slots.len();
            let target = old_len.saturating_mul(2).max(16).min(self.max_elements);
            if self.slots.try_reserve_exact(target - old_len).is_err() {
                return Err((E::AllocationFailed, value));
            }
            self.slots.resize_with(target, || None);
            let gap = target - old_len;
            for index in (self.start..old_len).rev() {
                self.slots.swap(index, index + gap);
            }
            self.end += gap;
        }
        self.slots[self.start] = Some(value);
        self.start += 1;
        Ok(())
    }
    pub fn delete_before(&mut self) -> Option<T> {
        if self.start == 0 {
            return None;
        }
        self.start -= 1;
        self.slots[self.start].take()
    }
    pub fn delete_after(&mut self) -> Option<T> {
        if self.end == self.slots.len() {
            return None;
        }
        let value = self.slots[self.end].take();
        self.end += 1;
        value
    }
}
