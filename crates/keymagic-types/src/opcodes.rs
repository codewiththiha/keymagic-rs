//! KM2 wire opcode and modifier values from the pinned upstream format.

/// UTF-16 literal with a 16-bit code-unit count.
pub const STRING: u16 = 0x00F0;
/// One-based string-table reference.
pub const VARIABLE: u16 = 0x00F1;
/// One-based captured-pattern reference.
pub const REFERENCE: u16 = 0x00F2;
/// Internal virtual key or the RHS null marker.
pub const PREDEFINED: u16 = 0x00F3;
/// Context-dependent wildcard or capture-index operand.
pub const MODIFIER: u16 = 0x00F4;
/// Match one character contained in the preceding variable.
pub const ANY_OF: u16 = 0x00F5;
/// Begin a virtual-key combination.
pub const AND: u16 = 0x00F6;
/// Match one character not contained in the preceding variable.
pub const NOT_ANY_OF: u16 = 0x00F7;
/// Match a printable ASCII character.
pub const ANY: u16 = 0x00F8;
/// Read or emit a state identifier.
pub const SWITCH: u16 = 0x00F9;
