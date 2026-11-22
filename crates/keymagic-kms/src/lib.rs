//! KMS compiler: script text to keyboard IR, with diagnostics and formatting.
//!
//! No file-system access here; callers supply bytes through [`Resolver`].

pub mod ast;
pub mod diag;
pub mod format;
pub mod lexer;
pub mod lower;
pub mod parser;
pub mod resolve;

pub use diag::{Diagnostic, Severity};
pub use format::format;
pub use keymagic_types::Keyboard;
pub use resolve::Resolver;

/// Compiles `src` and its includes and icons, read through `resolver`.
#[doc = "# Errors\n\nReturns the error diagnostics. Warnings are reported by [`check`] only."]
pub fn compile(src: &str, resolver: &dyn Resolver) -> Result<Keyboard, Vec<Diagnostic>> {
    let script = resolve::load(src, resolver).map_err(|error| vec![error])?;
    lower::lower(&script, resolver).map_err(|error| vec![error])
}

/// Returns every diagnostic for `src`, errors first. Never fails.
#[must_use]
pub fn check(src: &str, resolver: &dyn Resolver) -> Vec<Diagnostic> {
    match resolve::load(src, resolver) {
        Err(error) => vec![error],
        Ok(script) => {
            let mut found = lower::warnings(&script);
            if let Err(error) = lower::lower(&script, resolver) {
                found.insert(0, error);
            }
            found
        }
    }
}
