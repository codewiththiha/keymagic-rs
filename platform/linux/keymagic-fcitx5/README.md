# keymagic-linux-fcitx5

**Status:** Placeholder only; no product functionality implemented.

**Planned responsibility:** OPTIONAL. Fcitx5 only loads C++ addons, so this is the single sanctioned non-Rust file: a ~200-line C++ shim calling keymagic-ffi. Feature-gated, built only when fcitx5 headers exist.

**Path:** `platform/linux/keymagic-fcitx5` · **Layer:** platform · **Kind:** cdylib

Read [AGENTS.md](../../../AGENTS.md) and the relevant task card before implementing. Rust 2024, strict lints and the no-unsafe rule apply to this crate. Proposed functionality must be tested and is not yet guaranteed feasible under that rule.
