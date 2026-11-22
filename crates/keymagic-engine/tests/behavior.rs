//! Behavior of hand-built keyboards: composing, chaining, states and backspace.

use keymagic_engine::{Action, Engine, KeyEvent, MAX_RECURSION_DEPTH, Modifiers, Output, VirtualKey};
use keymagic_types::{Keyboard, LayoutOptions, OutputElement, PatternElement, Rule, StateId};
use std::error::Error;
use std::sync::Arc;

type TestResult = Result<(), Box<dyn Error>>;

const STATE: StateId = 1;

fn rule(lhs: Vec<PatternElement>, rhs: Vec<OutputElement>) -> Rule {
    Rule {
        lhs,
        rhs,
        source: None,
    }
}

fn text_rule(from: &str, to: &str) -> Rule {
    rule(
        vec![PatternElement::Text(from.to_owned())],
        vec![OutputElement::Text(to.to_owned())],
    )
}

fn key_rule(key: VirtualKey, to: &str) -> Rule {
    rule(
        vec![PatternElement::VirtualKeys(vec![key])],
        vec![OutputElement::Text(to.to_owned())],
    )
}

fn engine_with(
    rules: Vec<Rule>,
    options: LayoutOptions,
    strings: Vec<String>,
) -> Result<Engine, Box<dyn Error>> {
    let keyboard = Keyboard {
        options,
        strings,
        rules,
        ..Keyboard::default()
    };
    Ok(Engine::new(Arc::new(keyboard))?)
}

fn engine(rules: Vec<Rule>) -> Result<Engine, Box<dyn Error>> {
    engine_with(rules, LayoutOptions::default(), Vec::new())
}

fn char_event(ch: char) -> KeyEvent {
    KeyEvent {
        ch: Some(ch),
        ..KeyEvent::default()
    }
}

fn key_event(key: VirtualKey) -> KeyEvent {
    KeyEvent {
        key: Some(key),
        ..KeyEvent::default()
    }
}

fn type_text(engine: &mut Engine, text: &str) -> Result<Output, Box<dyn Error>> {
    let last = text.chars().map(|ch| engine.process(char_event(ch))).last();
    last.ok_or_else(|| "empty text".into())
}

#[test]
fn ka_maps_to_one_character_with_delete_and_insert() -> TestResult {
    let mut engine = engine(vec![text_rule("ka", "က")])?;
    let first = engine.process(char_event('k'));
    assert_eq!(first.action, Action::Insert("k".into()));
    assert_eq!(first.composing, "k");
    let second = engine.process(char_event('a'));
    assert_eq!(
        second.action,
        Action::Replace {
            delete: 1,
            insert: "က".into()
        }
    );
    assert_eq!(second.composing, "က");
    assert!(second.consumed);
    Ok(())
}

#[test]
fn title_replaces_the_whole_match_at_the_end() -> TestResult {
    let mut engine = engine(vec![text_rule("title", "Title")])?;
    let typed = type_text(&mut engine, "titl")?;
    assert_eq!(typed.composing, "titl");
    let output = engine.process(char_event('e'));
    assert_eq!(
        output.action,
        Action::Replace {
            delete: 4,
            insert: "Title".into()
        }
    );
    assert_eq!(engine.composing(), "Title");
    Ok(())
}

#[test]
fn virtual_key_output_feeds_a_later_text_rule() -> TestResult {
    let mut engine = engine(vec![
        key_rule(VirtualKey::KeyA, "ab"),
        text_rule("abc", "x"),
    ])?;
    let a = engine.process(key_event(VirtualKey::KeyA));
    assert_eq!(a.composing, "ab");
    let c = engine.process(char_event('c'));
    assert_eq!(
        c.action,
        Action::Replace {
            delete: 2,
            insert: "x".into()
        }
    );
    assert_eq!(c.composing, "x");
    Ok(())
}

#[test]
fn recursion_applies_text_rules_to_a_rule_output() -> TestResult {
    let mut engine = engine(vec![key_rule(VirtualKey::KeyA, "xy"), text_rule("xy", "z")])?;
    let output = engine.process(key_event(VirtualKey::KeyA));
    assert_eq!(output.action, Action::Insert("z".into()));
    assert_eq!(output.composing, "z");
    Ok(())
}

#[test]
fn identity_rule_stops_recursion_without_looping() -> TestResult {
    let mut engine = engine(vec![text_rule("a", "a")])?;
    let output = type_text(&mut engine, "aaaaa")?;
    assert_eq!(output.composing, "aaaaa");
    Ok(())
}

#[test]
fn growing_rule_is_capped_at_the_recursion_depth() -> TestResult {
    let mut engine = engine(vec![text_rule("a", "aa")])?;
    let output = engine.process(char_event('a'));
    assert_eq!(output.composing.chars().count(), 2 + MAX_RECURSION_DEPTH);
    Ok(())
}

#[test]
fn state_set_by_one_event_applies_to_the_next_only() -> TestResult {
    let mut engine = engine(vec![
        rule(
            vec![PatternElement::Text("x".into())],
            vec![OutputElement::Null, OutputElement::State(STATE)],
        ),
        rule(
            vec![
                PatternElement::State(STATE),
                PatternElement::Text("y".into()),
            ],
            vec![OutputElement::Text("Y".into())],
        ),
    ])?;
    engine.process(char_event('x'));
    assert_eq!(engine.process(char_event('y')).composing, "Y");
    assert_eq!(engine.process(char_event('y')).composing, "Yy");
    Ok(())
}

#[test]
fn reset_and_set_composing_clear_states() -> TestResult {
    let mut engine = engine(vec![
        rule(
            vec![PatternElement::Text("x".into())],
            vec![OutputElement::Null, OutputElement::State(STATE)],
        ),
        rule(
            vec![
                PatternElement::State(STATE),
                PatternElement::Text("y".into()),
            ],
            vec![OutputElement::Text("Y".into())],
        ),
    ])?;
    engine.process(char_event('x'));
    engine.set_composing("");
    assert_eq!(engine.process(char_event('y')).composing, "y");
    engine.process(char_event('x'));
    engine.reset();
    assert_eq!(engine.process(char_event('y')).composing, "y");
    Ok(())
}

#[test]
fn reset_clears_composing_text() -> TestResult {
    let mut engine = engine(Vec::new())?;
    engine.process(char_event('a'));
    engine.reset();
    assert_eq!(engine.composing(), "");
    Ok(())
}

#[test]
fn set_composing_is_visible_to_the_next_key() -> TestResult {
    let mut engine = engine(Vec::new())?;
    engine.set_composing("ab");
    let output = engine.process(char_event('c'));
    assert_eq!(output.action, Action::Insert("c".into()));
    assert_eq!(output.composing, "abc");
    Ok(())
}

#[test]
fn scalar_values_are_counted_not_bytes() -> TestResult {
    let mut engine = engine(vec![rule(
        vec![PatternElement::Text("\u{301}z".into())],
        vec![OutputElement::Text("Z".into())],
    )])?;
    engine.set_composing("e\u{301}");
    let output = engine.process(char_event('z'));
    assert_eq!(
        output.action,
        Action::Replace {
            delete: 1,
            insert: "Z".into()
        }
    );
    assert_eq!(output.composing, "eZ");
    Ok(())
}

#[test]
fn peek_leaves_the_engine_unchanged() -> TestResult {
    let mut engine = engine(vec![text_rule("ka", "က")])?;
    engine.process(char_event('k'));
    let peeked = engine.peek(char_event('a'));
    assert_eq!(peeked.composing, "က");
    assert_eq!(engine.composing(), "k");
    Ok(())
}

#[test]
fn smart_backspace_restores_the_previous_snapshot() -> TestResult {
    let options = LayoutOptions {
        auto_bksp: true,
        ..LayoutOptions::default()
    };
    let mut engine = engine_with(Vec::new(), options, Vec::new())?;
    type_text(&mut engine, "ab")?;
    let first = engine.process(key_event(VirtualKey::Back));
    assert_eq!(first.action, Action::Delete(1));
    assert_eq!(first.composing, "a");
    let second = engine.process(key_event(VirtualKey::Back));
    assert_eq!(second.composing, "");
    let third = engine.process(key_event(VirtualKey::Back));
    assert!(!third.consumed);
    assert_eq!(third.action, Action::None);
    Ok(())
}

#[test]
fn backspace_without_smart_option_deletes_one_scalar() -> TestResult {
    let mut engine = engine(Vec::new())?;
    type_text(&mut engine, "ab")?;
    let output = engine.process(key_event(VirtualKey::Back));
    assert_eq!(output.action, Action::Delete(1));
    assert_eq!(output.composing, "a");
    Ok(())
}

#[test]
fn eat_option_consumes_unmatched_characters() -> TestResult {
    let options = LayoutOptions {
        eat: true,
        ..LayoutOptions::default()
    };
    let mut engine = engine_with(Vec::new(), options, Vec::new())?;
    let output = engine.process(char_event('q'));
    assert!(output.consumed);
    assert_eq!(output.action, Action::None);
    assert_eq!(output.composing, "");
    Ok(())
}

#[test]
fn any_of_captures_index_into_the_variable() -> TestResult {
    let strings = vec!["abc".to_owned(), "xyz".to_owned()];
    let mut engine = engine_with(
        vec![rule(
            vec![PatternElement::AnyOf(0)],
            vec![OutputElement::VarIndexed { var: 1, by_ref: 0 }],
        )],
        LayoutOptions::default(),
        strings,
    )?;
    assert_eq!(engine.process(char_event('b')).composing, "y");
    Ok(())
}

#[test]
fn any_matches_printable_ascii_only() -> TestResult {
    let mut engine = engine(vec![rule(
        vec![PatternElement::Any],
        vec![OutputElement::Ref(0)],
    )])?;
    assert_eq!(engine.process(char_event('~')).composing, "~");
    let space = engine.process(char_event(' '));
    assert_eq!(space.composing, "~ ");
    Ok(())
}

#[test]
fn virtual_key_requires_exact_modifiers() -> TestResult {
    let mut engine = engine(vec![rule(
        vec![PatternElement::VirtualKeys(vec![
            VirtualKey::Shift,
            VirtualKey::KeyA,
        ])],
        vec![OutputElement::Text("S".into())],
    )])?;
    let plain = engine.process(KeyEvent {
        key: Some(VirtualKey::KeyA),
        ..KeyEvent::default()
    });
    assert!(!plain.consumed);
    let shifted = engine.process(KeyEvent {
        key: Some(VirtualKey::KeyA),
        ch: None,
        mods: Modifiers {
            shift: true,
            ..Modifiers::default()
        },
    });
    assert_eq!(shifted.composing, "S");
    Ok(())
}
