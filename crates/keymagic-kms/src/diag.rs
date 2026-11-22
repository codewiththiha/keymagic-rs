//! Diagnostics shared by the lexer, parser, lowering and the CLI.

use std::ops::Range;

/// A byte range in the source text.
pub type Span = Range<usize>;

/// How serious a diagnostic is. Warnings never stop compilation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

/// One problem found in a script.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: Severity,
    /// Name of the file the span points into; empty for the root script.
    pub file: String,
    pub span: Span,
    pub message: String,
    pub help: Option<String>,
}

impl Diagnostic {
    #[must_use]
    pub fn error(span: Span, message: impl Into<String>) -> Self {
        Self::new(Severity::Error, span, message)
    }

    #[must_use]
    pub fn warning(span: Span, message: impl Into<String>) -> Self {
        Self::new(Severity::Warning, span, message)
    }

    fn new(severity: Severity, span: Span, message: impl Into<String>) -> Self {
        Self {
            severity,
            file: String::new(),
            span,
            message: message.into(),
            help: None,
        }
    }

    #[must_use]
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    #[must_use]
    pub fn in_file(mut self, file: &str) -> Self {
        file.clone_into(&mut self.file);
        self
    }

    /// Renders the message, location, and root-file excerpt.
    #[must_use]
    pub fn render(&self, src: &str, root: &str) -> String {
        let label = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        let file = if self.file.is_empty() {
            root
        } else {
            self.file.as_str()
        };
        let (line, col, text) = excerpt_position(src, self.span.start);
        let mut out = format!("{label}: {}\n --> {file}:{line}:{col}\n", self.message);
        if self.file.is_empty() {
            out = format!("{out}{line:>4} | {text}\n     | {}^\n", " ".repeat(col - 1));
        }
        if let Some(help) = &self.help {
            out = format!("{out}help: {help}\n");
        }
        out
    }
}

impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Diagnostic {}

/// Line, column and text of `offset`; past the end, the last line.
fn excerpt_position(src: &str, offset: usize) -> (usize, usize, &str) {
    let (line, col) = line_col(src, offset);
    if let Some(text) = src.lines().nth(line - 1) {
        return (line, col, text);
    }
    let text = src.lines().last().unwrap_or("");
    (src.lines().count().max(1), text.chars().count() + 1, text)
}

fn line_col(src: &str, offset: usize) -> (usize, usize) {
    let before = src.get(..offset).unwrap_or(src);
    let line = before.matches('\n').count() + 1;
    let line_start = before.rfind('\n').map_or(0, |index| index + 1);
    let col = before[line_start..].chars().count() + 1;
    (line, col)
}
