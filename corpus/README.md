# Behavioral corpus

This is an initial, incomplete offline corpus. It does not establish engine parity.

## Upstream test fixtures

The four files listed in [`sources.tsv`](sources.tsv) are byte-for-byte copies
from [KeyMagic 3](https://github.com/thantthet/keymagic-3) at commit
[`2db30dc8e8a5f091d1434a2bc1702b29cf113804`](https://github.com/thantthet/keymagic-3/tree/2db30dc8e8a5f091d1434a2bc1702b29cf113804).
That upstream repository supplies a [GPL version 2 license](https://github.com/thantthet/keymagic-3/blob/2db30dc8e8a5f091d1434a2bc1702b29cf113804/LICENSE.md).
The manifest records the source path and SHA-256 of each file. The upstream
fixtures are unmodified; do not assume that the comprehensive KMS and KM2
fixtures form a byte-identical compile pair without independently verifying it.

## Derived cases

[`cases/basic_rules.toml`](cases/basic_rules.toml) transcribes three tests from
[`keymagic-core/tests/basic_rule_tests.rs`](https://github.com/thantthet/keymagic-3/blob/2db30dc8e8a5f091d1434a2bc1702b29cf113804/keymagic-core/tests/basic_rule_tests.rs).
The accompanying `kms/cases/` scripts express the rules those tests construct
in memory. They are derived test inputs, not copied upstream fixtures. The case
file preserves each tested input, composing text, action, insertion and deletion
count; the single-character case uses U+200B followed by `test`. Each case
starts with a fresh engine. These cases are data only: no engine runs them yet,
and neither the scripts nor parity have been validated by a KMS compiler.

`python3 -m unittest discover -s scripts -p test_corpus.py` verifies fixture
SHA-256 and Git blob hashes and the transcribed expectations without network
access. On the self-hosted runner, `cargo xtask corpus-sync` fetches the pinned
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

Only three behavioral cases, three upstream KMS fixtures and one upstream KM2
fixture are included. The lab targets of at least four bundled KM2 files, ten
KMS files, and 150 cases remain unmet. The source test suite contains many more
behaviors that must be transcribed and validated without changing expectations.
