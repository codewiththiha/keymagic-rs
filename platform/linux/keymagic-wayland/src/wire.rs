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
