//! File reads for `include(...)` and `@ICON`, supplied by the caller.

use crate::ast::Script;
use crate::diag::Diagnostic;
use crate::parser::parse;

/// Supplies file bytes. Paths are relative to the root file's directory.
pub trait Resolver {
    /// Returns the bytes at `path`, or `None` when the file cannot be read.
    fn read(&self, path: &str) -> Option<Vec<u8>>;
}

/// Parses `src` and appends each include's variables and rules.
#[doc = "# Errors\n\nReturns the first syntax error, unreadable include, or circular include."]
pub fn load(src: &str, resolver: &dyn Resolver) -> Result<Script, Diagnostic> {
    expand(src, "", &mut Vec::new(), resolver)
}

fn expand(
    src: &str,
    path: &str,
    stack: &mut Vec<String>,
    resolver: &dyn Resolver,
) -> Result<Script, Diagnostic> {
    let src = src.strip_prefix('\u{feff}').unwrap_or(src);
    let mut script = parse(src).map_err(|error| error.in_file(path))?;
    for include in std::mem::take(&mut script.includes) {
        let target = join(path, &include.path);
        if stack.contains(&target) {
            return Err(
                Diagnostic::error(include.span, format!("circular include of '{target}'"))
                    .in_file(path),
            );
        }
        let bytes = resolver.read(&target).ok_or_else(|| {
            Diagnostic::error(
                include.span.clone(),
                format!("cannot read included file '{target}'"),
            )
            .in_file(path)
        })?;
        let text = String::from_utf8(bytes).map_err(|_| {
            Diagnostic::error(0..0, format!("'{target}' is not UTF-8")).in_file(&target)
        })?;
        stack.push(target.clone());
        let nested = expand(&text, &target, stack, resolver)?;
        stack.pop();
        script.variables.extend(nested.variables);
        script.rules.extend(nested.rules);
    }
    Ok(script)
}

fn join(base: &str, child: &str) -> String {
    let dir = base.rsplit_once('/').map_or("", |(dir, _)| dir);
    if dir.is_empty() {
        child.to_owned()
    } else {
        format!("{dir}/{child}")
    }
}
