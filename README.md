# keymagic-rs

[![Development CI](https://github.com/codewiththiha/keymagic-rs/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/codewiththiha/keymagic-rs/actions/workflows/ci.yml?query=branch%3Amain)

An engine-first Rust reimplementation of the KeyMagic input method, following the [KeyMagic Rust Lab](https://01a1ff77-4137-7a69-a0fa-36ed3fcc921d.arena.site/). The Cargo workspace currently contains only the implemented corpus synchronization tool; the input method, compiler, GUI, platform adapters, and packaging are not implemented yet.

Read [AGENTS.md](AGENTS.md) and [docs/PLAN.md](docs/PLAN.md). The project targets Rust 2024, forbids unsafe Rust across all crates, and treats compiler and strict Clippy warnings as errors. Routine checks run on a self-hosted GitHub Actions runner; hosted cross-platform checks run only when needed or for releases. Check the Actions run for the relevant commit before relying on a result.
