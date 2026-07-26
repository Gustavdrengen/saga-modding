//! # Saga Standard Library (Rust Wrapper)
//!
//! Safe, idiomatic Rust wrappers around the host-level **Saga standard library**
//! described in the Saga Platform Mod Specification. Mod authors targeting
//! `wasm32-unknown-unknown` can rely on these wrappers to interact with the
//! Saga Launcher runtime (asset protocol, worker spawning, etc).
//!
//! ## Modules
//!
//! | Module             | Spec section | Purpose                                |
//! | ------------------ | ------------ | -------------------------------------- |
//! | [`assets`]         | `saga:assets`| Open / read / close `saga://` URIs     |
//! | [`thread`]         | `saga:thread`| Spawn `Worker`s on Web Worker threads |
//! | [`sys`]            | (n/a)        | Raw 1:1 `extern "C"` bindings          |
//!
//! ## Target
//!
//! This crate is `no_std` + `alloc` and is intended to be compiled for the
//! `wasm32-unknown-unknown` target. The host imports listed in [`sys`] are
//! provided at runtime by the Saga Launcher via the `saga:assets` and
//! `saga:thread` import namespaces.
//!
//! On non-WASM targets, the [`sys`] module exposes stub functions that simply
//! return the documented "failure" sentinel. This allows `cargo check`,
//! `cargo build` (with `std`), `cargo test`, and `cargo doc` to function on
//! a developer's native machine without a Saga runtime.
//!
//! ## Global allocator (for `wasm32-unknown-unknown`)
//!
//! This crate declares `extern crate alloc;` to expose `alloc::vec::Vec<u8>`
//! in [`fetch_buffer`](crate::assets::fetch_buffer). When the `alloc_handler`
//! feature is enabled (default), we ship a small inline bump allocator that
//! is `wasm32-unknown-unknown`-only and is registered as `#[global_allocator]`.
//! Consumers can disable this with `--no-default-features --features std` and
//! supply their own Wasm-memory allocator linked in via the host build.
//!
//! [`assets`]: crate::assets
//! [`thread`]: crate::thread
//! [`sys`]:    crate::sys

#![no_std]
#![cfg_attr(docsrs, feature(doc_cfg))]
// `#[alloc_error_handler]` is still an unstable language feature. We gate
// the whole allocator module + this feature attribute behind the
// `alloc_handler` Cargo feature so crates built without that feature do not
// trigger the unstable-feature error.
#![cfg_attr(
    all(target_family = "wasm", feature = "alloc_handler"),
    feature(alloc_error_handler)
)]

extern crate alloc;

pub mod assets;
pub mod sys;
pub mod thread;

/// Re-exports of the most common high-level helpers.
///
/// The full API is under [`assets`] and [`thread`].
pub use crate::assets::{fetch_buffer, open, AssetError, AssetHandle, AssetResult};
pub use crate::thread::{spawn_thread, spawn_thread_raw, yield_now, ThreadError, ThreadResult, Worker};

// ---------------------------------------------------------------------------
// Global allocator – wasm guest only, opt-in via `alloc_handler` feature.
// Thread-safe bump built on `AtomicUsize`. Production runtimes are expected
// to replace this with a proper Wasm-linear-memory allocator linked via the
// host build pipeline.
// ---------------------------------------------------------------------------
#[cfg(all(target_family = "wasm", feature = "alloc_handler"))]
mod allocator {
    use core::alloc::{GlobalAlloc, Layout};
    use core::ptr;
    use core::sync::atomic::{AtomicUsize, Ordering};

    const HEAP_SIZE: usize = 64 * 1024; // 64 KiB – generous for a demo mod.

    /// Tiny bump allocator. Thread-safe via `AtomicUsize`, doesn't free.
    pub struct Bump;

    // SAFETY: see [`GlobalAlloc`].
    unsafe impl GlobalAlloc for Bump {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            static HEAP: [u8; HEAP_SIZE] = [0; HEAP_SIZE];
            static NEXT: AtomicUsize = AtomicUsize::new(0);

            let align = layout.align().max(1);
            let size  = layout.size();

            // CAS-loop: align the current offset, attempt to claim the
            // required slice. The bumps are independent of any other
            //                         worker / guest that shares the static.
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
                    current, end, Ordering::Relaxed, Ordering::Relaxed,
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
    pub fn on_oom(_layout: Layout) -> ! { loop {} }
}
