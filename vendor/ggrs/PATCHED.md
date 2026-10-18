# ggrs 0.13.0, patched

This is ggrs 0.13.0 from crates.io (MIT OR Apache-2.0, see LICENSE-MIT and LICENSE-APACHE), minus its examples, tests and docs. The only change:

- `js-sys` and the two `cfg(target_arch = "wasm32")` code paths in `src/network/protocol.rs` and `src/sessions/p2p_session.rs` now apply only to wasm32 targets other than Emscripten.

Upstream ggrs uses wasm-bindgen's `js_sys::Date` on every wasm32 target. Under `wasm32-unknown-emscripten` those imports never resolve, so the web build does not link. Emscripten provides `SystemTime` and `thread::yield_now`, so the native code paths work there.

Drop this directory and the `[patch.crates-io]` entry in the root `Cargo.toml` once a ggrs release makes the same distinction.
