//! Safe wrappers around the `saga:assets` host imports.
//!
//! Provides:
//!
//! - [`AssetHandle`]: a strongly-typed, RAII-managed handle to an opened
//!   asset. Closing is automatic on `Drop`.
//! - [`AssetError`] / [`AssetResult`]: typed error model.
//! - [`fetch_buffer`]: convenience to read the entire asset into a
//!   `Vec<u8>`.
//! - [`open`]: convenience to obtain an [`AssetHandle`].
//!
//! URIs follow the Saga Asset Protocol:
//! `saga://<mod-id>/<path-to-asset>` (e.g. `saga://self/textures/grass.png`).

use core::fmt;

use alloc::{vec, vec::Vec};

use crate::sys;

/// Errors returned by the Saga asset protocol wrappers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssetError {
    /// `saga_asset_open` rejected the URI. Per the spec, handles `<= 0`
    /// are considered a failure (not found, parser error, permission denied,
    /// etc). We preserve the raw `i32` for diagnostics where possible.
    OpenFailed(i32),

    /// `saga_asset_read` returned a negative byte count.
    ReadFailed(i32),

    /// The caller provided a buffer whose length does not match the asset's
    /// declared size when using [`AssetHandle::read_exact`].
    SizeMismatch { expected: usize, actual: usize },
}

impl AssetError {
    /// True if the error originated from the host's `saga_asset_open`.
    pub fn is_open_failure(&self) -> bool {
        matches!(self, AssetError::OpenFailed(_))
    }

    /// True if the error originated from the host's `saga_asset_read`.
    pub fn is_read_failure(&self) -> bool {
        matches!(self, AssetError::ReadFailed(_))
    }
}

impl fmt::Display for AssetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AssetError::OpenFailed(code) => {
                write!(f, "saga_asset_open failed (host code {code})")
            }
            AssetError::ReadFailed(code) => {
                write!(f, "saga_asset_read failed (host code {code})")
            }
            AssetError::SizeMismatch { expected, actual } => write!(
                f,
                "asset size mismatch: caller buffer was {expected} bytes, \
                 host-reported asset size was {actual} bytes"
            ),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for AssetError {}

/// Convenience result alias for asset operations.
pub type AssetResult<T> = Result<T, AssetError>;

/// An open Saga asset handle. Calling `Drop` will close the handle on
/// the host side.
#[derive(Debug)]
pub struct AssetHandle {
    handle: i32,
}

impl AssetHandle {
    /// Open an asset using a Saga Asset URI of the form
    /// `saga://<mod-id>/<path>`.
    ///
    /// Per the Saga spec, a non-positive handle indicates failure.
    pub fn open(uri: &str) -> AssetResult<Self> {
        // `&str` lives in WASM linear memory. Since the host call is
        // synchronous and we don't move the string during it, passing the
        // raw pointer + length is sound without a copy.
        let code = unsafe { sys::saga_asset_open(uri.as_ptr(), uri.len()) };
        if code <= 0 {
            return Err(AssetError::OpenFailed(code));
        }
        Ok(AssetHandle { handle: code })
    }

    /// Construct an `AssetHandle` from a raw, host-provided handle. This
    /// is intended for advanced use (e.g. when handing an already-open
    /// asset off into `AssetHandle` for RAII management).
    pub fn from_raw(handle: i32) -> AssetResult<Self> {
        if handle <= 0 {
            return Err(AssetError::OpenFailed(handle));
        }
        Ok(AssetHandle { handle })
    }

    /// The raw host handle. Bypasses `Drop`-time closing — handle with care.
    pub fn raw(&self) -> i32 {
        self.handle
    }

    /// Query the byte size of the asset (host-reported).
    pub fn size(&self) -> usize {
        unsafe { sys::saga_asset_get_size(self.handle) }
    }

    /// Read up to `buf.len()` bytes into `buf`. Returns the number of
    /// bytes actually copied. Zero means end-of-stream (or an empty
    /// asset); a negative value is reported as [`AssetError::ReadFailed`].
    pub fn read(&self, buf: &mut [u8]) -> AssetResult<usize> {
        let n = unsafe { sys::saga_asset_read(self.handle, buf.as_mut_ptr(), buf.len()) };
        if n < 0 {
            return Err(AssetError::ReadFailed(n));
        }
        Ok(n as usize)
    }

    /// Read the entire asset into a newly allocated `Vec<u8>`.
    pub fn read_to_end(&self) -> AssetResult<Vec<u8>> {
        let size = self.size();
        let mut buf = vec![0u8; size];
        let n = self.read(&mut buf)?;
        // The host may legally read fewer bytes than `size`; truncate to
        // the actual count.
        buf.truncate(n);
        Ok(buf)
    }

    /// Read exactly `buf.len()` bytes, asserting that it matches the
    /// host-reported size and that all bytes were delivered.
    pub fn read_exact(&self, buf: &mut [u8]) -> AssetResult<()> {
        let size = self.size();
        if buf.len() != size {
            return Err(AssetError::SizeMismatch {
                expected: buf.len(),
                actual: size,
            });
        }
        let n = self.read(buf)?;
        if n != size {
            return Err(AssetError::SizeMismatch {
                expected: size,
                actual: n,
            });
        }
        Ok(())
    }
}

impl Drop for AssetHandle {
    fn drop(&mut self) {
        // Closing must always succeed even on non-WASM stubs (no-op).
        unsafe { sys::saga_asset_close(self.handle) };
    }
}

// AssetHandle is intentionally neither `Clone` nor `Copy` — the helper does
// not `#[derive(Clone, Copy)]` so two `Drop` calls cannot accidentally close
// the underlying host resource twice.

/// Convenience: open an asset URI and return an RAII-managed handle.
///
/// Equivalent to `AssetHandle::open(uri)`.
pub fn open(uri: &str) -> AssetResult<AssetHandle> {
    AssetHandle::open(uri)
}

/// Convenience: open an asset URI and read it entirely into a `Vec<u8>`.
///
/// This opens -> queries size -> reads -> closes. For repeated access,
/// prefer opening an [`AssetHandle`] explicitly and reusing it.
pub fn fetch_buffer(uri: &str) -> AssetResult<Vec<u8>> {
    let handle = AssetHandle::open(uri)?;
    handle.read_to_end()
}
