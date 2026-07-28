//! `basic_usage.rs` — Demonstrates the `saga-stdlib` public API on the
//! host platform.
//!
//! This is a plain Rust binary that uses `std` and a normal `fn main`,
//! so it `cargo check`s / builds / runs on a developer machine without
//! any WASM toolchain. Calls into the Saga host are expected to fail
//! against the native stub (which returns the documented failure
//! sentinel) — surface-level smoke-tests want to fail loudly rather
//! than pretend to have a working Saga runtime attached.
//!
//! # Recipe for using these patterns in a real Saga mod
//!
//! 1. Compile your mod for `wasm32-unknown-unknown`.
//! 2. Replace `fn main()` with a uniquely-named registration entrypoint
//!    (e.g. `#[no_mangle] pub extern "C" fn com_example_register() -> i32`).
//! 3. Drop the `#![allow(dead_code)]` and the "failure-is-fine" attitude;
//!    real Saga mods propagate errors through their own conventions.

#![allow(dead_code)]

use std::vec::Vec;

use saga_stdlib::{
    elapsed, emit, fetch_buffer, log, now, spawn_thread, AssetError, AssetHandle,
    LogLevel, StorageResult,
};

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
    match fetch_buffer("saga://com.example.audio/sfx/jump.wav") {
        Ok(bytes) => println!("fetched {} bytes", bytes.len()),
        Err(e)    => println!("fetch_buffer failed (expected on host): {e}"),
    }

    // 2. Open a handle, query size, partial-read.
    if let Err(e) = demo_asset_handle() {
        println!("asset handle demo failed (expected on host): {e}");
    }

    // 3. Spawn a worker. The args vec stays alive in `main`'s scope,
    // which is longer than the worker's lifetime — good practice. The
    // Saga runtime does not copy the args buffer; the caller must
    // guarantee the pointer is valid for the worker's entire run.
    let mut buf: Vec<u8> = Vec::new();
    match spawn_thread(worker, &mut buf as *mut _ as usize) {
        Ok(tid) => println!("spawned worker tid={tid}"),
        Err(e)  => println!("spawn_thread failed (expected on host): {e}"),
    }
    println!("worker wrote {} bytes", buf.len());

    // 4. Time queries. The native stub returns 0 for both; a real
    // Saga runtime returns wall-clock Unix-ms and monotonic seconds
    // since the session started.
    println!("clock now={} ms; elapsed={} s", now(), elapsed());

    // 5. Structured logging goes through saga:log. `emit` formats in
    // place into a bounded stack buffer.
    log(LogLevel::Info, "structured log via saga:log");
    emit(
        LogLevel::Debug,
        format_args!("now={} ms elapsed={:.3}s", now(), elapsed()),
    );

    // 6. Save-file storage API. Every call returns the host's failure
    // sentinel on the stub platform.
    match demo_storage() {
        Ok(()) => println!("storage roundtrip ok"),
        Err(e) => println!("storage failed (expected on host): {e}"),
    }
}

fn demo_asset_handle() -> Result<(), AssetError> {
    let handle: AssetHandle = AssetHandle::open("saga://self/textures/grass.png")?;
    let total = handle.size();
    let mut chunk = vec![0u8; total.min(64)];
    let n = handle.read(&mut chunk)?;
    println!("read {n} bytes (host-reported size was {total})");
    drop(handle);
    Ok(())
}

fn demo_storage() -> StorageResult<()> {
    let _ = saga_stdlib::list()?;
    let _ = saga_stdlib::read("autosave")?;
    saga_stdlib::write("autosave", b"hello", r#"{"slot":0}"#)?;
    saga_stdlib::delete("autosave")?;
    Ok(())
}
