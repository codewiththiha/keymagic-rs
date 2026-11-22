//! Output of one key event: the host edit and the resulting composing text.

use crate::record::Trace;
use crate::text::Edit;

/// Text change the host applies to the editor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Nothing changed.
    None,
    /// Insert text at the cursor.
    Insert(String),
    /// Delete this many characters before the cursor.
    Delete(usize),
    /// Delete characters, then insert text.
    Replace {
        /// Characters to delete before the cursor.
        delete: usize,
        /// Text to insert after deleting.
        insert: String,
    },
}

/// Result of processing or peeking one key event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Output {
    /// Edit that turns the previous composing text into `composing`.
    pub action: Action,
    /// Whether the engine handled the key.
    pub consumed: bool,
    /// Composing text after the event.
    pub composing: String,
    /// Rule-application log; `Some` only with the `trace` feature.
    pub trace: Option<Trace>,
}

impl Action {
    /// Converts a buffer edit into a host action.
    #[must_use]
    pub fn from_edit(edit: &Edit) -> Self {
        let insert: String = edit.insert.iter().collect();
        match (edit.delete, insert.is_empty()) {
            (0, true) => Self::None,
            (0, false) => Self::Insert(insert),
            (delete, true) => Self::Delete(delete),
            (delete, false) => Self::Replace { delete, insert },
        }
    }
}
