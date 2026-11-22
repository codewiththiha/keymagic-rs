# ADR 0001: Engine-first architecture

Status: proposed foundation; platform feasibility remains unverified.

## Context

KeyMagic 3 combines its Rust engine, codec and FFI and implements input-method host behavior in multiple platform languages. The lab proposes a portable engine plus a shared host-policy layer to prevent platform drift.

## Decision

- Make `keymagic-engine` a pure library; put storage, session and commit policy in `keymagic-host`. Platform adapters translate events and apply host actions.
- Keep the proposed C ABI boundary separate from the engine. Use a provenance-documented corpus and pinned-upstream differential tests to establish behavior.
- Apply Rust 2024, strict lints and the user's no unsafe Rust anywhere requirement to every crate, including platform and FFI. Do not make a silent exception.

## Consequences and unresolved risk

Engine semantics become independently testable, but the design introduces boundaries and integration work. It is not yet established that all desired C ABI exports, Windows COM or macOS APIs can be implemented with zero unsafe Rust. Escalate any demonstrated incompatibility to the user before changing the constraint. Cross-platform and manual integration behavior is not established by stub compilation.
