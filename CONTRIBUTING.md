# Contributing

Read [AGENTS.md](AGENTS.md), [docs/PLAN.md](docs/PLAN.md), and the relevant phase notes before starting. The current project is a skeleton, not a working input method.

## Task workflow

1. Confirm the current branch, task status and self-hosted CI results for the actual commit. Read the lab card and all named upstream references; identify three behaviors to preserve.
2. Write tests first, implement one task card per PR, document behavior and public API, and link the card and supporting evidence. Do not change expected corpus results merely to make a test pass.
3. Use Rust edition 2024. Unsafe Rust is forbidden in every crate, including platform and FFI. Treat compiler warnings and Clippy all/pedantic/nursery as errors; unwrap and expect are denied. If a required integration cannot be implemented within that policy, stop and flag the conflict rather than introducing unsafe.
4. All routine Rust builds, checks, formatting, tests and Cargo-related tools run on the trusted **self-hosted** GitHub runner, not the agent workspace. Review its logs for the precise SHA. Hosted runners are only for releases or necessary cross-platform checks, not every PR. Use the manual format-patch and lockfile workflows when required; review the resulting artifacts.
5. PRs use `[pN-N] ...` in the title and describe What, Why, three preserved behaviors and How tested, with run URLs, SHA and relevant manual evidence. Explain new dependencies and document design changes in ADRs. Never mark a task done without its complete evidence.

## Git identity and commit messages

All commits must have author `codewiththiha <kt007441@gmail.com>`. Set repository-local `git config user.name codewiththiha` and `git config user.email kt007441@gmail.com`; these settings may need to be reapplied in ephemeral workspaces. Verify with `git log` and `python3 scripts/check_commit_authors.py --range <revision-range>`.

Each commit message must be one nonempty line of at most **50 Unicode characters** in total. Before committing run `python3 scripts/check_commit_messages.py --message 'Your message'`. CI checks the commit range for both policies. Do not force-push unless the user explicitly requests it; if requested inspect the remote and use `--force-with-lease`.

## Scope and license

The workspace metadata currently uses GPL-2.0-only; see [LICENSE](LICENSE). Upstream-derived reference docs identify their source and revision. Confirm any new vendored material's license and provenance before including it. Keep credentials outside the repo and out of logs.
