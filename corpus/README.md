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

There are 60 scenarios derived from assertions in the upstream Rust tests:
18 basic/variable, two backreference, six state, six backspace, eight history,
thirteen modifier/virtual-key and seven vowel-reordering scenarios. One scenario
also cites a second upstream test whose asserted input is its prefix. Each case names
its upstream test. The KMS scripts are derived models, not copied production
keyboards. The vowel cases use a minimal model of the rules needed by those
assertions, not the complete MyanSan keyboard; empty-keyboard history cases use
a fixture whose extra `ka` rule is not reached by those input sequences. The
bounded-history case additionally asserts the inferred remaining text. Offline
tests check traceability against every test in the nine engine-behavior source
files. The upstream `test_complex_modifier_combination` is an empty skipped
placeholder and has no scenario; loader, metadata and FFI tests are reference
sources, not represented as engine events or claimed as passing.

A separate 98-case [`any_ascii.toml`](cases/any_ascii.toml) suite exhaustively
tests all 94 different printable ASCII characters plus four nonmatching boundary
characters. These are new source-derived cases, **not** 98 transcribed upstream
tests. Each contains a distinct input and expected output; the suite checks the
pinned [matcher implementation](https://github.com/thantthet/keymagic-3/blob/2db30dc8e8a5f091d1434a2bc1702b29cf113804/keymagic-core/src/engine/matching/matcher.rs).
Offline tests enforce the complete input range, distinct scenario signatures,
and at least one explicit expectation per scenario.

Each case starts with a fresh engine. An event may omit composing text or action
when the original test did not assert it; omission is not an empty value. Events
can include physical keys, modifiers, reset, composing-text replacement and
non-mutating preview. The self-hosted reference workflow compiles the scripts
and compares asserted fields against the separately fetched, pinned upstream
compiler and engine. [Run 44987233685](https://github.com/codewiththiha/keymagic-rs/actions/runs/44987233685)
checked 158 cases with zero differences. This validates the inputs and transcriptions, not parity with this
project's unimplemented engine. The upstream reference is external code
and includes unsafe FFI; it is not a workspace crate or a claim that dependencies
are unsafe-free.

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
individual provenance and redistribution terms are verified. A
[Debian ITP report](https://bugs.debian.org/cgi-bin/bugreport.cgi?bug=933071)
claims GPL2 for a separate IBus package and lists similarly named layouts, but
it does not establish that these exact bundled KM2 bytes are covered. A
[Keyman Malayalam Mozhi keyboard](https://keyman.com/en/keyboards/mozhi_malayalam)
is MIT-licensed, but is a different implementation, not evidence of the
KeyMagic binary's terms.

There are 158 distinct scenarios (60 test-derived and 98 source-derived), 34
derived KMS case scripts, three upstream KMS fixtures and one upstream KM2
fixture, alongside the source references. The derived scripts are not a
substitute for the lab's real keyboard corpus. Four bundled KM2 files with
verified redistribution terms remain missing. Several upstream non-engine tests
and behavioral assertions are still not represented by runnable case scripts.
