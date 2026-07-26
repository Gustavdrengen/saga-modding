//! Safe wrappers around the `saga:storage` host import.
//!
//! Save-file CRUD; each operation takes a save identifier plus a payload
//! and (for writes) a metadata record. The host mediates persistence
//! across sessions.

use core::fmt;

use alloc::borrow::ToOwned;
use alloc::{string::String, vec, vec::Vec};

use crate::sys;

/// Errors returned by the [`storage`] API. Wraps the host's negative
/// `i32` return codes so callers can match on a single enum.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageError {
    /// Host reported a non-negative status that the wrapper doesn't
    /// recognise.
    BadStatus(i32),
    /// The output buffer was too small for the host's payload.
    BufferTooSmall { requested: usize, capacity: usize },
    /// `saga_save_read_meta` returned bytes that were not valid UTF-8.
    InvalidUtf8,
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageError::BadStatus(code) => write!(f, "storage host reported status {code}"),
            StorageError::BufferTooSmall { requested, capacity } => write!(
                f,
                "storage buffer too small: host needed {requested} bytes, got {capacity}"
            ),
            StorageError::InvalidUtf8 => write!(f, "storage host returned non-UTF-8 metadata"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for StorageError {}

pub type StorageResult<T> = Result<T, StorageError>;

fn check(code: i32) -> StorageResult<usize> {
    if code < 0 {
        return Err(StorageError::BadStatus(code));
    }
    Ok(code as usize)
}

fn as_str(buf: &[u8]) -> StorageResult<&str> {
    core::str::from_utf8(buf).map_err(|_| StorageError::InvalidUtf8)
}

/// Implementation-defined host return code that this wrapper treats as
/// "write buffer was too small, please try again with more capacity".
///
/// **Heuristic only.** The Saga spec deliberately leaves negative host
/// return codes engine-defined, so this constant is a best-effort guess
/// at one common engine convention. Real hosts may return a different
/// negative value for the same condition, in which case the auto-grow
/// ladder below will not advance and the call will surface as a
/// [`StorageError::BadStatus`] instead. Callers that need
/// deterministic sizing should use circular double-buffering on their
/// side rather than relying on auto-grow.
const HOST_BUF_TOO_SMALL_HEURISTIC_CODE: i32 = -2;

/// Hard upper bound on the auto-grow ladder. Once the buffer reaches
/// this size the wrapper stops growing and surfaces a
/// [`StorageError::BadStatus`].
const MAX_GROW_BYTES: usize = 64 * 1024;

/// Manifest of every save the host currently has on disk. Returns the
/// raw JSON document the host emitted; the schema is engine-defined.
pub fn list() -> StorageResult<String> {
    let mut buf = vec![0u8; 1024];
    loop {
        let n = unsafe { sys::saga_save_list(buf.as_mut_ptr(), buf.len()) };
        if n == HOST_BUF_TOO_SMALL_HEURISTIC_CODE && buf.len() < MAX_GROW_BYTES {
            buf.resize(buf.len() * 2, 0);
            continue;
        }
        if n < 0 {
            return Err(StorageError::BadStatus(n));
        }
        buf.truncate(n as usize);
        return as_str(&buf).map(str::to_owned);
    }
}

/// Fetch the metadata record for a given save id. Returns the raw JSON
/// the host emitted.
pub fn read_meta(save_id: &str) -> StorageResult<String> {
    fetch_into_string(save_id, |id_ptr, id_len, out, cap| unsafe {
        sys::saga_save_read_meta(id_ptr, id_len, out, cap)
    })
}

/// Fetch the binary/text payload of a given save id.
pub fn read(save_id: &str) -> StorageResult<Vec<u8>> {
    fetch_into_vec(save_id, |id_ptr, id_len, out, cap| unsafe {
        sys::saga_save_read(id_ptr, id_len, out, cap)
    })
}

/// Write `data` and its accompanying JSON `meta` record under
/// `save_id`. The host overwrites any existing entry with the same id.
pub fn write(save_id: &str, data: &[u8], meta: &str) -> StorageResult<()> {
    let n = unsafe {
        sys::saga_save_write(
            save_id.as_ptr(),
            save_id.len(),
            data.as_ptr(),
            data.len(),
            meta.as_ptr(),
            meta.len(),
        )
    };
    check(n).map(|_| ())
}

/// Delete the save entry identified by `save_id`. Returns `Ok(())` if
/// the entry was gone after the call (idempotent).
pub fn delete(save_id: &str) -> StorageResult<()> {
    let n = unsafe { sys::saga_save_delete(save_id.as_ptr(), save_id.len()) };
    check(n).map(|_| ())
}

fn fetch_into_string(
    save_id: &str,
    call: impl Fn(*const u8, usize, *mut u8, usize) -> i32,
) -> StorageResult<String> {
    let mut buf = vec![0u8; 1024];
    loop {
        let n = call(save_id.as_ptr(), save_id.len(), buf.as_mut_ptr(), buf.len());
        if n == HOST_BUF_TOO_SMALL_HEURISTIC_CODE && buf.len() < MAX_GROW_BYTES {
            buf.resize(buf.len() * 2, 0);
            continue;
        }
        if n < 0 {
            return Err(StorageError::BadStatus(n));
        }
        buf.truncate(n as usize);
        return as_str(&buf).map(str::to_owned);
    }
}

fn fetch_into_vec(
    save_id: &str,
    call: impl Fn(*const u8, usize, *mut u8, usize) -> i32,
) -> StorageResult<Vec<u8>> {
    let mut buf = vec![0u8; 1024];
    loop {
        let n = call(save_id.as_ptr(), save_id.len(), buf.as_mut_ptr(), buf.len());
        if n == HOST_BUF_TOO_SMALL_HEURISTIC_CODE && buf.len() < MAX_GROW_BYTES {
            buf.resize(buf.len() * 2, 0);
            continue;
        }
        if n < 0 {
            return Err(StorageError::BadStatus(n));
        }
        buf.truncate(n as usize);
        return Ok(buf);
    }
}
