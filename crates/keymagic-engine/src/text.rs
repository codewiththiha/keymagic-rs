//! Composing text stored as Unicode scalar values.

use smallvec::SmallVec;

/// Inline capacity for the characters an edit inserts.
pub const INLINE_EDIT_CHARS: usize = 32;

/// Removes `delete` trailing characters, then appends `insert`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Edit {
    /// Characters removed from the end of the previous text.
    pub delete: usize,
    /// Characters appended after the common prefix.
    pub insert: SmallVec<[char; INLINE_EDIT_CHARS]>,
}

/// Composing text held as a vector of scalar values.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ComposingBuffer(Vec<char>);

impl ComposingBuffer {
    /// Creates a buffer holding the scalar values of `text`.
    #[must_use]
    pub fn from_text(text: &str) -> Self {
        Self(text.chars().collect())
    }

    /// Returns the scalar values in order.
    #[must_use]
    pub fn chars(&self) -> &[char] {
        &self.0
    }

    /// Returns the composing text as a string.
    #[must_use]
    pub fn to_text(&self) -> String {
        self.0.iter().collect()
    }

    /// Reports whether the buffer holds no scalar values.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Appends one scalar value.
    pub fn push(&mut self, ch: char) {
        self.0.push(ch);
    }

    /// Removes the last scalar value, if any.
    pub fn pop(&mut self) {
        self.0.pop();
    }

    /// Removes `count` trailing scalar values, then appends `text`.
    pub fn replace_tail(&mut self, count: usize, text: &str) {
        let keep = self.0.len().saturating_sub(count);
        self.0.truncate(keep);
        self.0.extend(text.chars());
    }

    /// Returns the edit that turns `prev` into this buffer.
    #[must_use]
    pub fn diff(&self, prev: &Self) -> Edit {
        let common = prev
            .0
            .iter()
            .zip(&self.0)
            .take_while(|(before, after)| before == after)
            .count();
        Edit {
            delete: prev.0.len() - common,
            insert: self.0[common..].iter().copied().collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_appends_after_common_prefix() {
        let prev = ComposingBuffer::from_text("ka");
        let next = ComposingBuffer::from_text("kab");
        let edit = next.diff(&prev);
        assert_eq!(edit.delete, 0);
        assert_eq!(edit.insert.as_slice(), ['b'].as_slice());
    }

    #[test]
    fn diff_counts_scalar_values() {
        let prev = ComposingBuffer::from_text("e\u{301}");
        let next = ComposingBuffer::from_text("eZ");
        let edit = next.diff(&prev);
        assert_eq!(edit.delete, 1);
        assert_eq!(edit.insert.as_slice(), ['Z'].as_slice());
    }

    #[test]
    fn replace_tail_removes_scalars_then_appends() {
        let mut buffer = ComposingBuffer::from_text("ကြ");
        buffer.replace_tail(1, "က");
        assert_eq!(buffer.to_text(), "ကက");
        buffer.replace_tail(9, "x");
        assert_eq!(buffer.to_text(), "x");
    }

    #[test]
    fn edits_of_up_to_inline_capacity_stay_inline() {
        let prev = ComposingBuffer::default();
        let next = ComposingBuffer::from_text("abcdefghijklmnopqrstuvwxyz012345");
        let edit = next.diff(&prev);
        assert_eq!(edit.insert.len(), INLINE_EDIT_CHARS);
        assert!(!edit.insert.spilled());
    }
}
