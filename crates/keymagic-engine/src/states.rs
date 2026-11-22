//! Transient state sets with event-scoped activation.

use keymagic_types::StateId;
use smallvec::SmallVec;

/// Small set of state identifiers, searched linearly.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StateSet(SmallVec<[StateId; 8]>);

impl StateSet {
    /// Reports whether `id` is a member.
    #[must_use]
    pub fn contains(&self, id: StateId) -> bool {
        self.0.contains(&id)
    }

    /// Adds `id` unless it is already a member.
    pub fn insert(&mut self, id: StateId) {
        if !self.contains(id) {
            self.0.push(id);
        }
    }

    /// Removes every member.
    pub fn clear(&mut self) {
        self.0.clear();
    }
}

/// States visible to the current event and states activated during it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct States {
    /// States that matching checks during the event.
    pub active: StateSet,
    /// States activated by rules applied during the event.
    pub next: StateSet,
}

impl States {
    /// Makes this event's activations visible to the next event.
    pub fn finish_event(&mut self) {
        std::mem::swap(&mut self.active, &mut self.next);
        self.next.clear();
    }

    /// Clears both sets.
    pub fn clear(&mut self) {
        self.active.clear();
        self.next.clear();
    }
}
