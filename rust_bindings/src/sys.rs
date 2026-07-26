//! Raw 1:1 bindings to the Saga platform's WASM host imports.
//!
//! Most users should prefer the safe wrappers in [`crate::assets`] and
//! [`crate::thread`]. The functions in this module are direct mappings of
//! the host-level C ABI described in the Saga Platform Mod Specification,
//! with no lifetime, error, or ownership management.
//!
//! Host bindings are imported under the canonical namespaces:
//!
//! - `saga:assets` — asset protocol (open / size / read / close).
//! - `saga:thread` — worker spawning (spawn / yield).
//!
//! On non-WASM targets the bindings are stubbed so that the rest of the
//! crate continues to compile, check, and document on a developer machine
//! without a Saga runtime attached.

#[cfg(target_family = "wasm")]
mod bindings {
    //! WASM host imports. Namespaced via `#[link(wasm_import_module = ...)]`
    //! so the Saga runtime recognises the symbols at instantiation time.

    #[link(wasm_import_module = "saga:assets")]
    unsafe extern "C" {
        /// Open an asset identified by a `saga://` URI.
        ///
        /// Returns the asset handle (> 0 on success, <= 0 on error).
        pub fn saga_asset_open(uri_ptr: *const u8, uri_len: usize) -> i32;

        /// Query the byte length of an opened asset.
        pub fn saga_asset_get_size(handle: i32) -> usize;

        /// Copy asset bytes into the supplied buffer.
        ///
        /// Returns the number of bytes actually read (< 0 on error).
        pub fn saga_asset_read(handle: i32, dest_ptr: *mut u8, length: usize) -> i32;

        /// Close a previously-opened asset handle, releasing host resources.
        pub fn saga_asset_close(handle: i32);
    }

    #[link(wasm_import_module = "saga:thread")]
    unsafe extern "C" {
        /// Spawn a Web Worker that runs `entry_idx` (table index) over `arg_ptr`.
        ///
        /// Returns a thread id (> 0) or error code (< 0).
        pub fn saga_thread_spawn(entry_idx: usize, arg_ptr: usize) -> i32;

        /// Yield execution on the current worker.
        pub fn saga_thread_yield();
    }
}

#[cfg(not(target_family = "wasm"))]
mod bindings {
    //! Native stubs. These make the crate type-check on non-WASM hosts
    //! (developer machines, CI, `cargo test`) — they always return the
    //! canonical "failure" sentinel documented in the Saga spec, so any
    //! accidental call on the wrong target fails loudly rather than silently
    //! corrupting state.
    //!
    //! Each stub is `unsafe fn` to mirror the safety contract of the WASM
    //! host imports: any call is a guest/host boundary crossing and is
    //! therefore `unsafe`. Declaring them `unsafe fn` lets the safe wrappers
    //! in this crate have an identical `unsafe { ... }` surface on both
    //! targets, and suppresses the `unnecessary_unsafe_block` lint on
    //! native builds (where these would otherwise look like plain fns).
    #![allow(unused_variables)]

    pub unsafe fn saga_asset_open(_uri_ptr: *const u8, _uri_len: usize) -> i32 { 0 }
    pub unsafe fn saga_asset_get_size(_handle: i32) -> usize { 0 }
    pub unsafe fn saga_asset_read(_handle: i32, _dest_ptr: *mut u8, _length: usize) -> i32 { -1 }
    pub unsafe fn saga_asset_close(_handle: i32) {}

    pub unsafe fn saga_thread_spawn(_entry_idx: usize, _arg_ptr: usize) -> i32 { -1 }
    pub unsafe fn saga_thread_yield() {}
}

pub use bindings::*;
