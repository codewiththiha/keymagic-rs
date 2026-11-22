# keymagic-engine

A no-unsafe, deterministic key-to-text engine for KeyMagic layouts. It performs no I/O, spawns no threads, and holds no global state.

Behavior follows the pinned upstream KM3 engine at `2db30dc8e8a5f091d1434a2bc1702b29cf113804`. Differential tests compare the two engines on every corpus keyboard.

Enable the `trace` feature to record rule applications in `Output::trace`. Without it, no logging code is compiled.
