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
//! ## Example
//!
//! ```no_run
//! # extern crate saga_stdlib;
//! use saga_stdlib::{fetch_buffer, AssetHandle};
//!
//! // Read an entire asset in one call.
//! let bytes = fetch_buffer("saga://com.example.audio/sfx/jump.wav")?;
//!
//! // …or open a handle and read in chunks.
//! let handle = AssetHandle::open("saga://self/textures/grass.png")?;
//! let total = handle.size();
//! let mut chunk = vec![0u8; total.min(1024)];
//! let n = handle.read(&mut chunk)?;
//! # Ok::<(), saga_stdlib::AssetError>(())
//! ```
//!
//! [`assets`]: crate::assets
//! [`thread`]: crate::thread
//! [`sys`]:    crate::sys

#![no_std]
#![cfg_attr(docsrs, feature(doc_cfg))]

extern crate alloc;

pub mod assets;
pub mod sys;
pub mod thread;

/// Re-exports of the most common high-level helpers.
///
/// The full API is under [`assets`] and [`thread`].
pub use crate::assets::{fetch_buffer, open, AssetError, AssetHandle, AssetResult};
pub use crate::thread::{spawn_thread, spawn_thread_raw, yield_now, ThreadError, ThreadResult, Worker};
