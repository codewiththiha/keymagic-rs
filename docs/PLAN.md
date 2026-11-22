# Roadmap

Derived from the [lab roadmap](https://01a1ff77-4137-7a69-a0fa-36ed3fcc921d.arena.site/#roadmap). Full lab steps and detailed evidence are in `/home/user/phases/` and `/home/user/progress.md` when available. A checkbox means all applicable completion gates have evidence, not merely that work has started.

## Current status — 2026-11-22

**Current phase: 00 — incomplete.** Working through its outstanding gates, not phase 01. At the user's direction unused product/platform stubs were removed; the workspace contains the implemented corpus-sync tool only. The engine and KM2 codec are not implemented.

| Card | Status | Remaining work |
| --- | --- | --- |
| p0-1 workspace | User-directed deviation | Unused crate stubs removed instead of copying the full lab tree. Final-SHA checks still required; planned product crates will be created with real work in their phases. |
| p0-2 agent and docs | Complete for the documentation card | Required documentation and the sourced virtual-key reference exist; platform manual tests remain future work. |
| p0-3 CI | User-directed deviation | Branch protection deferred at user's request; PR-oriented steps conflict with direct-main policy. The Clippy failure probe is retained on the implemented tool. |
| p0-4 corpus | In progress | [158 distinct scenarios pass](https://github.com/codewiththiha/keymagic-rs/actions/runs/44987503868): 60 test-derived and 98 exhaustive wildcard/boundary cases. User approved seeking licensed alternatives to the four upstream bundled KM2s. Loader, metadata and FFI tests remain reference sources, not event cases. |

[Development CI runs](https://github.com/codewiththiha/keymagic-rs/actions/workflows/ci.yml?query=branch%3Amain) show which commit was tested. Green CI currently validates tooling, not a working input method. Corpus source references are not runnable engine tests.

**Working rule:** Finish each phase end-to-end. Before ending, reread its entire phase file, compare every card, ordered step and completion gate against actual work and evidence, fill gaps, and repeat until complete. Ordinary remaining work is not a blocker. If an external dependency or a conflicting user instruction genuinely blocks an item, finish unblocked work and report the exact blocker. Update this status and the external progress record when work ends.

Critical path: 00 → 01 → 02 → 03 → 04 → 05 → (06 ∥ 07 ∥ 08) → 09. Do not start platform work before p2-4 differential CI is green.

## Phase 00 — Bootstrap the lab

- [ ] p0-1 — Create workspace skeleton
- [x] p0-2 — Write AGENTS.md + docs skeleton
- [ ] p0-3 — CI pipeline
- [ ] p0-4 — Vendor the behavioural corpus

## Phase 01 — Types + KM2 codec

- [ ] p1-1 — VirtualKey enum & tables
- [ ] p1-2 — Rule IR
- [ ] p1-3 — KM2 reader
- [ ] p1-4 — KM2 writer

## Phase 02 — The engine

- [ ] p2-1 — Engine state & buffer
- [ ] p2-2 — Matcher
- [ ] p2-3 — Rule application & recursion
- [ ] p2-4 — Differential harness + bench + fuzz

## Phase 03 — KMS compiler & CLI

- [ ] p3-1 — Lexer
- [ ] p3-2 — Parser + semantic checks
- [ ] p3-3 — Formatter
- [ ] p3-4 — CLI
- [ ] p3-5 — C FFI + header

## Phase 04 — Host layer

- [ ] p4-1 — Config + KeyboardStore
- [ ] p4-2 — Session policy
- [ ] p4-3 — IPC + Updater

## Phase 05 — egui GUI

- [ ] p5-1 — App shell + theme
- [ ] p5-2 — Keyboards page
- [ ] p5-3 — Tester + Editor pages
- [ ] p5-4 — Settings, Hotkeys, About, Tray

## Phase 06 — Linux (IBus → Wayland → KDE → Arch)

- [ ] p6-1 — IBus engine over zbus
- [ ] p6-2 — Wayland input-method-v2 + KDE
- [ ] p6-3 — Arch, Debian, Fedora packaging
- [ ] p6-4 — Fcitx5 shim (optional)

## Phase 07 — Windows TSF in Rust

- [ ] p7-1 — COM skeleton + registration
- [ ] p7-2 — Key handling + edit sessions
- [ ] p7-3 — Tray, HUD, language profiles, installer

## Phase 08 — macOS IMK in Rust

- [ ] p8-1 — IMK server & controller
- [ ] p8-2 — Input modes, menu, bundle, notarization

## Phase 09 — Release & 1.0

- [ ] p9-1 — Release workflow
- [ ] p9-2 — Docs + migration
- [ ] p9-3 — Hardening before 1.0
