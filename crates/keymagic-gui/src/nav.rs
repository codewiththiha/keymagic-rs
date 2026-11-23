//! The six pages of the app and their order in the nav rail.

/// One page of the app, in nav rail order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Page {
    #[default]
    Keyboards,
    Tester,
    Editor,
    Settings,
    Hotkeys,
    About,
}

impl Page {
    /// Every page in nav rail order.
    pub const ALL: [Self; 6] = [
        Self::Keyboards,
        Self::Tester,
        Self::Editor,
        Self::Settings,
        Self::Hotkeys,
        Self::About,
    ];

    /// The nav rail label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Keyboards => "Keyboards",
            Self::Tester => "Tester",
            Self::Editor => "Editor",
            Self::Settings => "Settings",
            Self::Hotkeys => "Hotkeys",
            Self::About => "About",
        }
    }

    /// One line shown at the top of the page.
    #[must_use]
    pub const fn summary(self) -> &'static str {
        match self {
            Self::Keyboards => "Install, enable and switch keyboards.",
            Self::Tester => "Type with the active keyboard and see the output.",
            Self::Editor => "Edit, compile and check a keyboard file.",
            Self::Settings => "Startup, updates and per-app modes.",
            Self::Hotkeys => "The global switch and next-keyboard shortcut.",
            Self::About => "Version, licences and credits.",
        }
    }

    /// The page after this one, wrapping at the end.
    #[must_use]
    pub const fn next(self) -> Self {
        Self::ALL[(self.index() + 1) % Self::ALL.len()]
    }

    /// The page before this one, wrapping at the start.
    #[must_use]
    pub const fn prev(self) -> Self {
        Self::ALL[(self.index() + Self::ALL.len() - 1) % Self::ALL.len()]
    }

    /// The page for a 1-based number, as used by Ctrl+1 to Ctrl+6.
    #[must_use]
    pub fn from_number(number: usize) -> Option<Self> {
        number
            .checked_sub(1)
            .and_then(|i| Self::ALL.get(i).copied())
    }

    const fn index(self) -> usize {
        match self {
            Self::Keyboards => 0,
            Self::Tester => 1,
            Self::Editor => 2,
            Self::Settings => 3,
            Self::Hotkeys => 4,
            Self::About => 5,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Page;
    use std::collections::HashSet;

    #[test]
    fn labels_are_unique_and_in_rail_order() {
        let labels: Vec<_> = Page::ALL.iter().map(|page| page.label()).collect();
        let unique: HashSet<_> = labels.iter().copied().collect();
        assert_eq!(unique.len(), 6);
        assert_eq!(
            labels,
            [
                "Keyboards",
                "Tester",
                "Editor",
                "Settings",
                "Hotkeys",
                "About"
            ]
        );
    }

    #[test]
    fn next_and_prev_wrap_at_both_ends() {
        assert_eq!(Page::About.next(), Page::Keyboards);
        assert_eq!(Page::Keyboards.prev(), Page::About);
        assert_eq!(Page::Tester.next(), Page::Editor);
        assert_eq!(Page::Tester.prev(), Page::Keyboards);
    }

    #[test]
    fn every_page_round_trips_through_next() {
        let mut page = Page::Keyboards;
        for _ in 0..Page::ALL.len() {
            page = page.next();
        }
        assert_eq!(page, Page::Keyboards);
    }

    #[test]
    fn numbers_select_pages_from_one() {
        assert_eq!(Page::from_number(1), Some(Page::Keyboards));
        assert_eq!(Page::from_number(6), Some(Page::About));
        assert_eq!(Page::from_number(0), None);
        assert_eq!(Page::from_number(7), None);
    }
}
