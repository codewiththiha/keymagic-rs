# keymagic-km2

A no-unsafe, in-memory reader and writer for KeyMagic KM2 files.

`parse` reads supported 1.3, 1.4 and 1.5 files into shared keyboard data. `write` emits canonical 1.5 and is fallible when an input is invalid or exceeds a 16-bit format limit. `parse_document` and `write_document` retain the original version and 1.5 padding byte: this is essential for byte-identical 1.3 and 1.4 round trips. Version 1.3 cannot represent metadata; versions 1.3 and 1.4 cannot represent `right_alt = false`. Older versions read with right-alt enabled, matching the pinned KM3 loader.

On-disk variable and capture references are one-based; IR indices are zero-based. State IDs are copied without adjustment, as in KM3. Metadata records retain order, unknown IDs and raw bytes. Virtual-key groups retain their original key order and sided modifiers rather than reducing them to unordered flag bits. Source spans are compiler-side information and are not serialized.

`read_info_only` skips the string payloads and stops after metadata; it does not check UTF-16 or rule bytes. Full parsing validates UTF-16 and rule boundaries, rejects unknown opcodes, invalid boolean values and trailing bytes, and reports byte offsets. UTF-16 lengths are code-unit counts; info lengths are byte counts; rule lengths are 16-bit-word counts. The 1.5 header is 18 bytes including its alignment byte, despite the older upstream format-document sketch.
