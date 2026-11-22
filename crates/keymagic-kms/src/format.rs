//! Canonical formatter: one item per line, comments kept above their item.

use crate::ast::{Out, Pat, Rule, Script, Value, Variable};
use crate::diag::{Diagnostic, Span};
use crate::lexer;
use crate::parser::parse;

const WIDTH: usize = 100;

/// Formats one file without following includes.
#[doc = "# Errors\n\nReturns the syntax errors that stop formatting."]
pub fn format(src: &str) -> Result<String, Vec<Diagnostic>> {
    let src = src.strip_prefix('\u{feff}').unwrap_or(src);
    let script: Script = parse(src).map_err(|error| vec![error])?;
    let comments = lexer::lex(src).map_err(|error| vec![error])?.comments;
    let mut items: Vec<(Span, String)> = Vec::new();
    for include in &script.includes {
        items.push((
            include.span.clone(),
            format!("include(\"{}\")", include.path),
        ));
    }
    for variable in &script.variables {
        items.push((variable.span.clone(), variable_text(variable, src)));
    }
    for rule in &script.rules {
        items.push((rule.span.clone(), rule_text(rule, src)));
    }
    items.sort_by_key(|(span, _)| span.start);
    Ok(layout(src, &items, &comments))
}

fn variable_text(variable: &Variable, src: &str) -> String {
    let pieces: Vec<String> = variable
        .value
        .iter()
        .map(|part| match part {
            Value::Str(_, span) => source(src, span),
            Value::Uni(code, _) => format!("U{code:04X}"),
            Value::Var(name, _) => format!("${name}"),
        })
        .collect();
    let mut out = format!("${} =", variable.name);
    let mut width = out.chars().count();
    for (index, piece) in pieces.iter().enumerate() {
        let piece_width = piece.chars().count();
        if index == 0 {
            out.push(' ');
            width += 1;
        } else if width + 3 + piece_width + 2 > WIDTH {
            out.push_str(" + \\\n    ");
            width = 4;
        } else {
            out.push_str(" + ");
            width += 3;
        }
        out.push_str(piece);
        width += piece_width;
    }
    out
}

fn rule_text(rule: &Rule, src: &str) -> String {
    let lhs: Vec<String> = rule
        .lhs
        .iter()
        .map(|pattern| pattern_text(pattern, src))
        .collect();
    let rhs: Vec<String> = rule
        .rhs
        .iter()
        .map(|output| output_text(output, src))
        .collect();
    format!("{} => {}", lhs.join(" + "), rhs.join(" + "))
}

fn pattern_text(pattern: &Pat, src: &str) -> String {
    match pattern {
        Pat::Str(_, span) | Pat::State(_, span) => source(src, span),
        Pat::Uni(code, _) => format!("U{code:04X}"),
        Pat::Var(name, _) => format!("${name}"),
        Pat::AnyOf(name, _) => format!("${name}[*]"),
        Pat::NotAnyOf(name, _) => format!("${name}[^]"),
        Pat::Any(_) => "ANY".to_owned(),
        Pat::Keys(keys, _) => {
            let names: Vec<&str> = keys.iter().map(|(name, _)| name.as_str()).collect();
            format!("<{}>", names.join(" & "))
        }
    }
}

fn output_text(output: &Out, src: &str) -> String {
    match output {
        Out::Str(_, span) | Out::State(_, span) => source(src, span),
        Out::Uni(code, _) => format!("U{code:04X}"),
        Out::Var(name, _) => format!("${name}"),
        Out::Indexed(name, number, _) => format!("${name}[${number}]"),
        Out::Ref(number, _) => format!("${number}"),
        Out::Null(_) => "NULL".to_owned(),
    }
}

/// Lays out items in order. Comments that touch an item go above it.
fn layout(src: &str, items: &[(Span, String)], comments: &[Span]) -> String {
    let mut out = String::new();
    let mut cursor = 0;
    for (span, text) in items {
        let lead: Vec<Span> = comments
            .iter()
            .filter(|comment| comment.start >= cursor && comment.start < span.end)
            .cloned()
            .collect();
        let gap_end = lead
            .first()
            .map_or(span.start, |comment| comment.start.min(span.start));
        if !out.is_empty() && blank_line(src, cursor..gap_end) {
            out.push('\n');
        }
        for comment in &lead {
            out.push_str(&source(src, comment));
            out.push('\n');
        }
        out.push_str(text);
        out.push('\n');
        cursor = span.end;
    }
    let tail: Vec<Span> = comments
        .iter()
        .filter(|comment| comment.start >= cursor)
        .cloned()
        .collect();
    let gap = tail
        .first()
        .is_some_and(|first| !out.is_empty() && blank_line(src, cursor..first.start));
    if gap {
        out.push('\n');
    }
    for comment in &tail {
        out.push_str(&source(src, comment));
        out.push('\n');
    }
    out
}

fn blank_line(src: &str, span: Span) -> bool {
    source(src, &span).matches('\n').count() >= 2
}

fn source(src: &str, span: &Span) -> String {
    src.get(span.clone()).unwrap_or("").to_owned()
}

#[cfg(test)]
mod tests {
    use super::format;

    #[test]
    fn wraps_long_variables_at_the_width() -> Result<(), Vec<crate::Diagnostic>> {
        let value = "\"ကခဂဃငစဆဇဈဉညဋဌဍဎဏတထဒဓနပဖဗဘမယရလဝသဟဠအ\"";
        let src = format!("$long = {value} + {value} + {value}\n");
        let out = format(&src)?;
        assert!(out.lines().all(|line| line.chars().count() <= 100 + 2));
        assert!(out.contains(" + \\\n    "));
        Ok(())
    }
}
