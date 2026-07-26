//! Safe wrappers around the `saga:thread` host import.
//!
//! Provides [`spawn_thread`] / [`spawn_thread_raw`] for dispatching
//! work onto a fresh Web Worker, plus [`yield_now`] for cooperative
//! preemption.
//!
//! # Function pointer → table index
//!
//! On `wasm32-unknown-unknown`, an `extern "C" fn(...)` is materialised
//! as a slot in the WASM indirect-call table. [`spawn_thread`] reuses
//! the function pointer as the table index directly.
//!
//! # Lifetime of `arg_ptr`
//!
//! The Saga runtime does not copy the argument buffer when spawning
//! the worker — the spawned worker simply receives the same pointer
//! value. The caller must therefore keep the pointed-to memory valid
//! for the entire lifetime of the spawned task; the typical patterns
//! are a `static`, a `Vec<u8>` on the shared heap, or an
//! intentionally-leaked `Box<T>` scoped to the worker's run.

use core::fmt;

use crate::sys;

/// Errors returned by [`spawn_thread`] / [`spawn_thread_raw`]. Wraps
/// the host-reported negative code so mod authors can match on a
/// single error type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThreadError(i32);

impl ThreadError {
    pub fn new(code: i32) -> Self {
        Self(code)
    }

    pub fn code(&self) -> i32 {
        self.0
    }
}

impl fmt::Display for ThreadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "saga_thread_spawn failed (host code {})", self.0)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ThreadError {}

pub type ThreadResult<T> = Result<T, ThreadError>;

/// The signature a worker task must satisfy to be passed to
/// [`spawn_thread`].
pub type Worker = extern "C" fn(arg_ptr: *mut u8);

/// Spawn a Web Worker that will run `worker(arg_ptr)` on a fresh
/// thread. Returns the thread id (> 0) on success, or [`ThreadError`]
/// on failure.
///
/// The caller must keep the memory at `arg_ptr` valid for the entire
/// lifetime of the spawned worker (see module-level docs).
///
/// This function does not panic; host failure codes propagate via
/// [`ThreadError`].
pub fn spawn_thread(worker: Worker, arg_ptr: usize) -> ThreadResult<i32> {
    let entry_idx = worker as usize;
    spawn_thread_raw(entry_idx, arg_ptr)
}

/// Spawn a Web Worker by specifying the worker's WASM table index
/// directly. Most callers will prefer [`spawn_thread`]; this lower-level
/// form is useful when the worker was produced by something other
/// than a direct `extern "C" fn` (e.g. a function table you manage
/// manually, or a pointer obtained from a guest module).
pub fn spawn_thread_raw(entry_idx: usize, arg_ptr: usize) -> ThreadResult<i32> {
    let tid = unsafe { sys::saga_thread_spawn(entry_idx, arg_ptr) };
    if tid < 0 {
        return Err(ThreadError(tid));
    }
    Ok(tid)
}

/// Yield execution on the current worker. Maps to `Atomics.wait` or
/// equivalent in the host; always returns immediately.
pub fn yield_now() {
    unsafe { sys::saga_thread_yield() };
}
