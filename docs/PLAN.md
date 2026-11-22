# Roadmap

Derived from the [lab roadmap](https://01a1ff77-4137-7a69-a0fa-36ed3fcc921d.arena.site/#roadmap). Full lab steps and detailed evidence are in `/home/user/phases/` and `/home/user/progress.md` when available. A checkbox means all applicable completion gates have evidence, not merely that work has started.

## Current status — 2026-11-22

**Phase 00 is complete under the user's adapted scope; Phase 01 is complete at `9f62008`; Phase 02 is complete at code SHA `2ac31e0`.** Phase 02 CI, bench, and fuzz evidence is in `progress.md`. Branch protection remains deferred. Product crates are added as implemented. Platform phases may begin now that the p2-4 differential gate is green.

| Card | Status | Remaining work |
| --- | --- | --- |
| p0-1 workspace | Complete with user-directed deviation | Unused crate stubs removed instead of copying the full lab tree; planned product crates will be created with real work in their phases. [Self-hosted checks](https://github.com/codewiththiha/keymagic-rs/actions/runs/44988835071) and [Linux, macOS and Windows checks](https://github.com/codewiththiha/keymagic-rs/actions/runs/44988839936) passed at `14cb903`. |
| p0-2 agent and docs | Complete for the documentation card | Required documentation and the sourced virtual-key reference exist; platform manual tests remain future work. |
| p0-3 CI | Complete with user-directed deviations | Branch protection is optional and deferred; PR-oriented steps conflict with direct-main policy. Self-hosted CI includes an intentional Clippy failure probe; a hosted cross-platform verification passed. |
| p0-4 corpus | Complete with user-directed substitution | Four licensed alternative KMS/KM2 pairs are not the original upstream bundles. [Reference CI](https://github.com/codewiththiha/keymagic-rs/actions/runs/44988835011) passed 165 cases; [corpus sync check](https://github.com/codewiththiha/keymagic-rs/actions/runs/44988835071) passed at `14cb903`. My-Win's source cannot compile with the pinned KM3 compiler, although its binary smoke case passes. Loader, metadata and FFI tests remain reference sources. |

[Development CI runs](https://github.com/codewiththiha/keymagic-rs/actions/workflows/ci.yml?query=branch%3Amain) show which commit was tested. Green CI currently validates tooling, not a working input method. Corpus source references are not runnable engine tests.

**Working rule:** Finish each phase end-to-end. Before ending, reread its entire phase file, compare every card, ordered step and completion gate against actual work and evidence, fill gaps, and repeat until complete. Ordinary remaining work is not a blocker. If an external dependency or a conflicting user instruction genuinely blocks an item, finish unblocked work and report the exact blocker. Update this status and the external progress record when work ends.

Critical path: 00 → 01 → 02 → 03 → 04 → 05 → (06 ∥ 07 ∥ 08) → 09. Do not start platform work before p2-4 differential CI is green.

## Phase 00 — Bootstrap the lab

- [x] p0-1 — Workspace, with user-directed no-stubs deviation
- [x] p0-2 — Write AGENTS.md + docs skeleton
- [x] p0-3 — CI pipeline, with protection deferred
- [x] p0-4 — Behavioural corpus, with user-approved alternatives

## Phase 01 — Types + KM2 codec

- [x] p1-1 — VirtualKey enum & tables
- [x] p1-2 — Rule IR
- [x] p1-3 — KM2 reader
- [x] p1-4 — KM2 writer

Evidence and deviations: exact-SHA CI and fuzz are in `progress.md`. Deviations: the engine-level preserved behaviors are recorded but their executable tests belong to Phase 02; the codec tests were written after implementation, not test-first.

## Phase 02 — The engine

- [x] p2-1 — Engine state & buffer
- [x] p2-2 — Matcher
- [x] p2-3 — Rule application & recursion
- [x] p2-4 — Differential harness + bench + fuzz

Evidence and deviations: exact-SHA CI, bench (1.42 M keys/s), and fuzz (657,231 runs in 601 s) for `2ac31e0` are in `progress.md`. Deviations: recursion cap 10 (upstream parity) instead of the card's 32; the benchmark uses the licensed myWin alternative; upstream tests are covered by the corpus and hand-written behavior tests, not ported one-for-one.

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
