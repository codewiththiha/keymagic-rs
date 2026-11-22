# Roadmap

Derived from the [lab roadmap](https://01a1ff77-4137-7a69-a0fa-36ed3fcc921d.arena.site/#roadmap). Full local notes, evidence and status live outside this repo in `/home/user/phases/` and `/home/user/progress.md`. Check them each turn. All boxes below are initially unchecked, not proof of completion.

Critical path: 00 → 01 → 02 → 03 → 04 → 05 → (06 ∥ 07 ∥ 08) → 09. Do not start platform work before p2-4 differential CI is green.

## Phase 00 — Bootstrap the lab

- [ ] p0-1 — Create workspace skeleton
- [ ] p0-2 — Write AGENTS.md + docs skeleton
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
