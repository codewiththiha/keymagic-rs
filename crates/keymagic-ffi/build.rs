//! Compiles the C smoke client when the `c-smoke` feature is enabled.

fn main() {
    println!("cargo::rerun-if-changed=tests/ffi_smoke.c");
    if std::env::var_os("CARGO_FEATURE_C_SMOKE").is_some() {
        cc::Build::new()
            .file("tests/ffi_smoke.c")
            .compile("ffi_smoke");
    }
}
