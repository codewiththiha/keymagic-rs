//! Keyboards bundled with the app, installable from the Keyboards page.

/// The KM2 files compiled into the binary.
pub const KEYBOARDS: &[&[u8]] = &[include_bytes!(
    "../../../corpus/km2/alternatives/Pyidaungsu-MM.km2"
)];
