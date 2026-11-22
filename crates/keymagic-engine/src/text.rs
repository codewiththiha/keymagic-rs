//! Composing text stored as Unicode scalar values, with an edit journal.

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

/// Original tail an event removed, enough to reverse the event.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Journal {
    /// Leading characters the event left unchanged.
    kept: usize,
    /// Removed original characters, last one first (replayed in reverse).
    removed: SmallVec<[char; 16]>,
}

impl Journal {
    /// Starts a journal for text of `len` characters.
    fn anchored(len: usize) -> Self {
        Self {
            kept: len,
            ..Self::default()
        }
    }
}

/// Composing text held as scalar values with a UTF-8 copy.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ComposingBuffer {
    chars: Vec<char>,
    text: String,
    journal: Journal,
}

impl ComposingBuffer {
    /// Creates a buffer holding the scalar values of `text`.
    #[must_use]
    pub fn from_text(text: &str) -> Self {
        let chars: Vec<char> = text.chars().collect();
        let journal = Journal::anchored(chars.len());
        Self {
            chars,
            text: text.to_string(),
            journal,
        }
    }

    /// Returns the scalar values in order.
    #[must_use]
    pub fn chars(&self) -> &[char] {
        &self.chars
    }

    /// Returns the composing text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// Reports whether the buffer holds no scalar values.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.chars.is_empty()
    }

    /// Starts journaling a new key event from the current text.
    pub fn begin_event(&mut self) {
        self.journal = Journal::anchored(self.chars.len());
    }

    /// Returns the edit made since the last `begin_event`.
    #[must_use]
    pub fn edit(&self) -> Edit {
        let kept = self.journal.kept;
        let common = self
            .journal
            .removed
            .iter()
            .rev()
            .zip(&self.chars[kept..])
            .take_while(|(before, after)| before == after)
            .count();
        Edit {
            delete: self.journal.removed.len() - common,
            insert: self.chars[kept + common..].iter().copied().collect(),
        }
    }

    /// Returns the journal that reverses the current event.
    #[must_use]
    pub fn journal(&self) -> Journal {
        self.journal.clone()
    }

    /// Reverses the event recorded in `journal`.
    pub fn restore(&mut self, journal: &Journal) {
        self.truncate(journal.kept);
        for ch in journal.removed.iter().rev() {
            self.push(*ch);
        }
    }

    /// Appends one scalar value.
    pub fn push(&mut self, ch: char) {
        self.chars.push(ch);
        self.text.push(ch);
    }

    /// Removes the last scalar value, if any.
    pub fn pop(&mut self) {
        let keep = self.chars.len().saturating_sub(1);
        self.truncate(keep);
    }

    /// Removes `count` trailing scalar values, then appends `text`.
    pub fn replace_tail(&mut self, count: usize, text: &str) {
        let keep = self.chars.len().saturating_sub(count);
        self.truncate(keep);
        for ch in text.chars() {
            self.push(ch);
        }
    }

    fn truncate(&mut self, keep: usize) {
        if keep < self.journal.kept {
            for index in (keep..self.journal.kept).rev() {
                self.journal.removed.push(self.chars[index]);
            }
            self.journal.kept = keep;
        }
        for _ in keep..self.chars.len() {
            self.text.pop();
        }
        self.chars.truncate(keep);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edit_appends_after_common_prefix() {
        let mut buffer = ComposingBuffer::from_text("ka");
        buffer.begin_event();
        buffer.push('b');
        let edit = buffer.edit();
        assert_eq!(edit.delete, 0);
        assert_eq!(edit.insert.as_slice(), ['b'].as_slice());
    }

    #[test]
    fn edit_counts_scalar_values() {
        let mut buffer = ComposingBuffer::from_text("e\u{301}");
        buffer.begin_event();
        buffer.replace_tail(1, "Z");
        let edit = buffer.edit();
        assert_eq!(edit.delete, 1);
        assert_eq!(edit.insert.as_slice(), ['Z'].as_slice());
        assert_eq!(buffer.as_str(), "eZ");
    }

    #[test]
    fn replace_tail_removes_scalars_then_appends() {
        let mut buffer = ComposingBuffer::from_text("ကြ");
        buffer.replace_tail(1, "က");
        assert_eq!(buffer.as_str(), "ကက");
        buffer.replace_tail(9, "x");
        assert_eq!(buffer.as_str(), "x");
    }

    #[test]
    fn restore_reverses_the_journaled_event() {
        let mut buffer = ComposingBuffer::from_text("ab");
        buffer.begin_event();
        buffer.replace_tail(1, "xyz");
        let undo = buffer.journal();
        buffer.restore(&undo);
        assert_eq!(buffer.as_str(), "ab");
    }

    #[test]
    fn edits_of_up_to_inline_capacity_stay_inline() {
        let mut buffer = ComposingBuffer::default();
        buffer.begin_event();
        for ch in "abcdefghijklmnopqrstuvwxyz012345".chars() {
            buffer.push(ch);
        }
        let edit = buffer.edit();
        assert_eq!(edit.insert.len(), INLINE_EDIT_CHARS);
        assert!(!edit.insert.spilled());
    }
}
