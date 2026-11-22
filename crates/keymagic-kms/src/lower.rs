//! Lowers a parsed script to the shared keyboard IR, and reports warnings.

use crate::ast::{Out, Pat, Rule, Script, Value, Variable};
use crate::diag::{Diagnostic, Span};
use crate::resolve::Resolver;
use keymagic_types::{
    InfoEntry, Keyboard, KeyboardInfo, LayoutOptions, OutputElement, PatternElement,
    Rule as IrRule, StateId, VarId, VirtualKey,
};
use std::collections::{HashMap, HashSet};

const ICON_LIMIT: usize = 1024 * 1024;
const INFO_IDS: [(&str, [u8; 4]); 4] = [
    ("NAME", *b"eman"),
    ("DESCRIPTION", *b"csed"),
    ("FONTFAMILY", *b"tnof"),
    ("HOTKEY", *b"ykth"),
];

/// Lowers a script whose includes are already expanded.
#[doc = "# Errors\n\nReturns the first error: undefined names, bad back-references, unknown keys or states, or an unusable icon."]
pub fn lower(script: &Script, resolver: &dyn Resolver) -> Result<Keyboard, Diagnostic> {
    let mut lowerer = Lowerer::default();
    for variable in &script.variables {
        lowerer.variable(variable)?;
    }
    for rule in &script.rules {
        lowerer.register_states(rule)?;
    }
    let rules = script
        .rules
        .iter()
        .map(|rule| lowerer.rule(rule))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Keyboard {
        options: layout(&script.options),
        strings: lowerer.strings,
        info: KeyboardInfo {
            entries: info(&script.options, resolver)?,
        },
        rules,
    })
}

/// Non-fatal warnings: repeated names and states that nothing reads.
#[must_use]
pub fn warnings(script: &Script) -> Vec<Diagnostic> {
    let mut found = Vec::new();
    let mut seen_variables = HashSet::new();
    for variable in &script.variables {
        if !seen_variables.insert(variable.name.as_str()) {
            found.push(Diagnostic::warning(
                variable.span.clone(),
                format!(
                    "variable '${}' is defined again; later uses see the new value",
                    variable.name
                ),
            ));
        }
    }
    let mut seen_options = HashSet::new();
    for (key, _) in &script.options {
        if !seen_options.insert(key.as_str()) {
            found.push(Diagnostic::warning(
                0..0,
                format!("option '{key}' is set more than once; the last value is used"),
            ));
        }
    }
    let read: HashSet<&str> = script
        .rules
        .iter()
        .flat_map(|rule| rule.lhs.iter())
        .filter_map(|pattern| match pattern {
            Pat::State(name, _) => Some(name.as_str()),
            _ => None,
        })
        .collect();
    for rule in &script.rules {
        for output in &rule.rhs {
            let Out::State(name, span) = output else {
                continue;
            };
            if !read.contains(name.as_str()) {
                found.push(Diagnostic::warning(
                    span.clone(),
                    format!("state '{name}' is set here but no left side reads it"),
                ));
            }
        }
    }
    found
}

#[derive(Default)]
struct Lowerer {
    strings: Vec<String>,
    vars: HashMap<String, VarId>,
    states: HashMap<String, StateId>,
}

impl Lowerer {
    fn variable(&mut self, variable: &Variable) -> Result<(), Diagnostic> {
        let mut value = String::new();
        for part in &variable.value {
            match part {
                Value::Str(raw, span) => value.push_str(&escapes(raw, span)?),
                Value::Uni(code, span) => value.push(scalar(*code, span)?),
                Value::Var(name, span) => {
                    let index = self.var(name, span)?;
                    value.push_str(&self.strings[index]);
                }
            }
        }
        let id = self.intern(value);
        self.vars.insert(variable.name.clone(), id);
        Ok(())
    }

    fn intern(&mut self, value: String) -> VarId {
        if let Some(index) = self.strings.iter().position(|known| *known == value) {
            index
        } else {
            self.strings.push(value);
            self.strings.len() - 1
        }
    }

    fn var(&self, name: &str, span: &Span) -> Result<VarId, Diagnostic> {
        self.vars
            .get(name)
            .copied()
            .ok_or_else(|| Diagnostic::error(span.clone(), format!("undefined variable '${name}'")))
    }

    fn register_states(&mut self, rule: &Rule) -> Result<(), Diagnostic> {
        for pattern in &rule.lhs {
            if let Pat::State(name, span) = pattern {
                self.register(name, span)?;
            }
        }
        for output in &rule.rhs {
            if let Out::State(name, span) = output {
                self.register(name, span)?;
            }
        }
        Ok(())
    }

    fn register(&mut self, name: &str, span: &Span) -> Result<(), Diagnostic> {
        if !self.states.contains_key(name) {
            let id = StateId::try_from(self.states.len()).map_err(|_| {
                Diagnostic::error(span.clone(), "too many states; the limit is 65536")
            })?;
            self.states.insert(name.to_owned(), id);
        }
        Ok(())
    }

    fn state(&self, name: &str, span: &Span) -> Result<StateId, Diagnostic> {
        self.states
            .get(name)
            .copied()
            .ok_or_else(|| Diagnostic::error(span.clone(), format!("unknown state '{name}'")))
    }

    fn rule(&self, rule: &Rule) -> Result<IrRule, Diagnostic> {
        let lhs = rule
            .lhs
            .iter()
            .map(|pattern| self.pattern(pattern))
            .collect::<Result<Vec<_>, _>>()?;
        let captures = lhs.iter().filter(|element| is_capture(element)).count();
        let rhs = rule
            .rhs
            .iter()
            .map(|output| self.output(output, captures))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(IrRule {
            lhs,
            rhs,
            source: None,
        })
    }

    fn pattern(&self, pattern: &Pat) -> Result<PatternElement, Diagnostic> {
        match pattern {
            Pat::Str(raw, span) => Ok(PatternElement::Text(escapes(raw, span)?)),
            Pat::Uni(code, span) => Ok(PatternElement::Text(scalar(*code, span)?.to_string())),
            Pat::Var(name, span) => Ok(PatternElement::Variable(self.var(name, span)?)),
            Pat::AnyOf(name, span) => Ok(PatternElement::AnyOf(self.var(name, span)?)),
            Pat::NotAnyOf(name, span) => Ok(PatternElement::NotAnyOf(self.var(name, span)?)),
            Pat::Any(_) => Ok(PatternElement::Any),
            Pat::Keys(keys, _) => keys
                .iter()
                .map(|(name, span)| virtual_key(name, span))
                .collect::<Result<Vec<_>, _>>()
                .map(PatternElement::VirtualKeys),
            Pat::State(name, span) => Ok(PatternElement::State(self.state(name, span)?)),
        }
    }

    fn output(&self, output: &Out, captures: usize) -> Result<OutputElement, Diagnostic> {
        match output {
            Out::Str(raw, span) => Ok(OutputElement::Text(escapes(raw, span)?)),
            Out::Uni(code, span) => Ok(OutputElement::Text(scalar(*code, span)?.to_string())),
            Out::Var(name, span) => Ok(OutputElement::Variable(self.var(name, span)?)),
            Out::Indexed(name, number, span) => Ok(OutputElement::VarIndexed {
                var: self.var(name, span)?,
                by_ref: capture(*number, captures, span)?,
            }),
            Out::Ref(number, span) => Ok(OutputElement::Ref(capture(*number, captures, span)?)),
            Out::Null(_) => Ok(OutputElement::Null),
            Out::State(name, span) => Ok(OutputElement::State(self.state(name, span)?)),
        }
    }
}

/// Text-consuming elements yield a capture; keys and states do not.
const fn is_capture(element: &PatternElement) -> bool {
    matches!(
        element,
        PatternElement::Text(_)
            | PatternElement::Variable(_)
            | PatternElement::AnyOf(_)
            | PatternElement::NotAnyOf(_)
            | PatternElement::Any
    )
}

/// Maps a 1-based `$n` to the zero-based IR index.
fn capture(number: usize, captures: usize, span: &Span) -> Result<usize, Diagnostic> {
    if (1..=captures).contains(&number) {
        Ok(number - 1)
    } else {
        let valid: Vec<String> = (1..=captures).map(|n| format!("${n}")).collect();
        let help = if valid.is_empty() {
            "the left side has no captures".to_owned()
        } else {
            format!("valid references: {}", valid.join(", "))
        };
        let message = format!("${number} is not a capture of the left side");
        Err(Diagnostic::error(span.clone(), message).with_help(help))
    }
}

fn virtual_key(name: &str, span: &Span) -> Result<VirtualKey, Diagnostic> {
    VirtualKey::from_kms_name(name).ok_or_else(|| {
        let error = Diagnostic::error(span.clone(), format!("unknown virtual key '{name}'"));
        match nearest_key_name(name) {
            Some(near) => error.with_help(format!("did you mean '{near}'?")),
            None => error,
        }
    })
}

fn nearest_key_name(name: &str) -> Option<&'static str> {
    (0..=u16::MAX)
        .filter_map(VirtualKey::from_u16)
        .map(VirtualKey::kms_name)
        .map(|candidate| (edit_distance(name, candidate), candidate))
        .filter(|(distance, _)| *distance <= 3)
        .min_by_key(|(distance, _)| *distance)
        .map(|(_, candidate)| candidate)
}

fn edit_distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut diagonal = row[0];
        row[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let above = row[j + 1];
            row[j + 1] = (above + 1)
                .min(row[j] + 1)
                .min(diagonal + usize::from(ca != *cb));
            diagonal = above;
        }
    }
    row[b.len()]
}

fn scalar(code: u32, span: &Span) -> Result<char, Diagnostic> {
    char::from_u32(code)
        .ok_or_else(|| Diagnostic::error(span.clone(), format!("U{code:04X} is not a character")))
}

/// Decodes escapes; unknown escapes keep their backslash.
fn escapes(raw: &str, span: &Span) -> Result<String, Diagnostic> {
    let mut out = String::new();
    let mut chars = raw.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some(kept @ ('\\' | '"' | '\'')) => out.push(kept),
            Some('u') => {
                let hex: String = chars.by_ref().take(4).collect();
                let code = if hex.len() == 4 {
                    u32::from_str_radix(&hex, 16).ok()
                } else {
                    None
                };
                let decoded = code.and_then(char::from_u32).ok_or_else(|| {
                    Diagnostic::error(span.clone(), format!("invalid escape \\u{hex}"))
                })?;
                out.push(decoded);
            }
            Some('x') => {
                let hex: String = chars.by_ref().take(2).collect();
                let byte = if hex.len() == 2 {
                    u8::from_str_radix(&hex, 16).ok()
                } else {
                    None
                };
                let decoded = byte.map(char::from).ok_or_else(|| {
                    Diagnostic::error(span.clone(), format!("invalid escape \\x{hex}"))
                })?;
                out.push(decoded);
            }
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    Ok(out)
}

fn option<'a>(options: &'a [(String, String)], key: &str) -> Option<&'a str> {
    options
        .iter()
        .rev()
        .find(|(known, _)| known == key)
        .map(|(_, value)| value.as_str())
}

const fn is_true(value: &str) -> bool {
    value.eq_ignore_ascii_case("true")
}

fn layout(options: &[(String, String)]) -> LayoutOptions {
    let mut layout = LayoutOptions::default();
    if let Some(value) = option(options, "TRACK_CAPSLOCK") {
        layout.track_caps = is_true(value);
    }
    if let Some(value) = option(options, "SMART_BACKSPACE") {
        layout.auto_bksp = is_true(value);
    }
    if let Some(value) = option(options, "EAT_ALL_UNUSED_KEYS") {
        layout.eat = is_true(value);
    }
    if let Some(value) = option(options, "US_LAYOUT_BASED") {
        layout.pos_based = is_true(value);
    }
    if let Some(value) = option(options, "TREAT_CTRL_ALT_AS_RALT") {
        layout.right_alt = is_true(value);
    }
    layout
}

fn info(
    options: &[(String, String)],
    resolver: &dyn Resolver,
) -> Result<Vec<InfoEntry>, Diagnostic> {
    let mut entries = Vec::new();
    for (key, id) in INFO_IDS {
        if let Some(value) = option(options, key) {
            entries.push(InfoEntry {
                id,
                data: value.as_bytes().to_vec(),
            });
        }
    }
    if let Some(icon) = option(options, "ICON") {
        entries.push(InfoEntry {
            id: *b"noci",
            data: icon_bytes(icon, resolver)?,
        });
    }
    Ok(entries)
}

fn icon_bytes(path: &str, resolver: &dyn Resolver) -> Result<Vec<u8>, Diagnostic> {
    let fail = |message: String| {
        Diagnostic::error(0..0, message).with_help("check @ICON in the file header")
    };
    let data = resolver
        .read(path)
        .ok_or_else(|| fail(format!("icon file not found: {path}")))?;
    let known = data.starts_with(b"\x89PNG\r\n\x1a\n")
        || data.starts_with(b"BM")
        || data.starts_with(&[0xFF, 0xD8, 0xFF]);
    if !known {
        return Err(fail(format!("icon must be PNG, BMP or JPEG: {path}")));
    }
    if data.len() > ICON_LIMIT {
        return Err(fail(format!("icon is larger than 1 MB: {path}")));
    }
    Ok(data)
}
