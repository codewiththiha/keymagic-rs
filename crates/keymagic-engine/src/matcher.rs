//! Right-aligned rule matching over composing text and the current key.

use crate::input::KeyEvent;
use crate::rules::{CompiledRule, Elem};
use crate::states::StateSet;
use smallvec::SmallVec;

/// Composing characters followed by an optional virtual trailing character.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Text<'a> {
    /// Composing characters before the tail.
    pub head: &'a [char],
    /// Delivered character treated as the last text position.
    pub tail: Option<char>,
}

impl Text<'_> {
    /// Returns the number of positions, including the tail.
    #[must_use]
    pub(crate) fn char_count(self) -> usize {
        self.head.len() + usize::from(self.tail.is_some())
    }

    /// Returns the character at `index`, if the position exists.
    #[must_use]
    pub(crate) fn at(self, index: usize) -> Option<char> {
        match self.head.get(index) {
            Some(ch) => Some(*ch),
            None if index == self.head.len() => self.tail,
            None => None,
        }
    }
}

/// A matched span of text and, for `[*]` matches, its position in the variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Capture {
    /// First text position of the span.
    pub start: usize,
    /// Number of text positions in the span.
    pub len: usize,
    /// Zero-based position of the character within the variable.
    pub index: Option<usize>,
}

/// Captures in left-hand-side order; capture n (one-based) is element n.
pub(crate) type Captures = SmallVec<[Capture; 8]>;

/// A rule that matched, with its text view and captures.
pub(crate) struct Found<'a, 'r> {
    /// The matched rule.
    pub rule: &'r CompiledRule,
    /// Text the rule matched against.
    pub text: Text<'a>,
    /// Captures produced by the left-hand side.
    pub captures: Captures,
    /// One-based position of the rule in priority order.
    pub rank: usize,
}

/// Finds the highest-priority rule matching the text and optional key.
#[must_use]
pub(crate) fn find<'a, 'r>(
    rules: &'r [CompiledRule],
    head: &'a [char],
    key: Option<&KeyEvent>,
    states: &StateSet,
) -> Option<Found<'a, 'r>> {
    rules.iter().enumerate().find_map(|(position, rule)| {
        let text = text_for(rule, head, key)?;
        let captures = match_rule(rule, text, key, states)?;
        Some(Found {
            rule,
            text,
            captures,
            rank: position + 1,
        })
    })
}

fn text_for<'a>(rule: &CompiledRule, head: &'a [char], key: Option<&KeyEvent>) -> Option<Text<'a>> {
    let tail = match (rule.has_vk(), key) {
        (true, None) => return None,
        (true, Some(_)) | (false, None) => None,
        (false, Some(event)) => Some(event.ch?),
    };
    Some(Text { head, tail })
}

fn match_rule(
    rule: &CompiledRule,
    text: Text<'_>,
    key: Option<&KeyEvent>,
    states: &StateSet,
) -> Option<Captures> {
    let end = text.char_count();
    let mut pos = end.checked_sub(rule.match_len)?;
    let mut captures = Captures::new();
    for element in &rule.elements {
        pos = step(element, text, pos, key, states, &mut captures)?;
    }
    (pos == end).then_some(captures)
}

fn step(
    element: &Elem,
    text: Text<'_>,
    pos: usize,
    key: Option<&KeyEvent>,
    states: &StateSet,
    captures: &mut Captures,
) -> Option<usize> {
    match element {
        Elem::State(id) => states.contains(*id).then_some(pos),
        Elem::Key(group) => group.matches(key?).then_some(pos),
        Elem::Text(chars) | Elem::Exact(chars) => {
            if !literal_at(text, pos, chars) {
                return None;
            }
            captures.push(Capture {
                start: pos,
                len: chars.len(),
                index: None,
            });
            Some(pos + chars.len())
        }
        Elem::AnyOf(chars) => {
            let ch = text.at(pos)?;
            let index = chars.iter().position(|c| *c == ch)?;
            captures.push(Capture {
                start: pos,
                len: 1,
                index: Some(index),
            });
            Some(pos + 1)
        }
        Elem::NotAnyOf(chars) => {
            let ch = text.at(pos)?;
            if chars.contains(&ch) {
                return None;
            }
            captures.push(Capture {
                start: pos,
                len: 1,
                index: None,
            });
            Some(pos + 1)
        }
        Elem::Any => {
            let ch = text.at(pos)?;
            if !matches!(ch, '!'..='~') {
                return None;
            }
            captures.push(Capture {
                start: pos,
                len: 1,
                index: None,
            });
            Some(pos + 1)
        }
    }
}

fn literal_at(text: Text<'_>, pos: usize, chars: &[char]) -> bool {
    chars
        .iter()
        .enumerate()
        .all(|(offset, ch)| text.at(pos + offset) == Some(*ch))
}
