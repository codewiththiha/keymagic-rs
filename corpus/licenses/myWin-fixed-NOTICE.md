# myWin-fixed.kms notice

`corpus/kms/alternatives/myWin-fixed.kms` is a modified copy of `corpus/kms/alternatives/myWin.kms`.

- Upstream: https://github.com/thantthet/keymagic-keyboards at commit `61da86297fdb99da56b2892190f73009dfe8bc07`, file `myWin/myWin.kms`.
- Original git blob SHA-1 (`corpus/alternatives-lgpl.tsv`): `eb2e029b90dd3aa1ae104a4f6fca87b6b4425d05`.
- Licence: unverified. The upstream repository at `61da862` has no licence file at its root or in `myWin/`, and its README gives no licence for myWin. The LGPL-2.1 label in `corpus/alternatives-lgpl.tsv` and this file does not come from upstream and needs a user decision. The LGPL-2.1 text is at `corpus/licenses/LGPL-2.1.txt`.
- Change: the 12 rule lines that end in a line-continuation backslash after `=>` (original lines 224 to 251) are joined onto one line with the next line, and the backslash is removed. Pinned KM3 rejects a continuation at that point (`Expected output expression` at line 224). Continuations inside variable and string definitions are left unchanged, because the pinned compiler accepts them. Removing the backslashes and whitespace leaves the token stream identical. Line endings stay CRLF.
- Icon: `corpus/kms/alternatives/myWin.png` (1616 bytes, SHA-256 `e01a000c5d6275cf0d376c5ed4e6fc3f0ed4fb3bf3f1a15fbd18cbfc4a00a49b`) is the upstream file `myWin/myWin.png` at commit `61da862`, blob `45200a3361b59573253eecccaf1658d7543be489`, recorded in `corpus/alternatives-lgpl.tsv`. `xtask corpus-sync --check` compares it with upstream. It is byte-identical to the icon blob stored in the pinned upstream `corpus/km2/alternatives/myWin.km2`, at offset 804.
- The original `myWin.kms` stays unchanged and is not compiled by the reference set.
