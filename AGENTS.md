# AGENTS.md: keymagic-rs

Read this file before changing the project. The [lab](https://01a1ff77-4137-7a69-a0fa-36ed3fcc921d.arena.site/) supplies the roadmap; the user's instructions below override conflicting lab examples. External planning notes live in `/home/user/agent-notes.md`, `/home/user/progress.md`, and `/home/user/phases/` when available.

## Every turn

1. Read the external notes, progress, relevant phase file and repo plan. Inspect Git status and actual code; review CI logs for the current SHA. Ask for confirmation of user-only manual steps. Never infer completion from a plan, checkbox, old run or existing code.
2. Complete the current phase end-to-end. Before ending, reread its entire phase file, compare every card, ordered step and completion criterion against actual code and evidence, fill gaps and repeat until complete. Ordinary remaining work is not a blocker. If an external dependency or a user-instruction conflict genuinely blocks an item, finish unblocked work and report the exact blocker. Inspect upstream sources, identify three behaviors to preserve, write tests first, validate, and keep `docs/PLAN.md` and external progress current.
3. Develop directly on `main`, committing and pushing regular commits. Do not create branches or PRs unless the user explicitly asks. At session end, squash related very small commits; fold failed-CI commits into their fixes. The user authorizes force-pushing these rewrites with an exact lease, without overwriting unrelated remote work. Validate the final SHA. Other force pushes need separate authorization; old GitHub Actions records may remain visible.

## Execution and Git

- Run all Cargo, rustc, Rust formatting, tests, clippy, cargo-deny, fuzz and build commands only on the self-hosted GitHub runner, never in the agent workspace. Inspect the exact run's logs. Use the manual self-hosted patch or lockfile workflows to retrieve generated changes.
- Git, Python and static checks may run locally. GitHub-hosted runners are only for releases or genuinely necessary cross-platform checks; never launch a matrix routinely. Never run untrusted fork code on the self-hosted runner.
- Rust 2024; all workspace Rust code forbids unsafe, including FFI and platform crates. Do not weaken this rule. If an API requires unsafe Rust, report the conflict before proceeding. This lint does not prove that dependencies or cfg-disabled paths are unsafe-free.
- CI uses `RUSTFLAGS=-D warnings`, `RUSTDOCFLAGS=-D warnings`, strict Clippy all/pedantic/nursery, and denies unwrap/expect. It runs locked metadata, rustfmt check, all-targets check, workspace build, Clippy, tests and cargo-deny. Fix failures; do not claim a green build until the current SHA's actual run is green.
- Every commit author is `codewiththiha <kt007441@gmail.com>`; restore local Git config if needed. Use a Conventional Commit such as `feat: add key parser` or `fix(engine): handle input`, on one line of at most 50 Unicode characters. Check with `scripts/check_commit_messages.py`; CI also checks author and message ranges. Never expose the external `.env` or token in files, URLs or logs.

## Code style and comments

- Use default rustfmt with `max_width = 100`, and keep Clippy clean with `-D warnings`. No TODO/FIXME markers, `todo!`, `unimplemented!`, placeholder functions or scaffolding for work not being implemented now. At the user's direction unused bootstrap placeholders were removed. Create crates when implementing their phase; planned work belongs in `docs/PLAN.md` and phase notes.
- A file's initial `//!` module doc (or first Python module docstring) may be at most five lines. The first line is one sentence within 80 columns. Other docs and comments are single lines, never adjacent multi-line blocks. Public core items need a concise contract: invariants, units, ordering and failure conditions where relevant; do not restate the name. Docs must not exceed the item they document. A `comment-check: allow` marker in a doc is a reviewer-visible claim, not automatic permission.
- Ordinary comments are at most 80 columns and 15 words; no allow markers. State constraints not evident in code; remove narration, repetition, history, dividers and commented-out code. A function has at most three ordinary comment lines. A diff has at most `max(2, added code lines / 25)` ordinary comment lines, using integer division. Review semantic and per-function rules manually.
- Run `python3 tools/comment_check.py <changed-source-files>` and `python3 tools/comment_budget.py --base <base-revision> <changed-source-files>` with explicit changed `.rs` and `.py` paths. CI runs these for changed files. The checkers are guardrails, not proof of good documentation or full Rust parsing.

## Architecture and compatibility

- `types ← km2/engine ← host ← platform/gui`. `keymagic-engine` is pure, deterministic, no IO, threads, globals or unsafe. `keymagic-host::Session` owns commit, mode and hotkey policy. Platform adapters translate events and apply actions; they do not duplicate policy.
- Preserve KM2 interoperability, KMS language, engine semantics and the intended `keymagic_*` ABI, unless a constraint is explicitly changed. Tests beat KeyMagic 3 docs; its Rust code beats legacy platform code. Keep 0-based IR indices and convert KM2's 1-based indices only at the codec boundary.
- Composing text persists under normal typing; states last one event unless re-emitted; ANY matches U+0021..U+007E; VK rules match physical events, not recursive matches; KeyMagic's internal `VK_BACK` is 2; right-alt can treat Ctrl+Alt as AltGr. Verify every nuance against upstream tests.
- No platform implementation before the differential harness p2-4 is green. Use one owner per responsibility; no duplicate state, permanent compatibility layers or unbounded fallbacks. New dependencies require a clear justification.

## Resources and egui

- Allocated resources, background workers, file watchers, timers, IPC connections and caches need explicit owners and teardown. Bound caches by bytes or entries. In-flight results carry sufficient session/keyboard generation context to reject stale results. Deferred work must have a guaranteed wake-up; idle GUI windows should not render continuously.
- Use native egui immediate mode, not a web architecture in Rust syntax. Keep durable keyboard/session policy in host/domain state and ephemeral widget state in GUI state, with no mirrored sources of truth. Views present state and request actions through the owning application/session; do not invent an unused command framework.
- Prefer egui containers and widgets; custom widgets need accessible labels. Centralize theme tokens; support keyboard navigation, reduced motion and accessibility. Never block the UI thread on IO, compilation or scans; background work must report results and wake the UI. No per-frame, per-page or per-scroll logging.
- Record memory/performance claims with workload, method, before/after measurements and limits. Do not import unrelated PDF-page, pane, renderer or document-cache rules from other projects.
