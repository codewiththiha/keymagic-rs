# Contributing

Read [AGENTS.md](AGENTS.md), [docs/PLAN.md](docs/PLAN.md), and the relevant phase notes before starting. The current project is a skeleton, not a working input method.

## Task workflow

1. Confirm the current branch, task status and self-hosted CI results for the actual commit. Read the lab card and all named upstream references; identify three behaviors to preserve.
2. Write tests first, implement the smallest useful task on `main`, document behavior and public API, and record the card and supporting evidence. Do not create branches or PRs unless requested. Do not change expected corpus results merely to make a test pass.
3. Use Rust edition 2024. Unsafe Rust is forbidden in every crate, including platform and FFI. Treat compiler warnings and Clippy all/pedantic/nursery as errors; unwrap and expect are denied. If a required integration cannot be implemented within that policy, stop and flag the conflict rather than introducing unsafe.
4. All routine Rust builds, checks, formatting, tests and Cargo-related tools run on the trusted **self-hosted** GitHub runner, not the agent workspace. Review its logs for the precise SHA. Hosted runners are only for releases or necessary cross-platform checks, not every PR. Use the manual format-patch and lockfile workflows when required; review the resulting artifacts.
5. Record what changed, three preserved behaviors, test/run URLs, SHA and relevant manual evidence in the external progress file. Explain new dependencies and document design changes in ADRs. Never mark a task done without its complete evidence.
6. Use rustfmt defaults with `max_width = 100`. No TODO/FIXME markers, speculative scaffolding or repeated narration in comments. Initial module docs are at most five lines; all other comments/docs are single lines. Run `python3 tools/comment_check.py <changed-files>` and `python3 tools/comment_budget.py --base <base> <changed-files>` on explicit `.rs` and `.py` files; also review semantic rules in AGENTS.md.

## Git identity and commit messages

All commits must have author `codewiththiha <kt007441@gmail.com>`. Set repository-local `git config user.name codewiththiha` and `git config user.email kt007441@gmail.com`; these settings may need to be reapplied in ephemeral workspaces. Verify with `git log` and `python3 scripts/check_commit_authors.py --range <revision-range>`.

Use a Conventional Commit such as `feat: add key parser` or `fix(engine): handle input`, on one line of at most **50 Unicode characters** total. Before committing run `python3 scripts/check_commit_messages.py --message 'feat: add key parser'`. CI checks the commit range for both policies. At session end, squash related very small commits; fold failed-CI commits into their fixes. The user authorizes force-pushing those rewrites with an exact lease after inspecting the remote, without overwriting unrelated work. Validate the final SHA. Other force pushes require separate authorization. Rewriting history does not necessarily remove GitHub Actions records.

## Scope and license

The workspace metadata currently uses GPL-2.0-only; see [LICENSE](LICENSE). Upstream-derived reference docs identify their source and revision. Confirm any new vendored material's license and provenance before including it. Keep credentials outside the repo and out of logs.
