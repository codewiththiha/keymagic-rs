//! Deterministic key-to-text engine for KeyMagic keyboard layouts.
//!
//! The engine performs no I/O, owns no threads and keeps no global state.

mod apply;
mod input;
mod matcher;
mod process;
mod record;
mod result;
mod rules;
mod states;
mod text;

pub use input::{KeyEvent, Modifiers};
pub use keymagic_types::VirtualKey;
pub use process::{Engine, MAX_RECURSION_DEPTH};
pub use record::{Trace, TraceStep};
pub use result::{Action, Output};
pub use text::{ComposingBuffer, Edit, INLINE_EDIT_CHARS};
