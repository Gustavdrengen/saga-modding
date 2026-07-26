# `saga-stdlib`

> Safe, idiomatic Rust wrappers around the Saga platform's WebAssembly
> standard library.

Saga is a modular modding platform in which mods target the
`wasm32-unknown-unknown` WebAssembly target and call a small set of host
imports provided by the Saga Launcher runtime. This crate wraps those
imports so a mod author can stay in safe, ergonomic Rust instead of
hand-rolling `extern "C"` blocks and sentinel-checking `i32`s.

## Modules

| Crate module | Spec namespace | What it does                                          |
| ------------ | -------------- | ----------------------------------------------------- |
| `assets`     | `saga:assets`  | Open / size / read / close assets by `saga://` URI.   |
| `thread`     | `saga:thread`  | Spawn `Worker`s on Web Workers + cooperative `yield`. |
| `sys`        | (n/a)          | Raw 1:1 `extern "C"` bindings.                        |

## Quickstart

Add to your Saga mod's `Cargo.toml`. The crate is not on crates.io yet, so
import it directly from the Saga git repository:

```toml
[dependencies]
saga-stdlib = "0.1.0"
```

Then in your `module.wasm` source:

```rust
#![no_std]
extern crate alloc;
use saga_stdlib::{fetch_buffer, spawn_thread, Worker};

extern "C" fn worker(_arg: *mut u8) {
    // ...
}

#[no_mangle]
pub extern "C" fn init() -> i32 {
    let _ = fetch_buffer("saga://com.example.audio/sfx/jump.wav");

    let args = [1u8, 2, 3, 4];
    let _ = spawn_thread(worker, &args as *const _ as usize);

    0
}
```

And build for the Saga runtime:

```bash
cargo build --target wasm32-unknown-unknown --release
```

## Target

This crate is `no_std` + `alloc` and is intended to be compiled for
`wasm32-unknown-unknown`. On non-WASM targets the raw bindings in
`sys` are stubbed so that `cargo check`, `cargo doc`, and `cargo test`
still work on the developer's machine — the stubs always return the
canonical "failure" sentinel so accidental calls fail loudly instead of
silently corrupting memory.

## License

Dual-licensed under MIT or Apache-2.0, at your option.
