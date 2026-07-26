//! Safe wrappers around the `saga:time` host import.
//!
//! Reads the engine's high-resolution clock. Hosts typically implement
//! these as `performance.now()` deltas and a ticked counter.

use crate::sys;

/// Time elapsed since the previous frame, in seconds.
#[inline]
pub fn delta() -> f32 {
    unsafe { sys::saga_time_delta() }
}

/// Total engine execution time since boot, in seconds.
#[inline]
pub fn elapsed() -> f64 {
    unsafe { sys::saga_time_elapsed() }
}

/// Total fixed engine ticks executed.
#[inline]
pub fn ticks() -> u64 {
    unsafe { sys::saga_time_ticks() }
}
