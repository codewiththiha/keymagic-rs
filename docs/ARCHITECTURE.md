# Architecture: engine first

Status: design and stub workspace. None of the input-method functionality below is implemented yet. The lab is the design source; [AGENTS.md](../AGENTS.md) and the user's current instructions override any conflicting lab sketches.

## Dependency boundaries

`types ← km2/engine ← host ← platform/gui`

- `keymagic-types` owns virtual keys, modifiers, rule IR, layout options and hotkeys.
- `keymagic-km2` owns KM2 encoding/decoding and conversion between disk's 1-based variable indices and internal 0-based indices.
- `keymagic-engine` is a deterministic, pure key-event transformer; no IO, threads, globals or unsafe.
- `keymagic-kms` compiles/formats scripts; `keymagic-ffi` is the proposed C ABI boundary; `keymagic-cli` provides commands.
- `keymagic-host` centralizes keyboard storage, config, hotkeys, session/commit policy, IPC, updates and integration generation.
- `keymagic-gui` hosts egui pages; view models avoid egui dependencies. Platform crates translate events and apply `HostAction`, not policy.

## Intended event flow

OS input → platform key mapping → `KeyEvent` → host `Session` policy → engine `process` → `HostAction` → platform preedit/commit/direct edit. GUI and IME processes communicate through host IPC. Rule precedence, transient states, recursion and composing-buffer behavior must be established from the pinned upstream implementation, tests and corpus, not this overview alone.

## Safety and feasibility

Rust 2024 and `unsafe_code = forbid` apply to every workspace crate, without exceptions, per the user's instruction. The lab proposes C ABI exports and native OS integration; some may require unsafe Rust under the actual APIs. Feasibility is unproven: stop and report any such conflict instead of weakening the policy or asserting that the integration works.

## References

- [Lab](https://01a1ff77-4137-7a69-a0fa-36ed3fcc921d.arena.site/) architecture and crate specs.
- [KeyMagic 3](https://github.com/thantthet/keymagic-3) and its tests are the compatibility reference.
- [Engine reference](ENGINE_LOGIC.md), [KM2 reference](KM2_FORMAT.md), [KMS reference](KMS_FORMAT.md), [virtual keys](VIRTUAL_KEYS.md).
- [ADR 0001](ADR/0001-engine-first.md).
