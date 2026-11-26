#[cfg(test)]
mod tests {
    use super::{Step, consumes, delete_bytes, plan};
    use keymagic_host::session::HostAction;

    #[test]
    fn pass_and_hud_do_not_consume_the_key() {
        assert!(!consumes(&[HostAction::Pass]));
        assert!(!consumes(&[HostAction::Hud("Unicode".to_owned())]));
        assert!(consumes(&[HostAction::Eat]));
        assert!(consumes(&[HostAction::Commit("ခ".to_owned())]));
    }

    #[test]
    fn commit_and_preedit_steps_track_the_tail() {
        let mut tail = String::new();
        let steps = plan(&[HostAction::Commit("ခ".to_owned())], &mut tail);
        assert_eq!(steps, vec![Step::Commit("ခ".to_owned())]);
        assert_eq!(tail, "ခ");
        let steps = plan(&[HostAction::UpdatePreedit("က".to_owned())], &mut tail);
        assert_eq!(steps, vec![Step::Preedit("က".to_owned())]);
        assert_eq!(tail, "ခ");
    }

    #[test]
    fn replace_deletes_the_bytes_of_the_last_chars() {
        let mut tail = "ခa".to_owned();
        let action = HostAction::ReplaceCommitted {
            delete: 1,
            insert: "က".to_owned(),
        };
        let steps = plan(&[action], &mut tail);
        assert_eq!(
            steps,
            vec![Step::DeleteBefore(1), Step::Commit("က".to_owned())]
        );
        assert_eq!(tail, "ခက");
    }

    #[test]
    fn delete_counts_bytes_not_chars() {
        assert_eq!(delete_bytes("aခ", 1), 3);
        assert_eq!(delete_bytes("ab", 5), 2);
        assert_eq!(delete_bytes("", 1), 0);
    }
}
