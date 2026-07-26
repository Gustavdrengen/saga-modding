//! `basic_usage.rs` — A `cargo run`-friendly demonstration of the
//! `saga-stdlib` public API.
//!
//! This example is a plain Rust binary: it uses the standard library and a
//! normal `fn main`, so it `cargo check`s / builds / runs on a developer
//! machine without any WASM toolchain. Calls into the Saga host are
//! expected to fail on the host (returns the documented failure sentinel
//! from the native stub), which is exactly what surface-level smoke-tests
//! want — we want to fail loudly, not pretend to read real assets.
//!
//! # Recipe for using these patterns in a real Saga mod
//!
//! 1. Compile your mod for `wasm32-unknown-unknown`.
//! 2. Replace `fn main()` with `#[no_mangle] pub extern "C" fn init()`.
//! 3. Drop the `#![allow(dead_code)]` and the failure-doesn't-matter attitude:
//!    real Saga mods should propagate errors via the mod's own conventions.

#![allow(dead_code)]

use std::vec::Vec;

use saga_stdlib::{fetch_buffer, spawn_thread, yield_now, AssetError, AssetHandle};

// A `Vec<u8>` we hand to the worker. The pointer is taken from
// `&mut buf`, which on the WASM target lives in shared linear memory.
extern "C" fn worker(arg: *mut u8) {
    // SAFETY: see `main` below — `buf` outlives the spawned worker.
    let buf: &mut Vec<u8> = unsafe { &mut *(arg as *mut Vec<u8>) };
    buf.push(0xDE);
    buf.push(0xAD);
    buf.push(0xBE);
    buf.push(0xEF);
}

fn main() {
    // 1. One-liner to slurp a whole asset into a Vec<u8>.
    // (Returns Err on the host because the native stub rejects opens —
    // that's exactly what we want for a smoke-test example.)
    match fetch_buffer("saga://com.example.audio/sfx/jump.wav") {
        Ok(bytes) => println!("fetched {} bytes", bytes.len()),
        Err(e) => println!("fetch_buffer failed (expected on host): {e}"),
    }

    // 2. Open a handle, query size, partial-read.
    if let Err(e) = demo_asset_handle() {
        println!("asset handle demo failed (expected on host): {e}");
    }

    // 3. Spawn a worker. The args Vec stays alive in `main`'s scope, which
    // is longer than the worker's lifetime — good practice. The Saga
    // runtime does not copy the args buffer; you must guarantee the
    // pointer is valid for the worker's entire run.
    let mut buf: Vec<u8> = Vec::new();
    match spawn_thread(worker, &mut buf as *mut _ as usize) {
        Ok(tid) => println!("spawned worker tid={tid}"),
        Err(e) => println!("spawn_thread failed (expected on host): {e}"),
    }
    yield_now();
    println!("worker wrote {} bytes", buf.len());
}

fn demo_asset_handle() -> Result<(), AssetError> {
    let handle: AssetHandle = AssetHandle::open("saga://self/textures/grass.png")?;
    let total = handle.size();
    let mut chunk = vec![0u8; total.min(64)];
    let n = handle.read(&mut chunk)?;
    println!("read {n} bytes (host-reported size was {total})");
    // `handle` closes automatically on the next line.
    drop(handle);
    Ok(())
}
