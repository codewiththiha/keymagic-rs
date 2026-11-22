//! Compiled rules: matchable elements, match length and priority.

use crate::input::{KeyEvent, Modifiers};
use keymagic_types::{Keyboard, PatternElement, Rule, StateId, VirtualKey};
use std::cmp::Reverse;

/// Left-hand-side element prepared for matching.
#[derive(Debug, Clone)]
pub enum Elem {
    /// Literal characters matched exactly.
    Text(Box<[char]>),
    /// Variable content matched exactly.
    Exact(Box<[char]>),
    /// One character drawn from the variable.
    AnyOf(Box<[char]>),
    /// One character outside the variable.
    NotAnyOf(Box<[char]>),
    /// One printable ASCII character.
    Any,
    /// The state must be active.
    State(StateId),
    /// One primary key with exact modifiers.
    Key(KeyGroup),
}

/// Virtual-key group: one primary key plus shift, control and alt requirements.
#[derive(Debug, Clone, Copy)]
pub struct KeyGroup {
    primary: VirtualKey,
    mods: Modifiers,
}

impl KeyGroup {
    /// Builds a group, or returns `None` unless it has exactly one primary key.
    fn new(keys: &[VirtualKey]) -> Option<Self> {
        let mut mods = Modifiers::default();
        let mut primary = None;
        let mut count = 0;
        for key in keys {
            match key {
                VirtualKey::Shift => mods.shift = true,
                VirtualKey::Control => mods.ctrl = true,
                VirtualKey::Menu => mods.alt = true,
                other => {
                    count += 1;
                    primary = Some(*other);
                }
            }
        }
        match (count, primary) {
            (1, Some(primary)) => Some(Self { primary, mods }),
            _ => None,
        }
    }

    /// Reports whether `event` presses the primary key with these modifiers.
    pub fn matches(self, event: &KeyEvent) -> bool {
        event.key == Some(self.primary)
            && event.mods.shift == self.mods.shift
            && event.mods.ctrl == self.mods.ctrl
            && event.mods.alt == self.mods.alt
    }
}

/// A rule prepared for matching.
#[derive(Debug, Clone)]
pub struct CompiledRule {
    /// Index of the source rule in the keyboard.
    pub index: usize,
    /// Left-hand-side elements in order.
    pub elements: Vec<Elem>,
    /// Text characters the left-hand side consumes.
    pub match_len: usize,
    /// Virtual keys in the left-hand side; zero means text-only.
    pub vk_count: usize,
    /// State requirements in the left-hand side.
    pub state_count: usize,
    /// Literal and wildcard characters; exact variables are excluded.
    pub order_len: usize,
}

impl CompiledRule {
    /// Reports whether the left-hand side requires a virtual key.
    pub const fn has_vk(&self) -> bool {
        self.vk_count > 0
    }

    fn compile(index: usize, rule: &Rule, strings: &[String]) -> Option<Self> {
        let mut compiled = Self {
            index,
            elements: Vec::with_capacity(rule.lhs.len()),
            match_len: 0,
            vk_count: 0,
            state_count: 0,
            order_len: 0,
        };
        for element in &rule.lhs {
            compiled.push_element(element, strings)?;
        }
        Some(compiled)
    }

    fn push_element(&mut self, element: &PatternElement, strings: &[String]) -> Option<()> {
        let (elem, consumed, ordered) = match element {
            PatternElement::Text(text) => {
                let chars: Box<[char]> = text.chars().collect();
                let len = chars.len();
                (Elem::Text(chars), len, len)
            }
            PatternElement::Variable(var) => {
                let chars = var_chars(strings, *var)?;
                let len = chars.len();
                (Elem::Exact(chars), len, 0)
            }
            PatternElement::AnyOf(var) => (Elem::AnyOf(var_chars(strings, *var)?), 1, 1),
            PatternElement::NotAnyOf(var) => (Elem::NotAnyOf(var_chars(strings, *var)?), 1, 1),
            PatternElement::Any => (Elem::Any, 1, 1),
            PatternElement::State(id) => {
                self.state_count += 1;
                (Elem::State(*id), 0, 0)
            }
            PatternElement::VirtualKeys(keys) => {
                self.vk_count += keys.len();
                (Elem::Key(KeyGroup::new(keys)?), 0, 0)
            }
        };
        self.elements.push(elem);
        self.match_len += consumed;
        self.order_len += ordered;
        Some(())
    }
}

fn var_chars(strings: &[String], var: usize) -> Option<Box<[char]>> {
    strings.get(var).map(|text| text.chars().collect())
}

/// Compiles every rule that can match, sorted by priority.
pub fn compile(keyboard: &Keyboard) -> Vec<CompiledRule> {
    let mut rules: Vec<CompiledRule> = keyboard
        .rules
        .iter()
        .enumerate()
        .filter_map(|(index, rule)| CompiledRule::compile(index, rule, &keyboard.strings))
        .collect();
    rules.sort_by_key(|rule| {
        (
            Reverse(rule.state_count),
            Reverse(rule.vk_count),
            Reverse(rule.order_len),
            rule.index,
        )
    });
    rules
}
