//! Recursive-descent parser for one KMS file.
//! Follows the upstream grammar: pattern elements may omit '+'.

use crate::ast::{Include, Out, Pat, Rule, Script, Value, Variable};
use crate::diag::{Diagnostic, Span};
use crate::lexer::{self, Kind, Token};

/// Parses one file without following includes.
#[doc = "# Errors\n\nReturns the first syntax error."]
pub fn parse(src: &str) -> Result<Script, Diagnostic> {
    let tokens = lexer::lex(src)?.tokens;
    let mut parser = Parser {
        tokens,
        pos: 0,
        end: src.len(),
    };
    let mut script = Script {
        options: lexer::options(src),
        variables: Vec::new(),
        rules: Vec::new(),
        includes: Vec::new(),
    };
    while parser.pos < parser.tokens.len() {
        match parser.peek().cloned() {
            Some(Kind::Include) => script.includes.push(parser.include()?),
            Some(Kind::Var(_)) if parser.peek_at(1) == Some(&Kind::Equals) => {
                script.variables.push(parser.variable()?);
            }
            _ => script.rules.push(parser.rule()?),
        }
    }
    Ok(script)
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    end: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Kind> {
        self.tokens.get(self.pos).map(|token| &token.kind)
    }

    fn peek_at(&self, offset: usize) -> Option<&Kind> {
        self.tokens.get(self.pos + offset).map(|token| &token.kind)
    }

    fn here(&self) -> Span {
        self.tokens
            .get(self.pos)
            .map_or(self.end..self.end, |token| token.span.clone())
    }

    fn prev_end(&self) -> usize {
        self.pos
            .checked_sub(1)
            .and_then(|index| self.tokens.get(index))
            .map_or(0, |token| token.span.end)
    }

    fn since(&self, start: usize) -> Span {
        start..self.prev_end()
    }

    const fn advance(&mut self) {
        self.pos += 1;
    }

    fn expect(&mut self, want: &Kind, what: &str) -> Result<(), Diagnostic> {
        if self.peek() == Some(want) {
            self.advance();
            Ok(())
        } else {
            Err(self.unexpected(what))
        }
    }

    fn unexpected(&self, what: &str) -> Diagnostic {
        Diagnostic::error(
            self.here(),
            format!("expected {what}, found {}", describe(self.peek())),
        )
    }

    fn include(&mut self) -> Result<Include, Diagnostic> {
        let start = self.here().start;
        self.advance();
        self.expect(&Kind::LParen, "'('")?;
        let Some(Kind::Str(path)) = self.peek().cloned() else {
            return Err(self.unexpected("a quoted file name"));
        };
        self.advance();
        self.expect(&Kind::RParen, "')'")?;
        Ok(Include {
            path,
            span: self.since(start),
        })
    }

    fn variable(&mut self) -> Result<Variable, Diagnostic> {
        let start = self.here().start;
        let Some(Kind::Var(name)) = self.peek().cloned() else {
            return Err(self.unexpected("a variable name"));
        };
        self.advance();
        self.expect(&Kind::Equals, "'='")?;
        let mut value = Vec::new();
        while let Some(element) = self.value_element() {
            value.push(element);
            if self.peek() != Some(&Kind::Plus) {
                break;
            }
            self.advance();
            if !self.starts_value() {
                return Err(self.unexpected("a value after '+'"));
            }
        }
        if value.is_empty() {
            return Err(self.unexpected("a value after '='"));
        }
        Ok(Variable {
            name,
            value,
            span: self.since(start),
        })
    }

    fn starts_value(&self) -> bool {
        matches!(
            self.peek(),
            Some(Kind::Str(_) | Kind::Uni(_) | Kind::Var(_))
        )
    }

    fn value_element(&mut self) -> Option<Value> {
        let start = self.here().start;
        let element = match self.peek()?.clone() {
            Kind::Str(text) => Value::Str(text, self.here()),
            Kind::Uni(code) => Value::Uni(code, self.here()),
            Kind::Var(name) => Value::Var(name, self.here()),
            _ => return None,
        };
        self.advance();
        Some(match element {
            Value::Str(text, _) => Value::Str(text, start..self.prev_end()),
            Value::Uni(code, _) => Value::Uni(code, start..self.prev_end()),
            Value::Var(name, _) => Value::Var(name, start..self.prev_end()),
        })
    }

    fn rule(&mut self) -> Result<Rule, Diagnostic> {
        let start = self.here().start;
        let lhs = self.pattern()?;
        self.expect(&Kind::Arrow, "'=>'")?;
        let rhs = self.output()?;
        Ok(Rule {
            lhs,
            rhs,
            span: self.since(start),
        })
    }

    fn pattern(&mut self) -> Result<Vec<Pat>, Diagnostic> {
        let mut lhs = Vec::new();
        while starts_pattern(self.peek()) {
            lhs.push(self.pattern_element()?);
            if self.peek() == Some(&Kind::Plus) {
                self.advance();
                if !starts_pattern(self.peek()) {
                    return Err(self.unexpected("a pattern element after '+'"));
                }
            }
        }
        if lhs.is_empty() {
            return Err(self.unexpected("a pattern before '=>'"));
        }
        Ok(lhs)
    }

    fn pattern_element(&mut self) -> Result<Pat, Diagnostic> {
        let start = self.here().start;
        match self.peek().cloned() {
            Some(Kind::Str(text)) => {
                self.advance();
                Ok(Pat::Str(text, self.since(start)))
            }
            Some(Kind::Uni(code)) => {
                self.advance();
                Ok(Pat::Uni(code, self.since(start)))
            }
            Some(Kind::Var(name)) => {
                self.advance();
                if self.peek() != Some(&Kind::LBracket) {
                    return Ok(Pat::Var(name, self.since(start)));
                }
                self.advance();
                let wildcard = match self.peek() {
                    Some(Kind::Star) => Kind::Star,
                    Some(Kind::Caret) => Kind::Caret,
                    _ => return Err(self.unexpected("'*' or '^' inside [ ]")),
                };
                self.advance();
                self.expect(&Kind::RBracket, "']'")?;
                let span = self.since(start);
                Ok(if wildcard == Kind::Star {
                    Pat::AnyOf(name, span)
                } else {
                    Pat::NotAnyOf(name, span)
                })
            }
            Some(Kind::Any) => {
                self.advance();
                Ok(Pat::Any(self.since(start)))
            }
            Some(Kind::LAngle) => self.keys(start),
            Some(Kind::LParen) => {
                let name = self.state_name()?;
                Ok(Pat::State(name, self.since(start)))
            }
            Some(Kind::Null) => Err(Diagnostic::error(
                self.here(),
                "NULL is only allowed on the right side of '=>'",
            )),
            _ => Err(self.unexpected("a pattern element")),
        }
    }

    fn keys(&mut self, start: usize) -> Result<Pat, Diagnostic> {
        self.advance();
        let mut names = Vec::new();
        loop {
            let Some(Kind::Ident(name)) = self.peek().cloned() else {
                return Err(self.unexpected("a virtual key name"));
            };
            let span = self.here();
            self.advance();
            names.push((name, span));
            if self.peek() == Some(&Kind::Amp) {
                self.advance();
            } else {
                break;
            }
        }
        self.expect(&Kind::RAngle, "'>'")?;
        Ok(Pat::Keys(names, self.since(start)))
    }

    fn state_name(&mut self) -> Result<String, Diagnostic> {
        self.expect(&Kind::LParen, "'('")?;
        let Some(Kind::Str(name)) = self.peek().cloned() else {
            return Err(self.unexpected("a state name in quotes"));
        };
        self.advance();
        self.expect(&Kind::RParen, "')'")?;
        Ok(name)
    }

    fn output(&mut self) -> Result<Vec<Out>, Diagnostic> {
        let mut rhs = Vec::new();
        while starts_output(self.peek()) {
            rhs.push(self.output_element()?);
            if self.peek() != Some(&Kind::Plus) {
                break;
            }
            self.advance();
            if !starts_output(self.peek()) {
                return Err(self.unexpected("an output element after '+'"));
            }
        }
        if rhs.is_empty() {
            return Err(self.unexpected("an output after '=>'"));
        }
        Ok(rhs)
    }

    fn output_element(&mut self) -> Result<Out, Diagnostic> {
        let start = self.here().start;
        match self.peek().cloned() {
            Some(Kind::Str(text)) => {
                self.advance();
                Ok(Out::Str(text, self.since(start)))
            }
            Some(Kind::Uni(code)) => {
                self.advance();
                Ok(Out::Uni(code, self.since(start)))
            }
            Some(Kind::Var(name)) => {
                self.advance();
                if self.peek() != Some(&Kind::LBracket) {
                    return Ok(Out::Var(name, self.since(start)));
                }
                self.advance();
                let Some(Kind::BackRef(number)) = self.peek().cloned() else {
                    return Err(self.unexpected("a back-reference such as $1 inside [ ]"));
                };
                self.advance();
                self.expect(&Kind::RBracket, "']'")?;
                Ok(Out::Indexed(name, number, self.since(start)))
            }
            Some(Kind::BackRef(number)) => {
                self.advance();
                Ok(Out::Ref(number, self.since(start)))
            }
            Some(Kind::Null) => {
                self.advance();
                Ok(Out::Null(self.since(start)))
            }
            Some(Kind::LParen) => {
                let name = self.state_name()?;
                Ok(Out::State(name, self.since(start)))
            }
            Some(Kind::Any) => Err(Diagnostic::error(
                self.here(),
                "ANY is only allowed on the left side of '=>'",
            )),
            Some(Kind::LAngle) => Err(Diagnostic::error(
                self.here(),
                "virtual keys are only allowed on the left side of '=>'",
            )),
            _ => Err(self.unexpected("an output element")),
        }
    }
}

const fn starts_pattern(kind: Option<&Kind>) -> bool {
    matches!(
        kind,
        Some(
            Kind::Str(_)
                | Kind::Uni(_)
                | Kind::Var(_)
                | Kind::Any
                | Kind::LAngle
                | Kind::LParen
                | Kind::Null
        )
    )
}

const fn starts_output(kind: Option<&Kind>) -> bool {
    matches!(
        kind,
        Some(
            Kind::Str(_)
                | Kind::Uni(_)
                | Kind::Var(_)
                | Kind::BackRef(_)
                | Kind::Null
                | Kind::LParen
                | Kind::Any
                | Kind::LAngle
        )
    )
}

fn describe(kind: Option<&Kind>) -> String {
    match kind {
        None => "end of file".to_owned(),
        Some(Kind::Str(_)) => "a string".to_owned(),
        Some(Kind::Ident(name)) => format!("'{name}'"),
        Some(Kind::Var(name)) => format!("'${name}'"),
        Some(Kind::BackRef(number)) => format!("'${number}'"),
        Some(Kind::Uni(code)) => format!("'U{code:04X}'"),
        Some(other) => format!("'{}'", symbol(other)),
    }
}

const fn symbol(kind: &Kind) -> &'static str {
    match kind {
        Kind::Include => "include",
        Kind::Any => "ANY",
        Kind::Null => "NULL",
        Kind::Arrow => "=>",
        Kind::Plus => "+",
        Kind::Amp => "&",
        Kind::Equals => "=",
        Kind::LParen => "(",
        Kind::RParen => ")",
        Kind::LBracket => "[",
        Kind::RBracket => "]",
        Kind::LAngle => "<",
        Kind::RAngle => ">",
        Kind::Star => "*",
        Kind::Caret => "^",
        Kind::Var(_) | Kind::BackRef(_) | Kind::Uni(_) | Kind::Str(_) | Kind::Ident(_) => "value",
    }
}
