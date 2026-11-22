# keymagic-ffi

**Status:** Placeholder only; no product functionality implemented.

**Planned responsibility:** Stable C ABI over engine + km2 + kms, with cbindgen header. Keeps KM3's `keymagic_*` symbol names so third-party integrations (Java, .NET, Fcitx5 shim) keep working.

**Path:** `crates/keymagic-ffi` · **Layer:** tooling · **Kind:** cdylib

Read [AGENTS.md](../../AGENTS.md) and the relevant task card before implementing. Rust 2024, strict lints and the no-unsafe rule apply to this crate. Proposed functionality must be tested and is not yet guaranteed feasible under that rule.
