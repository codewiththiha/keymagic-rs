//! Property checks over random key streams on a hand-built keyboard.

use keymagic_engine::{Action, Engine, KeyEvent, VirtualKey};
use keymagic_types::{Keyboard, LayoutOptions, OutputElement, PatternElement, Rule};
use std::error::Error;
use std::sync::Arc;

type TestResult = Result<(), Box<dyn Error>>;

const STREAMS: u64 = 200;
const STEPS: usize = 60;
const SEED_MIX: u64 = 0x9E37_79B9_7F4A_7C15;
const CHARS: [char; 6] = ['a', 'b', 'k', 'x', 'y', 'z'];

struct Rng(u64);

impl Rng {
    const fn draw(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, bound: usize) -> usize {
        let modulus = u64::try_from(bound).unwrap_or(1);
        usize::try_from(self.draw() % modulus).unwrap_or(0)
    }
}

fn text_rule(from: &str, to: &str) -> Rule {
    Rule {
        lhs: vec![PatternElement::Text(from.to_owned())],
        rhs: vec![OutputElement::Text(to.to_owned())],
        source: None,
    }
}

fn back_rule(from: &str, to: &str) -> Rule {
    Rule {
        lhs: vec![
            PatternElement::Text(from.to_owned()),
            PatternElement::VirtualKeys(vec![VirtualKey::Back]),
        ],
        rhs: vec![OutputElement::Text(to.to_owned())],
        source: None,
    }
}

fn keyboard() -> Keyboard {
    Keyboard {
        options: LayoutOptions {
            auto_bksp: true,
            ..LayoutOptions::default()
        },
        rules: vec![
            text_rule("ab", "X"),
            text_rule("X", "xyz"),
            text_rule("a", "bb"),
            text_rule("b", "b"),
            back_rule("y", "q"),
            back_rule("k", ""),
        ],
        ..Keyboard::default()
    }
}

fn random_event(rng: &mut Rng) -> KeyEvent {
    if rng.draw().is_multiple_of(5) {
        KeyEvent {
            key: Some(VirtualKey::Back),
            ..KeyEvent::default()
        }
    } else {
        KeyEvent {
            ch: Some(CHARS[rng.below(CHARS.len())]),
            ..KeyEvent::default()
        }
    }
}

/// Rebuilds the composing text that `action` turns `previous` into.
fn apply(previous: &str, action: &Action) -> String {
    let (delete, insert) = match action {
        Action::None => (0, ""),
        Action::Insert(text) => (0, text.as_str()),
        Action::Delete(count) => (*count, ""),
        Action::Replace { delete, insert } => (*delete, insert.as_str()),
    };
    let keep = previous.chars().count().saturating_sub(delete);
    previous.chars().take(keep).chain(insert.chars()).collect()
}

#[test]
fn edits_stay_within_the_composing_text() -> TestResult {
    let mut engine = Engine::new(Arc::new(keyboard()))?;
    for seed in 1..=STREAMS {
        engine.reset();
        let mut rng = Rng(SEED_MIX ^ seed);
        for _ in 0..STEPS {
            let input = random_event(&mut rng);
            let before = engine.composing().to_owned();
            let previewed = engine.peek(input);
            assert_eq!(
                engine.composing(),
                before,
                "peek changed the composing text"
            );
            let output = engine.process(input);
            assert_eq!(output.action, previewed.action, "peek differs from process");
            let delete = match &output.action {
                Action::None | Action::Insert(_) => 0,
                Action::Delete(count) => *count,
                Action::Replace { delete, .. } => *delete,
            };
            assert!(
                delete <= before.chars().count(),
                "seed {seed}: delete {delete} exceeds {before:?}"
            );
            assert_eq!(
                apply(&before, &output.action),
                output.composing,
                "seed {seed}"
            );
        }
    }
    Ok(())
}
