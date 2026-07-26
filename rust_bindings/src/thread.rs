//! Safe wrappers around the `saga:thread` host imports.
//!
//! Provides [`spawn_thread`] / [`spawn_thread_raw`] for dispatching work
//! onto a fresh Web Worker, plus [`yield_now`] for cooperative scheduling.
//!
//! # Function pointer → table index
//!
//! On the `wasm32-unknown-unknown` target, an `extern "C" fn(...)` is
//! materialised as a slot in the WASM indirect-call table. The Saga runtime
//! indexes into that table via `entry_idx`, so we can reuse the function
//! pointer as the index directly: `worker as usize`.
//!
//! # Lifetime of `arg_ptr`
//!
//! The Saga runtime does **not** copy the argument buffer when spawning a
//! worker — the spawned `Worker` simply receives the same pointer value.
//! Because of this the *caller* is responsible for keeping the pointed-to
//! memory valid for the entire lifetime of the spawned task. The typical
//! pattern is to back `arg_ptr` with a `static`, a `Vec<u8>` allocated on
//! the heap (which lives in the shared linear memory), or any `Box`-owned
//! value that is intentionally leaked for the run-time of the worker.

use core::fmt;

use crate::sys;

/// Errors returned by [`spawn_thread`]/[`spawn_thread_raw`].
///
/// Wraps the host-reported negative code so that mod authors can match on
/// a single error type rather than carrying host integers around.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThreadError(i32);

impl ThreadError {
    /// Construct from the raw host-reported error code.
    pub fn new(code: i32) -> Self {
        Self(code)
    }

    /// The raw host error code.
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

/// Convenience result alias for thread operations.
pub type ThreadResult<T> = Result<T, ThreadError>;

/// The signature a worker function must satisfy to be spawned.
///
/// On WASM, the function pointer of such a function is also its table
/// index — see [`spawn_thread`].
pub type Worker = extern "C" fn(arg_ptr: *mut u8);

/// Spawn a Web Worker that will run `worker(arg_ptr)` on a fresh thread.
///
/// Returns the thread id (> 0) on success, or [`ThreadError`] on failure.
///
/// **The caller must ensure** that the memory at `arg_ptr` remains valid
/// for the entire lifetime of the spawned worker — see the module-level
/// docs for the recommended patterns.
///
/// # Panics
///
/// This function does not panic; it surfaces host failure codes via
/// [`ThreadError`].
pub fn spawn_thread(worker: Worker, arg_ptr: usize) -> ThreadResult<i32> {
    let entry_idx = worker as usize;
    spawn_thread_raw(entry_idx, arg_ptr)
}

/// Spawn a Web Worker by specifying the worker's table index directly.
///
/// Most callers will prefer [`spawn_thread`]. This lower-level form is
/// useful when the worker was produced by something other than a direct
/// `extern "C" fn` (e.g. a function table you manage manually, or a pointer
/// obtained from a guest module).
///
/// **The caller must ensure** that the memory at `arg_ptr` remains valid
/// for the entire lifetime of the spawned worker.
pub fn spawn_thread_raw(entry_idx: usize, arg_ptr: usize) -> ThreadResult<i32> {
    let tid = unsafe { sys::saga_thread_spawn(entry_idx, arg_ptr) };
    if tid < 0 {
        return Err(ThreadError(tid));
    }
    Ok(tid)
}

/// Yield execution on the current worker.
///
/// This is a cooperative hint to the scheduler; on the WASM target it maps
/// to `Atomics.wait` or equivalent busy-wait, depending on the Saga
/// runtime configuration. Regardless, this function returns immediately.
pub fn yield_now() {
    unsafe { sys::saga_thread_yield() };
}
