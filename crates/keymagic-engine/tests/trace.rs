//! Trace of applied rules and the recursion cap, with the `trace` feature.

#![cfg(feature = "trace")]

use keymagic_engine::{Engine, KeyEvent, MAX_RECURSION_DEPTH, TraceStep};
use keymagic_types::{Keyboard, OutputElement, PatternElement, Rule};
use std::error::Error;
use std::sync::Arc;

#[test]
fn trace_records_the_first_match_then_the_capped_recursion() -> Result<(), Box<dyn Error>> {
    let rule = Rule {
        lhs: vec![PatternElement::Text("a".into())],
        rhs: vec![OutputElement::Text("aa".into())],
        source: None,
    };
    let keyboard = Keyboard {
        rules: vec![rule],
        ..Keyboard::default()
    };
    let mut engine = Engine::new(Arc::new(keyboard))?;
    let output = engine.process(KeyEvent {
        ch: Some('a'),
        ..KeyEvent::default()
    });
    let steps = output.trace.ok_or("trace missing")?.steps;
    assert_eq!(
        steps.first(),
        Some(&TraceStep::Applied {
            rule: 0,
            rank: 1,
            recursive: false,
            output: "aa".into(),
            captures: vec!["a".into()],
        })
    );
    let recursive = steps
        .iter()
        .filter(|step| {
            matches!(
                step,
                TraceStep::Applied {
                    recursive: true,
                    ..
                }
            )
        })
        .count();
    assert_eq!(recursive, MAX_RECURSION_DEPTH);
    assert_eq!(steps.last(), Some(&TraceStep::Capped));
    Ok(())
}

#[test]
fn identity_rule_of_two_chars_hits_the_cap_and_keeps_its_text() -> Result<(), Box<dyn Error>> {
    let rule = Rule {
        lhs: vec![PatternElement::Text("ab".into())],
        rhs: vec![OutputElement::Text("ab".into())],
        source: None,
    };
    let keyboard = Keyboard {
        rules: vec![rule],
        ..Keyboard::default()
    };
    let mut engine = Engine::new(Arc::new(keyboard))?;
    engine.process(KeyEvent {
        ch: Some('a'),
        ..KeyEvent::default()
    });
    let output = engine.process(KeyEvent {
        ch: Some('b'),
        ..KeyEvent::default()
    });
    assert_eq!(output.composing, "ab");
    let steps = output.trace.ok_or("trace missing")?.steps;
    assert_eq!(steps.last(), Some(&TraceStep::Capped));
    Ok(())
}

#[test]
fn trace_records_the_text_each_capture_matched() -> Result<(), Box<dyn Error>> {
    let rule = Rule {
        lhs: vec![PatternElement::Any],
        rhs: vec![OutputElement::Ref(0), OutputElement::Text("!".into())],
        source: None,
    };
    let keyboard = Keyboard {
        rules: vec![rule],
        ..Keyboard::default()
    };
    let mut engine = Engine::new(Arc::new(keyboard))?;
    let output = engine.process(KeyEvent {
        ch: Some('k'),
        ..KeyEvent::default()
    });
    let steps = output.trace.ok_or("trace missing")?.steps;
    assert_eq!(
        steps.first(),
        Some(&TraceStep::Applied {
            rule: 0,
            rank: 1,
            recursive: false,
            output: "k!".into(),
            captures: vec!["k".into()],
        })
    );
    Ok(())
}
