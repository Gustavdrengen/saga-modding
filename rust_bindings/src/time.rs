//! Safe wrappers around the `saga:time` host import.
//!
//! Hosts typically back these with `performance.now()` for the
//! monotonic counter and `Date.now()` for the wall-clock read.

use crate::sys;

/// Current wall-clock timestamp, in milliseconds since the Unix epoch.
#[inline]
pub fn now() -> u64 {
    unsafe { sys::saga_time_now() }
}

/// Monotonic seconds elapsed since the Saga session started.
#[inline]
pub fn elapsed() -> f64 {
    unsafe { sys::saga_time_elapsed() }
}
