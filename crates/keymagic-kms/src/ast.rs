//! Syntax tree for one KMS file. Each element keeps its byte span.

use crate::diag::Span;

/// One parsed file. Lists keep source order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Script {
    /// `@NAME = "value"` pairs; the last value for a key wins.
    pub options: Vec<(String, String)>,
    pub variables: Vec<Variable>,
    pub rules: Vec<Rule>,
    pub includes: Vec<Include>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Include {
    pub path: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variable {
    pub name: String,
    pub value: Vec<Value>,
    pub span: Span,
}

/// A value element. String spans include the quotes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Str(String, Span),
    Uni(u32, Span),
    Var(String, Span),
}

/// A left-side element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pat {
    Str(String, Span),
    Uni(u32, Span),
    Var(String, Span),
    AnyOf(String, Span),
    NotAnyOf(String, Span),
    Any(Span),
    Keys(Vec<(String, Span)>, Span),
    State(String, Span),
}

/// A right-side element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Out {
    Str(String, Span),
    Uni(u32, Span),
    Var(String, Span),
    Indexed(String, usize, Span),
    Ref(usize, Span),
    Null(Span),
    State(String, Span),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    pub lhs: Vec<Pat>,
    pub rhs: Vec<Out>,
    pub span: Span,
}
