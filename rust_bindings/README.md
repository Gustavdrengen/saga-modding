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
saga-stdlib = {
    git = "https://github.com/Gustavdrengen/saga-modding.git",
    package = "saga-stdlib",
}
```

Cargo searches Git dependency repositories for package manifests, so this
works even though the crate lives in the repository's `rust_bindings/`
subdirectory. Cargo records the resolved commit in your mod's `Cargo.lock`;
run `cargo update -p saga-stdlib` when you want to pull a newer commit.

Module skeleton:

```rust
#![no_std]
extern crate alloc;

// The final no_std + alloc module must provide its own allocator.
#[global_allocator]
static ALLOCATOR: MyAllocator = MyAllocator;

use saga_stdlib::{fetch_buffer, spawn_thread, Worker, log, LogLevel, now};

struct MyAllocator;

unsafe impl core::alloc::GlobalAlloc for MyAllocator {
    unsafe fn alloc(&self, _layout: core::alloc::Layout) -> *mut u8 {
        core::ptr::null_mut()
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: core::alloc::Layout) {}
}

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

This crate does not install a global allocator. A mod that uses this
crate from `no_std` + `alloc` must provide the allocator required by its
own final WebAssembly module, as shown in the skeleton above. A mod that
uses ordinary Rust `std` receives Rust's normal allocator from `std` and
does not write a `#[global_allocator]` just to use this crate. Saga does not
replace either allocator; its language-neutral merger relocates each
module's state and its optimizer removes only structurally identical
WebAssembly code.


## Target

`no_std` + `alloc`, intended for `wasm32-unknown-unknown`. On non-WASM
targets the raw bindings in `sys` are stubbed so `cargo check`,
`cargo doc`, and `cargo test` still work on a developer machine; the
stubs return the documented failure sentinel so accidental calls fail
loudly rather than silently corrupting memory.

## Repository

The source for this crate is maintained at
<https://github.com/Gustavdrengen/saga-modding/tree/main/rust_bindings>.

## License

Dual-licensed under MIT or Apache-2.0, at your option.
