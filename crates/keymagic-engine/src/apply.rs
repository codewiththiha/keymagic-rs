//! Right-hand-side rendering and the recursion stop rule.

use crate::matcher::{Capture, Captures, Text};
use crate::states::StateSet;
use keymagic_types::{Keyboard, OutputElement};

/// Renders the right-hand side of rule `rule` and activates its states.
pub fn render(
    keyboard: &Keyboard,
    rule: usize,
    text: Text<'_>,
    captures: &Captures,
    states: &mut StateSet,
) -> String {
    let mut out = String::new();
    for element in &keyboard.rules[rule].rhs {
        match element {
            OutputElement::Text(literal) => out.push_str(literal),
            OutputElement::Variable(var) => out.push_str(&keyboard.strings[*var]),
            OutputElement::Ref(index) => push_capture(&mut out, text, captures.get(*index)),
            OutputElement::VarIndexed { var, by_ref } => push_indexed(
                &mut out,
                &keyboard.strings[*var],
                text,
                captures.get(*by_ref),
            ),
            OutputElement::Null => {}
            OutputElement::State(id) => states.insert(*id),
        }
    }
    out
}

/// True when `output` is empty or one printable ASCII character.
#[must_use]
pub fn stops_recursion(output: &str) -> bool {
    let mut chars = output.chars();
    match (chars.next(), chars.next()) {
        (None, _) => true,
        (Some(ch), None) => matches!(ch, '!'..='~'),
        _ => false,
    }
}

fn span(text: Text<'_>, capture: Capture) -> impl Iterator<Item = char> + '_ {
    (capture.start..capture.start + capture.len).filter_map(move |pos| text.at(pos))
}

/// The text each capture matched, in left-hand-side order.
#[must_use]
pub fn captured(text: Text<'_>, captures: &Captures) -> Vec<String> {
    captures
        .iter()
        .map(|&capture| span(text, capture).collect::<String>())
        .collect()
}

fn push_capture(out: &mut String, text: Text<'_>, capture: Option<&Capture>) {
    if let Some(&capture) = capture {
        out.extend(span(text, capture));
    }
}

fn push_indexed(out: &mut String, variable: &str, text: Text<'_>, capture: Option<&Capture>) {
    let Some(&capture) = capture else {
        out.push_str(variable);
        return;
    };
    let index = capture
        .index
        .or_else(|| span(text, capture).collect::<String>().parse().ok());
    if let Some(ch) = index.and_then(|index| variable.chars().nth(index)) {
        out.push(ch);
    }
}
