# ADR 0002: Dependency-internal unsafe

Status: accepted by the user for Phase 06 Linux crates.

## Context

Workspace code forbids `unsafe` (`unsafe_code = "forbid"`). Some approved dependencies contain `unsafe` internally. The IBus keysym table uses `xkbcommon`, which wraps libxkbcommon FFI. The Wayland probe uses `wayland-client`, whose default `wayland-backend` is written with `unsafe` internally. A safe public API does not require `unsafe` from workspace code.

## Decision

- Dependency-internal `unsafe` is allowed when the dependency exposes a safe API and workspace code calls only that API.
- Workspace code and build scripts keep `unsafe` forbidden. Do not add `unsafe` blocks, `unsafe fn`, or `allow(unsafe_code)` in workspace crates.
- Each dependency that contains `unsafe` must be listed below with the reason it is needed.
- Adding a dependency with `unsafe` internals needs user approval, as before.

## Approved exceptions

| Dependency | Version | Reason |
| --- | --- | --- |
| `xkbcommon` | 0.9.0 | Keysym-to-character mapping for the IBus engine (libxkbcommon FFI). |
| `wayland-client` | 0.31.15 | Wayland client for the input-method probe. |
| `wayland-backend` | 0.3.17 | Default Rust backend of `wayland-client`. |
| `wayland-protocols-misc` | 0.3.12 | Generated input-method-v2 bindings from wayland-scanner. |

## Scope

The table lists the Phase 06 Linux additions. Earlier approved dependencies, such as tokio, zbus and the GUI stack, may also contain `unsafe` internally. They are covered by the same rule, but this ADR does not re-audit them.

## Consequences

The workspace lint does not prove that dependencies are free of `unsafe`. Reviews must check that new dependencies with `unsafe` are listed here before they are merged. This ADR does not allow `unsafe` in workspace code.
