//! Wire helpers shared by the input-method-v1 and v2 drivers.

use std::collections::HashSet;

/// Key codes sent to the application, so their releases can follow.
#[derive(Default)]
pub struct Passed(HashSet<u32>);

impl Passed {
    /// Records a press that was sent to the application.
    pub fn press(&mut self, code: u32) {
        self.0.insert(code);
    }

    /// True when this release should be sent; forgets the press.
    #[must_use]
    pub fn release(&mut self, code: u32) -> bool {
        self.0.remove(&code)
    }

    /// Forgets all pending presses, such as on deactivation.
    pub fn clear(&mut self) {
        self.0.clear();
    }
}

/// Index for `delete_surrounding_text`: `bytes` before the caret.
#[must_use]
pub fn v1_delete_index(bytes: u32) -> i32 {
    0_i32.saturating_sub(i32::try_from(bytes).unwrap_or(i32::MAX))
}

#[cfg(test)]
mod tests {
    use super::{Passed, v1_delete_index};

    #[test]
    fn releases_follow_only_forwarded_presses() {
        let mut passed = Passed::default();
        assert!(!passed.release(30));
        passed.press(30);
        assert!(passed.release(30));
        assert!(!passed.release(30));
    }

    #[test]
    fn clear_forgets_pending_presses() {
        let mut passed = Passed::default();
        passed.press(30);
        passed.clear();
        assert!(!passed.release(30));
    }

    #[test]
    fn delete_index_points_back_by_bytes() {
        assert_eq!(v1_delete_index(3), -3);
        assert_eq!(v1_delete_index(0), 0);
        assert_eq!(v1_delete_index(u32::MAX), -i32::MAX);
    }
}
