//! Safe wrappers around the `saga:log` host import.
//!
//! Provides [`log`] plus an [`emit`] helper that takes a
//! [`core::fmt::Arguments`] for in-place formatting with no allocation.

use core::fmt;

use crate::sys;

/// Severity tag passed to the host log sink.
///
/// The numeric `u32` values are part of the host ABI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum LogLevel {
    Trace = 0,
    Debug = 1,
    Info  = 2,
    Warn  = 3,
    Error = 4,
}

/// Write a string slice to the host log at the given severity.
pub fn log(level: LogLevel, msg: &str) {
    unsafe { sys::saga_log(level as u32, msg.as_ptr(), msg.len()) };
}

/// Write a formatted message to the host log at the given severity.
///
/// `format_args!`-style in-place formatting means no allocation.
pub fn emit(level: LogLevel, args: fmt::Arguments<'_>) {
    const BUF: usize = 1024;
    let mut storage = [0u8; BUF];

    // Scope `Sink` strictly inside this block so its mutable borrow of
    // `storage` ends before we re-slice the buffer to hand off to the
    // host call below.
    let written: usize = {
        struct Sink<'a> {
            buf: &'a mut [u8],
            pos: usize,
        }
        impl fmt::Write for Sink<'_> {
            fn write_str(&mut self, s: &str) -> fmt::Result {
                let rem = &mut self.buf[self.pos..];
                if rem.len() < s.len() {
                    return Err(fmt::Error);
                }
                rem[..s.len()].copy_from_slice(s.as_bytes());
                self.pos += s.len();
                Ok(())
            }
        }

        let mut sink = Sink { buf: &mut storage[..], pos: 0 };
        if fmt::write(&mut sink, args).is_err() {
            log(level, "<message exceeded 1KiB log buffer; truncated>");
            return;
        }
        sink.pos
    };

    let s = core::str::from_utf8(&storage[..written])
        .unwrap_or("<log message contained invalid UTF-8>");
    log(level, s);
}
