//! Double-buffered input-method state, applied on each `done` event.

/// Surrounding text from the last `surrounding_text` event.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Surrounding {
    /// Text around the cursor.
    pub text: String,
    /// Cursor position in bytes.
    pub cursor: u32,
    /// Anchor position in bytes.
    pub anchor: u32,
}

/// One buffer of input-method state.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Buffer {
    active: bool,
    surrounding: Option<Surrounding>,
}

/// Stages events until `done`, and counts the `done` events sent.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Tracker {
    pending: Buffer,
    current: Buffer,
    dones: u32,
}

impl Tracker {
    /// Stages an activation and drops staged surrounding text.
    pub fn activate(&mut self) {
        self.pending = Buffer { active: true, surrounding: None };
    }

    /// Stages a deactivation.
    pub fn deactivate(&mut self) {
        self.pending.active = false;
    }

    /// Stages the surrounding text.
    pub fn surrounding(&mut self, text: Surrounding) {
        self.pending.surrounding = Some(text);
    }

    /// Applies the staged state and counts this `done` event.
    pub fn done(&mut self) {
        self.current.clone_from(&self.pending);
        self.dones += 1;
    }

    /// Whether a commit with `serial` may change the current state.
    #[must_use]
    pub const fn accepts_serial(&self, serial: u32) -> bool {
        serial == self.dones
    }

    /// Whether the applied state is active.
    #[must_use]
    pub const fn is_active(&self) -> bool {
        self.current.active
    }

    /// Surrounding text in the applied state.
    #[must_use]
    pub const fn current_surrounding(&self) -> Option<&Surrounding> {
        self.current.surrounding.as_ref()
    }

    /// Number of `done` events received.
    #[must_use]
    pub const fn dones(&self) -> u32 {
        self.dones
    }
}

#[cfg(test)]
mod tests {
    use super::{Surrounding, Tracker};

    fn text(value: &str) -> Surrounding {
        Surrounding { text: value.to_owned(), cursor: 1, anchor: 1 }
    }

    #[test]
    fn changes_stay_staged_until_done() {
        let mut tracker = Tracker::default();
        tracker.activate();
        assert!(!tracker.is_active());
        tracker.done();
        assert!(tracker.is_active());
        assert_eq!(tracker.dones(), 1);
    }

    #[test]
    fn activate_drops_staged_surrounding_text() {
        let mut tracker = Tracker::default();
        tracker.surrounding(text("ab"));
        tracker.activate();
        tracker.done();
        assert_eq!(tracker.current_surrounding(), None);
    }

    #[test]
    fn deactivate_clears_the_active_state_on_done() {
        let mut tracker = Tracker::default();
        tracker.activate();
        tracker.done();
        tracker.deactivate();
        assert!(tracker.is_active());
        tracker.done();
        assert!(!tracker.is_active());
    }

    #[test]
    fn commit_serial_must_match_the_done_count() {
        let mut tracker = Tracker::default();
        assert!(tracker.accepts_serial(0));
        tracker.done();
        assert!(tracker.accepts_serial(1));
        assert!(!tracker.accepts_serial(0));
    }
}
