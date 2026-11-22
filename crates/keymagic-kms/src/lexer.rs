//! KMS tokenizer. Skips comments and continuations; keeps comment spans.

use crate::ast::Setting;
use crate::diag::{Diagnostic, Span};

/// Token kinds. Variable names drop the `$`; back-references keep their number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    Include,
    Any,
    Null,
    Arrow,
    Plus,
    Amp,
    Equals,
    LParen,
    RParen,
    LBracket,
    RBracket,
    LAngle,
    RAngle,
    Star,
    Caret,
    Var(String),
    BackRef(usize),
    Uni(u32),
    /// Raw contents between the quotes; escapes are processed later.
    Str(String),
    Ident(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: Kind,
    pub span: Span,
}

/// Tokens plus the span of every comment, in source order.
#[derive(Debug)]
pub struct Lexed {
    pub tokens: Vec<Token>,
    pub comments: Vec<Span>,
}

/// Splits `src` into tokens.
#[doc = "# Errors\n\nReturns the first character or literal that is not valid KMS."]
pub fn lex(src: &str) -> Result<Lexed, Diagnostic> {
    let bytes = src.as_bytes();
    let mut lexed = Lexed {
        tokens: Vec::new(),
        comments: Vec::new(),
    };
    let mut pos = 0;
    while pos < bytes.len() {
        let rest = &src[pos..];
        let byte = bytes[pos];
        if byte.is_ascii_whitespace() {
            pos += 1;
        } else if rest.starts_with("//") {
            let end = rest.find('\n').map_or(src.len(), |index| pos + index);
            lexed.comments.push(pos..end);
            pos = end;
        } else if let Some(body) = rest.strip_prefix("/*") {
            let Some(close) = body.find("*/") else {
                return Err(Diagnostic::error(pos..src.len(), "unterminated comment")
                    .with_help("close the comment with */"));
            };
            let end = pos + close + 4;
            lexed.comments.push(pos..end);
            pos = end;
        } else if byte == b'\\' {
            pos = continuation(src, pos)?;
        } else if byte == b'"' || byte == b'\'' {
            let end = string_end(bytes, pos)?;
            let text = src[pos + 1..end - 1].to_owned();
            lexed.tokens.push(Token {
                kind: Kind::Str(text),
                span: pos..end,
            });
            pos = end;
        } else if byte == b'$' {
            let (kind, end) = dollar(src, pos)?;
            lexed.tokens.push(Token {
                kind,
                span: pos..end,
            });
            pos = end;
        } else if byte.is_ascii_alphabetic() || byte == b'_' {
            let len = rest
                .bytes()
                .take_while(|b| b.is_ascii_alphanumeric() || *b == b'_')
                .count();
            let end = pos + len;
            lexed.tokens.push(Token {
                kind: word_kind(&src[pos..end]),
                span: pos..end,
            });
            pos = end;
        } else if let Some((kind, len)) = symbol(rest) {
            lexed.tokens.push(Token {
                kind,
                span: pos..pos + len,
            });
            pos += len;
        } else {
            let Some(ch) = rest.chars().next() else {
                break;
            };
            let message = if byte == b'@' {
                "'@' options must be inside a comment".to_owned()
            } else {
                format!("unexpected character '{ch}'")
            };
            return Err(Diagnostic::error(pos..pos + ch.len_utf8(), message));
        }
    }
    Ok(lexed)
}

#[doc = "Options as the upstream compiler reads them.\n\nThe first block comment, then each // line. Later pairs win."]
#[must_use]
pub fn options(src: &str) -> Vec<Setting> {
    let mut found = Vec::new();
    let block = src
        .find("/*")
        .and_then(|start| src[start..].find("*/").map(|end| start..start + end + 2));
    if let Some(span) = block {
        found.extend(settings_in(src, span));
    }
    let mut start = 0;
    for line in src.split_inclusive('\n') {
        let end = start + line.len();
        let trimmed = line.trim();
        if trimmed.starts_with("//") && trimmed.contains('@') {
            found.extend(settings_in(src, start..end));
        }
        start = end;
    }
    found
}

fn settings_in(src: &str, range: Span) -> Vec<Setting> {
    let mut out = Vec::new();
    let mut line_start = range.start;
    for raw in src[range].split_inclusive('\n') {
        let start = line_start;
        line_start += raw.len();
        let text = raw.trim();
        let content = text
            .trim_start_matches("/*")
            .trim_end_matches("*/")
            .trim_start_matches("//")
            .trim();
        let Some(at) = content.find('@') else {
            continue;
        };
        let rest = &content[at + 1..];
        let Some(eq) = rest.find('=') else {
            continue;
        };
        let lead = raw.len() - raw.trim_start().len();
        out.push(Setting {
            key: rest[..eq].trim().to_owned(),
            value: rest[eq + 1..]
                .trim()
                .trim_matches('"')
                .trim_matches('\'')
                .to_owned(),
            span: start + lead..start + lead + text.len(),
        });
    }
    out
}

fn continuation(src: &str, pos: usize) -> Result<usize, Diagnostic> {
    let after = src[pos + 1..].trim_start_matches([' ', '\t']);
    let body = after
        .strip_prefix("\r\n")
        .or_else(|| after.strip_prefix('\n'));
    body.map(|rest| src.len() - rest.len()).ok_or_else(|| {
        Diagnostic::error(pos..pos + 1, "'\\' must be the last character on its line")
    })
}

fn string_end(bytes: &[u8], start: usize) -> Result<usize, Diagnostic> {
    let quote = bytes[start];
    let mut index = start + 1;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' => index += 2,
            byte if byte == quote => return Ok(index + 1),
            _ => index += 1,
        }
    }
    Err(Diagnostic::error(start..bytes.len(), "unterminated string")
        .with_help("close the string with the same quote character"))
}

fn dollar(src: &str, pos: usize) -> Result<(Kind, usize), Diagnostic> {
    let rest = &src[pos + 1..];
    let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
    if digits > 0 {
        let end = pos + 1 + digits;
        let number = src[pos + 1..end]
            .parse::<usize>()
            .map_err(|_| Diagnostic::error(pos..end, "back-reference number is too large"))?;
        return Ok((Kind::BackRef(number), end));
    }
    let starts_name = rest
        .bytes()
        .next()
        .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_');
    if !starts_name {
        return Err(
            Diagnostic::error(pos..pos + 1, "'$' must be followed by a name or a number")
                .with_help("names like $vowel, back-references like $1"),
        );
    }
    let len = rest
        .bytes()
        .take_while(|b| b.is_ascii_alphanumeric() || *b == b'_')
        .count();
    let end = pos + 1 + len;
    Ok((Kind::Var(src[pos + 1..end].to_owned()), end))
}

fn word_kind(word: &str) -> Kind {
    match word {
        "include" => Kind::Include,
        "ANY" => Kind::Any,
        "NULL" | "null" => Kind::Null,
        _ => unicode_literal(word).map_or_else(|| Kind::Ident(word.to_owned()), Kind::Uni),
    }
}

/// `U` or `u` followed by exactly four hex digits.
fn unicode_literal(word: &str) -> Option<u32> {
    let hex = word.strip_prefix(['U', 'u'])?;
    if hex.len() == 4 && hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        u32::from_str_radix(hex, 16).ok()
    } else {
        None
    }
}

fn symbol(rest: &str) -> Option<(Kind, usize)> {
    if rest.starts_with("=>") {
        return Some((Kind::Arrow, 2));
    }
    let kind = match *rest.as_bytes().first()? {
        b'+' => Kind::Plus,
        b'&' => Kind::Amp,
        b'=' => Kind::Equals,
        b'(' => Kind::LParen,
        b')' => Kind::RParen,
        b'[' => Kind::LBracket,
        b']' => Kind::RBracket,
        b'<' => Kind::LAngle,
        b'>' => Kind::RAngle,
        b'*' => Kind::Star,
        b'^' => Kind::Caret,
        _ => return None,
    };
    Some((kind, 1))
}
