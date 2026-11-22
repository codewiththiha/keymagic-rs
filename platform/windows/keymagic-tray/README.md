# keymagic-windows-tray

**Status:** Placeholder only; no product functionality implemented.

**Planned responsibility:** Thin launcher: on Windows the egui GUI binary runs as the tray + HUD + IPC server; this crate adds Windows-only bits (elevation helper for language profiles, HUD layered window, RegisterHotKey).

**Path:** `platform/windows/keymagic-tray` · **Layer:** platform · **Kind:** bin

Read [AGENTS.md](../../../AGENTS.md) and the relevant task card before implementing. Rust 2024, strict lints and the no-unsafe rule apply to this crate. Proposed functionality must be tested and is not yet guaranteed feasible under that rule.
