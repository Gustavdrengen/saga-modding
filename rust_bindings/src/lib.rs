//! Safe, idiomatic Rust wrappers around the host-level Saga standard
//! library. Mod authors targeting `wasm32-unknown-unknown` use these
//! wrappers to interact with the Saga Launcher runtime without
//! hand-rolling `extern "C"` blocks or sentinel-checking `i32`s.
//!
//! # Modules
//!
//! - [`assets`]  — open / size / read / close assets by `saga://` URI.
//! - [`thread`]  — spawn `Worker`s on Web Worker threads.
//! - [`log`]     — structured, engine-tagged logging.
//! - [`time`]    — wall-clock (`now`) and monotonic-from-boot (`elapsed`) queries.
//! - [`storage`] — save file inspection, read, write, and deletion.
//! - [`sys`]     — raw 1:1 `extern "C"` bindings.
//!
//! # Target
//!
//! `no_std` + `alloc`, intended for `wasm32-unknown-unknown`. The
//! [`sys`] module exposes stub implementations on non-WASM targets so
//! `cargo check`, `cargo build` (with `std`), `cargo test`, and
//! `cargo doc` work on a developer machine without a Saga runtime.
//!
//! # Allocation
//!
//! The crate pulls in `extern crate alloc` so [`assets::fetch_buffer`]
//! and other helpers can return `Vec<u8>`. By default, the crate does not
//! install a global allocator. A `no_std` + `alloc` final module MUST supply
//! its own allocator, or explicitly enable this crate's `alloc_handler`
//! feature to use its fixed bump allocator. A module using ordinary Rust
//! `std` receives the normal allocator from `std` and does not need a
//! `#[global_allocator]` merely to use this crate. Saga does not replace
//! either runtime; its merger and optimizer operate on WebAssembly.

#![no_std]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(
    all(target_family = "wasm", feature = "alloc_handler"),
    feature(alloc_error_handler)
)]

extern crate alloc;

pub mod assets;
pub mod log;
pub mod storage;
pub mod sys;
pub mod thread;
pub mod time;

// Re-exports of the most common high-level helpers.
pub use crate::assets::{fetch_buffer, open, AssetError, AssetHandle, AssetResult};
pub use crate::log::{emit, log, LogLevel};
pub use crate::storage::{delete, list, read, read_meta, write, StorageError, StorageResult};
pub use crate::thread::{spawn_thread, spawn_thread_raw, yield_now, ThreadError, ThreadResult, Worker};
pub use crate::time::{elapsed, now};

// ----------------------------------------------------------------------------
// Optional in-crate bump allocator. It is enabled only by the explicit
// `alloc_handler` feature and is not required by Saga or by this crate's
// default configuration.
// ----------------------------------------------------------------------------
#[cfg(all(target_family = "wasm", feature = "alloc_handler"))]
mod allocator {
    use core::alloc::{GlobalAlloc, Layout};
    use core::ptr;
    use core::sync::atomic::{AtomicUsize, Ordering};

    const HEAP_SIZE: usize = 64 * 1024;

    pub struct Bump;

    // SAFETY: see [`GlobalAlloc`].
    unsafe impl GlobalAlloc for Bump {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            static HEAP: [u8; HEAP_SIZE] = [0; HEAP_SIZE];
            static NEXT: AtomicUsize = AtomicUsize::new(0);

            let align = layout.align().max(1);
            let size = layout.size();

            let mut current = NEXT.load(Ordering::Relaxed);
            loop {
                let aligned = (current + align - 1) & !(align - 1);
                let end = match aligned.checked_add(size) {
                    Some(end) => end,
                    None => return ptr::null_mut(),
                };
                if end > HEAP_SIZE {
                    return ptr::null_mut();
                }
                match NEXT.compare_exchange_weak(
                    current,
                    end,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => return HEAP.as_ptr().add(aligned) as *mut u8,
                    Err(actual) => current = actual,
                }
            }
        }

        unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
    }

    #[global_allocator]
    pub static ALLOC: Bump = Bump;

    #[alloc_error_handler]
    pub fn on_oom(_layout: Layout) -> ! {
        loop {}
    }
}
