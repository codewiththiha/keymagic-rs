# Roadmap

Derived from the [lab roadmap](https://01a1ff77-4137-7a69-a0fa-36ed3fcc921d.arena.site/#roadmap). Full lab steps and detailed evidence are in `/home/user/phases/` and `/home/user/progress.md` when available. A checkbox means all applicable completion gates have evidence, not merely that work has started.

## Current status: 2026-11-22

Phase 00 is complete under the user's adapted scope; Phase 01 is complete at `9f62008`; Phase 02 is complete at code SHA `2ac31e0`. Phase 02 CI, bench, and fuzz evidence is in `progress.md`. Branch protection remains deferred. Product crates are added as implemented. Platform phases may begin now that the p2-4 differential gate is green.

| Card | Status | Remaining work |
| --- | --- | --- |
| p0-1 workspace | Complete with user-directed deviation | Unused crate stubs removed instead of copying the full lab tree; planned product crates will be created with real work in their phases. [Self-hosted checks](https://github.com/codewiththiha/keymagic-rs/actions/runs/44988835071) and [Linux, macOS and Windows checks](https://github.com/codewiththiha/keymagic-rs/actions/runs/44988839936) passed at `14cb903`. |
| p0-2 agent and docs | Complete for the documentation card | Required documentation and the sourced virtual-key reference exist; platform manual tests remain future work. |
| p0-3 CI | Complete with user-directed deviations | Branch protection is optional and deferred; PR-oriented steps conflict with direct-main policy. Self-hosted CI includes an intentional Clippy failure probe; a hosted cross-platform verification passed. |
| p0-4 corpus | Complete with user-directed substitution | Four licensed alternative KMS/KM2 pairs are not the original upstream bundles. [Reference CI](https://github.com/codewiththiha/keymagic-rs/actions/runs/44988835011) passed 165 cases; [corpus sync check](https://github.com/codewiththiha/keymagic-rs/actions/runs/44988835071) passed at `14cb903`. My-Win's source cannot compile with the pinned KM3 compiler, although its binary smoke case passes. Loader, metadata and FFI tests remain reference sources. |

[Development CI runs](https://github.com/codewiththiha/keymagic-rs/actions/workflows/ci.yml?query=branch%3Amain) show which commit was tested. Green CI currently validates tooling, not a working input method. Corpus source references are not runnable engine tests.

Working rule: Finish each phase end-to-end. Before ending, reread its entire phase file, compare every card, ordered step and completion gate against actual work and evidence, fill gaps, and repeat until complete. Ordinary remaining work is not a blocker. If an external dependency or a conflicting user instruction genuinely blocks an item, finish unblocked work and report the exact blocker. Update this status and the external progress record when work ends.

Critical path: 00 → 01 → 02 → 03 → 04 → 05 → (06 ∥ 07 ∥ 08) → 09. Do not start platform work before p2-4 differential CI is green.

## Phase 00: Bootstrap the lab

- [x] p0-1: Workspace, with user-directed no-stubs deviation
- [x] p0-2: Write AGENTS.md + docs skeleton
- [x] p0-3: CI pipeline, with protection deferred
- [x] p0-4: Behavioural corpus, with user-approved alternatives

## Phase 01: Types + KM2 codec

- [x] p1-1: VirtualKey enum & tables
- [x] p1-2: Rule IR
- [x] p1-3: KM2 reader
- [x] p1-4: KM2 writer

Evidence and deviations: exact-SHA CI and fuzz are in `progress.md`. Deviations: the engine-level preserved behaviors are recorded but their executable tests belong to Phase 02; the codec tests were written after implementation, not test-first.

## Phase 02: The engine

- [x] p2-1: Engine state & buffer
- [x] p2-2: Matcher
- [x] p2-3: Rule application & recursion
- [x] p2-4: Differential harness + bench + fuzz

Evidence and deviations: the identity-cap test is in `5b8bab8` (runs 45014856703 and 45014856731). The ANY mutation check ran as run 45015076181 on the temporary branch `dcf6a14`, which was deleted afterwards. Exact-SHA CI, bench (3.64 M keys/s on Pyidaungsu MM, final head `31185cd`), and fuzz (657,231 runs in 601 s) for `2ac31e0` are in `progress.md`. Deviations: recursion cap 10 (upstream parity) instead of the card's 32; the benchmark uses Pyidaungsu MM at the user's request. The file has no license in its source repository, and the user chose to commit it anyway. See `corpus/licenses/Pyidaungsu-MM-NOTICE.md`; 52 of 54 engine-relevant upstream tests are mapped to corpus cases that run differentially against the pinned upstream engine; the other two are an assertion-free upstream TODO and a repeat of a mapped test. The recursion cap is 10, and `smallvec` is a required runtime dependency. Capture numbering follows upstream: `Key` and `State` elements yield no capture, though the card says each LHS element does. `auto_bksp` rewinds to the previous snapshot when one exists, not a single-character pop. The tests were added alongside the implementation, not written first. The card's "no allocation for ≤ 32 chars" item is verified by inline storage, not by a heap counter. The user accepted that evidence on 2026-11-22.

## Phase 03: KMS compiler & CLI

- [x] p3-1: Lexer
- [x] p3-2: Parser + semantic checks (byte identity on 41 of 41 corpus files, one of which is a whitespace-fixed copy of myWin; see progress.md)
- [x] p3-3: Formatter
- [x] p3-4: CLI (trycmd DoD met by in-house trycmd-style cases; see progress.md)
- [x] p3-5: C FFI + header

Phase 03 evidence is at code head `1b25aca`: Development `45035525104`, FFI platforms `45035525107`, corpus-verify `45035525132`. Byte identity covers 41 of 41 reference sources. The 41st is `myWin-fixed.kms`, a copy of upstream myWin with its 12 rule-level line continuations joined, because pinned KM3 rejects those continuations (ADR-27). The original `myWin.kms` still fails KM3 at line 224 and is not compared. The icon is upstream `myWin/myWin.png`, checked by `xtask corpus-sync --check`. CLI snapshots cover all nine subcommands (ADR-19). Card deviations, approved by the user on 2026-11-22: lexer hand-written (ADR-17); header from xtask, not build.rs (ADR-18); in-house trycmd-style snapshots (ADR-19); option settings carry their source span (ADR-24); no ariadne (ADR-25); tests added with the implementation (ADR-26); whitespace-fixed myWin copy (ADR-27); FFI guard scope (ADR-28); formatter keeps comments above their item (ADR-29). Implementation decisions, not card deviations: ADR-20 (approved `deny.toml` entries), ADR-21 (duplicated corpus replay), ADR-22 (`Diagnostic::to_json` removed), ADR-23 (`VirtualKey::ALL` added). The `doctor` and `install` stubs follow p3-4 step 1. Two stored km2 files (`myWin.km2` and `comprehensive_test.km2`) differ from our output; the pinned compiler matches our output for every reference source (see progress.md).

## Phase 04: Host layer

- [x] p4-1: Config + KeyboardStore (watcher and BMP→PNG restored with `image` and `notify`, approved 2026-11-22; Development `45046378062` success at `19b5fa5`; see progress.md)
- [x] p4-2: Session policy (42 matrix rows, `session_policy.rs`; Development `45044740802` success at `a81e285`; ADR-30 and ADR-31)
- [x] p4-3: IPC + Updater (socket transport and reconnect via `interprocess`, HTTP fetch via `ureq`, approved 2026-11-22; loopback on a real local socket and in memory; Development `45049655393` success at `188b1a0`; see progress.md)

Phase 04 evidence is in `progress.md`. Open items: Windows named pipe and macOS socket not run on a hosted runner, no OS focus verification (no adapters yet), reconnect after a dropped link untested. Update channel added with default `stable` and not yet read (ADR-36).

## Phase 05: egui GUI

- [x] p5-1: App shell + theme (windowed launch on ubuntu, macos, windows in `gui-launch` `45230830954` at `e808335`; shell kittest snapshots `shell_dark`, `shell_focus`, `shell_light`; nav rail icons drawn from strokes. See progress.md checkpoint 8.)
- [x] p5-2: Keyboards page (import test, `page_keyboards_empty` and `page_keyboards_wide` snapshots, context menu, reorder, bundled banner, filter test `filter_hides_keyboards_that_do_not_match`, hotkey chip test `hotkey_chip_shows_the_typed_shortcut`. Green on `9786d5e`. See progress.md checkpoint 9.)
- [x] p5-3: Tester + Editor pages (round-trip test, Record case, 150 ms check, Shift/AltGr layer test and snapshots, KMS colour test `kms_tokens_get_their_own_colors`, Malayalam picker test `malayalam_glyphs_are_included`. Green on `9786d5e`. See progress.md checkpoint 9.)
- [x] p5-4: Settings, Hotkeys, About, Tray. Done: HUD notice path (`hud.rs`, fake-IME test `tests/hud.rs`, in-window overlay), Linux probe checked against a real process on Ubuntu (`probe_sees_a_running_linux_card_process`, cross-platform `45755653077` on `e27cbbd`), Linux Fix on worker thread, About update check, tray IPC tests. Accepted deferrals (user sign-off 2026-11-26): Windows language profiles to p7-3 and macOS input-mode regeneration to p8-2. Manual check left to the user: run the Linux Fix buttons on a desktop (IBus, portal, KDE, Fcitx5). See progress.md checkpoint 11.)

## Phase 06: Linux (IBus → Wayland → KDE → Arch)

- [ ] p6-1: IBus engine over zbus
- [ ] p6-2: Wayland input-method-v2 + KDE
- [ ] p6-3: Arch, Debian, Fedora packaging
- [ ] p6-4: Fcitx5 shim (optional)

Phase 06 evidence: at code head `14f9147`, the Linux container typing test passes (`linux-ibus.yml` `45787551512`: `kk` types to `ကက` through IBus into GTK). Dev CI `45787117523` and cross-platform `45786291472` are green on the same head. p6-1 stays open: the GNOME manual typing check (gedit and Firefox) and the keyboard menu (step 3) are not done. Details are in `progress.md`.

At `cca1a30` the IBus engine also registers a keyboard menu (`RegisterProperties`). Dev CI `45794413268`, Linux typing `45794424916` and cross-platform `45794780711` are green on that head. Switching through the menu and the GNOME check are still open, so p6-1 stays unticked.

## Phase 07: Windows TSF in Rust

- [ ] p7-1: COM skeleton + registration
- [ ] p7-2: Key handling + edit sessions
- [ ] p7-3: Tray, HUD, language profiles, installer

## Phase 08: macOS IMK in Rust

- [ ] p8-1: IMK server & controller
- [ ] p8-2: Input modes, menu, bundle, notarization

## Phase 09: Release & 1.0

- [ ] p9-1: Release workflow
- [ ] p9-2: Docs + migration
- [ ] p9-3: Hardening before 1.0
