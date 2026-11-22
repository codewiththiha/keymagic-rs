# xtask

Workspace automation. Rust 2024, strict lints and the no-unsafe rule apply.

`cargo xtask corpus-sync` restores pinned upstream fixtures listed in
[`corpus/sources.tsv`](../corpus/sources.tsv). Add `--dry-run` to list changes
without writing, or `--check` to fail if local files differ. Git and network
access are required. Run Cargo commands on the self-hosted runner, not locally.

Only corpus synchronization is implemented; other automation described in the
lab is not implemented.
