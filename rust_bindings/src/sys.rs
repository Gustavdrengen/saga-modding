//! Raw 1:1 bindings to the Saga platform's WASM host imports.
//!
//! Most users should prefer the safe wrappers in [`crate::assets`],
//! [`crate::thread`], [`crate::log`], [`crate::time`], and
//! [`crate::storage`]. The functions in this module are direct mappings
//! of the host-level C ABI surface, with no lifetime, error, or
//! ownership management.
//!
//! Host bindings are namespaced under `#[link(wasm_import_module = ...)]`
//! so the Saga runtime recognises each function at instantiation time.
//!
//! On non-WASM targets the bindings are stubbed so that `cargo check`,
//! `cargo doc`, and `cargo test` continue to work on a developer machine
//! without a Saga runtime attached. The stubs return the canonical
//! "failure" sentinel documented for each function so accidental calls
//! fail loudly rather than silently corrupting state.

#[cfg(target_family = "wasm")]
mod bindings {
    #[link(wasm_import_module = "saga:assets")]
    unsafe extern "C" {
        pub fn saga_asset_open(uri_ptr: *const u8, uri_len: usize) -> i32;
        pub fn saga_asset_get_size(handle: i32) -> usize;
        pub fn saga_asset_read(handle: i32, dest_ptr: *mut u8, length: usize) -> i32;
        pub fn saga_asset_close(handle: i32);
    }

    #[link(wasm_import_module = "saga:thread")]
    unsafe extern "C" {
        pub fn saga_thread_spawn(entry_idx: usize, arg_ptr: usize) -> i32;
        pub fn saga_thread_yield();
    }

    #[link(wasm_import_module = "saga:log")]
    unsafe extern "C" {
        pub fn saga_log(level: u32, msg_ptr: *const u8, msg_len: usize);
    }

    #[link(wasm_import_module = "saga:time")]
    unsafe extern "C" {
        pub fn saga_time_now() -> u64;
        pub fn saga_time_elapsed() -> f64;
    }

    #[link(wasm_import_module = "saga:storage")]
    unsafe extern "C" {
        pub fn saga_save_list(out_buf: *mut u8, max_len: usize) -> i32;

        pub fn saga_save_read_meta(
            save_id_ptr: *const u8,
            save_id_len: usize,
            meta_buf: *mut u8,
            max_len: usize,
        ) -> i32;

        pub fn saga_save_read(
            save_id_ptr: *const u8,
            save_id_len: usize,
            dest_ptr: *mut u8,
            max_len: usize,
        ) -> i32;

        pub fn saga_save_write(
            save_id_ptr: *const u8,
            save_id_len: usize,
            data_ptr: *const u8,
            data_len: usize,
            meta_ptr: *const u8,
            meta_len: usize,
        ) -> i32;

        pub fn saga_save_delete(save_id_ptr: *const u8, save_id_len: usize) -> i32;
    }
}

#[cfg(not(target_family = "wasm"))]
mod bindings {
    // Stubs return the canonical "failure" sentinel so accidental calls
    // on the wrong target fail loudly rather than silently corrupting
    // state. Each stub is `unsafe fn` to mirror the safety contract of
    // the WASM host imports: any call is a guest/host boundary crossing.
    #![allow(unused_variables)]

    pub unsafe fn saga_asset_open(_uri_ptr: *const u8, _uri_len: usize) -> i32 { 0 }
    pub unsafe fn saga_asset_get_size(_handle: i32) -> usize { 0 }
    pub unsafe fn saga_asset_read(_handle: i32, _dest_ptr: *mut u8, _length: usize) -> i32 { -1 }
    pub unsafe fn saga_asset_close(_handle: i32) {}

    pub unsafe fn saga_thread_spawn(_entry_idx: usize, _arg_ptr: usize) -> i32 { -1 }
    pub unsafe fn saga_thread_yield() {}

    pub unsafe fn saga_log(_level: u32, _msg_ptr: *const u8, _msg_len: usize) {}

    pub unsafe fn saga_time_now() -> u64 { 0 }
    pub unsafe fn saga_time_elapsed() -> f64 { 0.0 }

    pub unsafe fn saga_save_list(_out_buf: *mut u8, _max_len: usize) -> i32 { -1 }

    pub unsafe fn saga_save_read_meta(
        _save_id_ptr: *const u8,
        _save_id_len: usize,
        _meta_buf: *mut u8,
        _max_len: usize,
    ) -> i32 {
        -1
    }

    pub unsafe fn saga_save_read(
        _save_id_ptr: *const u8,
        _save_id_len: usize,
        _dest_ptr: *mut u8,
        _max_len: usize,
    ) -> i32 {
        -1
    }

    pub unsafe fn saga_save_write(
        _save_id_ptr: *const u8,
        _save_id_len: usize,
        _data_ptr: *const u8,
        _data_len: usize,
        _meta_ptr: *const u8,
        _meta_len: usize,
    ) -> i32 {
        -1
    }

    pub unsafe fn saga_save_delete(_save_id_ptr: *const u8, _save_id_len: usize) -> i32 {
        -1
    }
}

pub use bindings::*;
