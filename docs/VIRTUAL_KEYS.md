# Virtual keys — reference and pending table

This is a **stub**, not the completed canonical table. Phase p1-1 must reproduce the exact KeyMagic internal enum and aliases from the upstream sources and add a snapshot test; do not substitute Windows virtual-key codes.

- [Original KeyCodes.h](https://github.com/thantthet/keymagic/blob/master/cross-platform/libkeymagic/KeyCodes.h).
- [KeyMagic 3 virtual_keys.rs, reference revision](https://github.com/thantthet/keymagic-3/blob/2db30dc8e8a5f091d1434a2bc1702b29cf113804/keymagic-core/src/types/virtual_keys.rs).
- [KeyMagic 3 KMS format](KMS_FORMAT.md), available virtual keys and aliases.

The lab highlights internal `VK_BACK = 2`; verify all values against upstream before implementation. The table, platform conversions and name snapshot have not yet been implemented.
