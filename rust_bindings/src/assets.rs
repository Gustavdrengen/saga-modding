//! Safe wrappers around the `saga:assets` host import.
//!
//! Provides:
//!
//! - [`AssetHandle`]: a strongly-typed, RAII-managed handle to an
//!   opened asset. Closing is automatic on `Drop`.
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssetError {
    /// A non-positive handle (`<= 0`) from `open` indicates failure;
    /// the raw host code is preserved for diagnostics.
    OpenFailed(i32),

    /// Negative byte count from `read`.
    ReadFailed(i32),

    /// Caller buffer length did not match the asset's host-reported
    /// size when using [`AssetHandle::read_exact`].
    SizeMismatch { expected: usize, actual: usize },
}

impl AssetError {
    pub fn is_open_failure(&self) -> bool {
        matches!(self, AssetError::OpenFailed(_))
    }

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

pub type AssetResult<T> = Result<T, AssetError>;

/// An open Saga asset handle. `Drop` releases the host-side resource.
#[derive(Debug)]
pub struct AssetHandle {
    handle: i32,
}

impl AssetHandle {
    /// Open an asset by `saga://<mod-id>/<path>` URI. Returns
    /// `AssetError::OpenFailed` on a non-positive handle.
    pub fn open(uri: &str) -> AssetResult<Self> {
        // `&str` lives in WASM linear memory. The call is synchronous
        // and won't move the string, so passing the raw pointer + length
        // is sound without a defensive copy.
        let code = unsafe { sys::saga_asset_open(uri.as_ptr(), uri.len()) };
        if code <= 0 {
            return Err(AssetError::OpenFailed(code));
        }
        Ok(AssetHandle { handle: code })
    }

    /// Construct an `AssetHandle` from a raw, host-provided handle.
    /// Bypasses the normal URI parsing — intended for advanced use.
    pub fn from_raw(handle: i32) -> AssetResult<Self> {
        if handle <= 0 {
            return Err(AssetError::OpenFailed(handle));
        }
        Ok(AssetHandle { handle })
    }

    /// The raw host handle. Bypasses `Drop`-time closing.
    pub fn raw(&self) -> i32 {
        self.handle
    }

    /// Bytes reported by the host for this asset.
    pub fn size(&self) -> usize {
        unsafe { sys::saga_asset_get_size(self.handle) }
    }

    /// Read up to `buf.len()` bytes into `buf`. Returns bytes actually
    /// copied. `0` means end-of-stream (or an empty asset); a
    /// negative return maps to [`AssetError::ReadFailed`].
    pub fn read(&self, buf: &mut [u8]) -> AssetResult<usize> {
        let n = unsafe { sys::saga_asset_read(self.handle, buf.as_mut_ptr(), buf.len()) };
        if n < 0 {
            return Err(AssetError::ReadFailed(n));
        }
        Ok(n as usize)
    }

    /// Read the entire asset into a freshly allocated `Vec<u8>`.
    pub fn read_to_end(&self) -> AssetResult<Vec<u8>> {
        let size = self.size();
        let mut buf = vec![0u8; size];
        let n = self.read(&mut buf)?;
        // The host may legally return fewer bytes than `size`; truncate
        // to the actual count rather than over-report.
        buf.truncate(n);
        Ok(buf)
    }

    /// Read exactly `buf.len()` bytes, asserting both that the
    /// caller's buffer matches the host-reported size and that
    /// all bytes were delivered.
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
        unsafe { sys::saga_asset_close(self.handle) };
    }
}

// `AssetHandle` is intentionally neither `Clone` nor `Copy`. Two
// `Drop` calls would close the host resource twice; without `Clone` the
// compiler enforces single ownership.

pub fn open(uri: &str) -> AssetResult<AssetHandle> {
    AssetHandle::open(uri)
}

pub fn fetch_buffer(uri: &str) -> AssetResult<Vec<u8>> {
    let handle = AssetHandle::open(uri)?;
    handle.read_to_end()
}
