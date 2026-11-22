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
