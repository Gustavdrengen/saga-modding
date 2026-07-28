# `saga-stdlib`

> Safe, idiomatic Rust wrappers around the Saga platform's WebAssembly
> standard library.

Saga is a modular modding platform in which mods target the
`wasm32-unknown-unknown` WebAssembly target and call a small set of
host imports provided by the Saga Launcher runtime. This crate wraps
those imports so a mod author can stay in safe, ergonomic Rust
instead of hand-rolling `extern "C"` blocks and sentinel-checking
`i32`s.

## Modules

| Crate module | Host namespace | What it does                                           |
| ------------ | -------------- | ------------------------------------------------------ |
| `assets`     | `saga:assets`  | Open / size / read / close assets by `saga://` URI.    |
| `thread`     | `saga:thread`  | Spawn `Worker`s on Web Workers + cooperative `yield`.  |
| `log`        | `saga:log`     | Structured logging with severity levels.               |
| `time`       | `saga:time`    | Wall-clock `now()` (Unix ms) and monotonic `elapsed()` since boot. |
| `storage`    | `saga:storage` | Save-file inspection, read, write, and deletion.       |
| `sys`        | (n/a)          | Raw 1:1 `extern "C"` bindings.                         |

## Quickstart

Add to your Saga mod's `Cargo.toml`:

```toml
[dependencies]
saga-stdlib = "0.1.0"
```

Module skeleton:

```rust
#![no_std]
extern crate alloc;
use saga_stdlib::{fetch_buffer, spawn_thread, Worker, log, LogLevel, now};

extern "C" fn worker(_arg: *mut u8) {
    log(LogLevel::Info, "worker running");
}

#[no_mangle]
pub extern "C" fn com_example_my_register() -> i32 {
    let _ = fetch_buffer("saga://com.example.audio/sfx/jump.wav");

    let args = [1u8, 2, 3, 4];
    let _ = spawn_thread(worker, &args as *const _ as usize);

    let ts = now();
    let _ = ts;

    0
}
```

Build for the Saga runtime:

```bash
cargo build --target wasm32-unknown-unknown --release
```

## Target

`no_std` + `alloc`, intended for `wasm32-unknown-unknown`. On non-WASM
targets the raw bindings in `sys` are stubbed so `cargo check`,
`cargo doc`, and `cargo test` still work on a developer machine; the
stubs return the documented failure sentinel so accidental calls fail
loudly rather than silently corrupting memory.

## License

Dual-licensed under MIT or Apache-2.0, at your option.
