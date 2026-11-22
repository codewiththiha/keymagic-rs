# Behavioral corpus

This is an initial, incomplete offline corpus. It does not establish engine parity.

## Upstream test fixtures

The files listed in [`sources.tsv`](sources.tsv) are byte-for-byte copies
from [KeyMagic 3](https://github.com/thantthet/keymagic-3) at commit
[`2db30dc8e8a5f091d1434a2bc1702b29cf113804`](https://github.com/thantthet/keymagic-3/tree/2db30dc8e8a5f091d1434a2bc1702b29cf113804).
That upstream repository supplies a [GPL version 2 license](https://github.com/thantthet/keymagic-3/blob/2db30dc8e8a5f091d1434a2bc1702b29cf113804/LICENSE.md).
The manifest records the source path and SHA-256 of each file. The upstream
fixtures are unmodified; do not assume that the comprehensive KMS and KM2
fixtures form a byte-identical compile pair without independently verifying it.

## Upstream engine test references

[`reference/km3/`](reference/km3/) contains all 15 Rust source files from the
upstream `keymagic-core/tests/` directory, including its two helper files. They
are preserved verbatim under their original `.rs` names as reference documents,
not compiled as part of this workspace; their original paths and hashes appear
in the manifest. CI excludes this reference directory from project comment-style
checks and separately verifies the exact file set, hashes and upstream bytes. The
sources contain 76 `#[test]` annotations, including non-engine tests such as
metadata and FFI checks. They have not been converted into 76 executable cases,
and tests embedded elsewhere in the upstream repository are not included.

## Derived cases

[`cases/basic_rules.toml`](cases/basic_rules.toml) transcribes 18 input
scenarios from the pinned `basic_rule_tests.rs` and `variable_tests.rs` source
references. The accompanying `kms/cases/` scripts express the rules those tests
construct in memory. They are derived test inputs, not copied upstream fixtures.
Each case starts with a fresh engine. An event may omit composing text or action
when the original test did not assert it; do not interpret omission as an empty
value. The single-character case uses U+200B followed by `test`. These cases are
data only: no engine runs them yet, and neither the scripts nor parity have been
validated by a KMS compiler.

`python3 -m unittest discover -s scripts -p test_corpus.py` verifies fixture
SHA-256 and Git blob hashes, case references, and selected transcribed
expectations without network access. On the self-hosted runner, `cargo xtask corpus-sync` fetches the pinned
commit using Git and verifies each source blob before restoring changed fixtures.
`--dry-run` lists differences without writing; `--check` fails on differences.
The sync command verifies Git blob IDs; the Python test independently verifies
SHA-256. All downloads are checked before any destination files are written.

## License and coverage limits

The upstream repository is GPL-2.0-licensed, but its bundled production layouts
may have separate authors or terms. The separate
[community keyboard collection](https://github.com/thantthet/keymagic-keyboards/blob/61da86297fdb99da56b2892190f73009dfe8bc07/README.md)
includes user submissions and grants distribution permissions to the collection;
it does not clearly grant this project redistribution rights. Neither those
layouts nor the bundled production binaries are copied here until their
individual provenance and redistribution terms are verified.

There are 18 transcribed behavioral scenarios, 15 derived KMS case scripts,
three upstream KMS fixtures and one upstream KM2 fixture, alongside the source
references. The derived scripts are not a substitute for the lab's real keyboard
corpus. The targets of four bundled KM2 files, ten KMS files, and 150 cases are
not all met. Many behaviors still need transcription and validation without
changing expectations.
