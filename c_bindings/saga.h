/* c_bindings/saga.h — Saga standard library host imports for C and C++.
 *
 * Single header consumable from both languages:
 *
 *     #include "saga.h"   // C
 *     #include "saga.h"   // C++
 *
 * Mirrors the host-binding declarations in MOD_SPEC.md §4 and the
 * `unsafe extern "C"` blocks in rust_bindings/src/sys.rs. Every
 * declaration here corresponds to a real WebAssembly host import
 * that the Saga Launcher provides under the `saga:*` namespaces;
 * on non-wasm targets, the import attributes compile away so
 * consuming code remains buildable as a native binary for tooling.
 *
 *
 * === Macro contract =================================================
 *
 * Each declaration is preceded by:
 *
 *     SAGA_HOST_IMPORT("saga:log", log)
 *     extern void saga_log(uint32_t level, ...);
 *
 * The **base symbol** is passed as a BARE IDENTIFIER (the `log`
 * token), NOT as a pre-quoted string. The macro stringifies it
 * via the `#` preprocessor operator and adjacent-string-literal
 * concatenation assembles the canonical import name:
 *
 *     "saga_" #log       →   "saga_" "log"       →   "saga_log"   ✓
 *
 * Pre-quoting the symbol — i.e.
 * `SAGA_HOST_IMPORT("saga:log", "log")` — causes the `#` operator
 * to escape the inner quotes, producing an import name like
 * `saga_"log"` (with literal quote characters) that the Saga
 * Launcher will never resolve. This is the bug
 * `example/mods/arena-ai/src/saga.h` shipped before this header
 * existed.
 *
 * === ABI ===========================================================
 *
 * Types match the `extern "C"` signatures in
 * `rust_bindings/src/sys.rs` 1-for-1:
 *
 *     usize     ↔   size_t
 *     u8 *      ↔   uint8_t *
 *     u32       ↔   uint32_t
 *     i32       ↔   int32_t
 *     u64       ↔   uint64_t
 *     f32       ↔   float
 *     f64       ↔   double
 *
 * On `wasm32-unknown-unknown` all of those map to identical
 * WebAssembly value types, so callers can freely mix
 * `<stdint.h>`/`size_t` and the legacy `unsigned int` /
 * `unsigned char` typedefs used elsewhere in this repo without
 * any runtime difference.
 */

#pragma once
#ifndef SAGA_H_
#define SAGA_H_

#include <stddef.h>
#include <stdint.h>

#if defined(__cplusplus)
extern "C" {
#endif

/* ---------- import-attribute plumbing ---------------------------------- */

#if defined(__wasm32__) || defined(__wasm__)
  #define SAGA_IMPORT_ATTR(module, name) \
      __attribute__((import_module(module), import_name(name)))
#else
  #define SAGA_IMPORT_ATTR(module, name)
#endif

/* Macro: take the BASE SYMBOL as a bare identifier (e.g. `log`),
 * not a string literal. The `#` stringify + adjacent literal
 * concatenation does the canonical "saga_<name>" assembly.
 * See the contract block at the top of the file. */
#define SAGA_HOST_IMPORT(module, name) \
    SAGA_IMPORT_ATTR(module, "saga_" #name)


/* ---------- saga:log ----------------------------------------------------- */

SAGA_HOST_IMPORT("saga:log", log)
extern void saga_log(uint32_t level,
                     const uint8_t *msg_ptr,
                     size_t msg_len);


/* ---------- saga:thread -------------------------------------------------- */

SAGA_HOST_IMPORT("saga:thread", thread_spawn)
extern int32_t saga_thread_spawn(size_t entry_idx,
                                 size_t arg_ptr);

SAGA_HOST_IMPORT("saga:thread", thread_yield)
extern void saga_thread_yield(void);


/* ---------- saga:time ---------------------------------------------------- */

SAGA_HOST_IMPORT("saga:time", time_delta)
extern float saga_time_delta(void);

SAGA_HOST_IMPORT("saga:time", time_elapsed)
extern double saga_time_elapsed(void);

SAGA_HOST_IMPORT("saga:time", time_ticks)
extern uint64_t saga_time_ticks(void);


/* ---------- saga:assets -------------------------------------------------- */

SAGA_HOST_IMPORT("saga:assets", asset_open)
extern int32_t saga_asset_open(const uint8_t *uri_ptr,
                               size_t uri_len);

SAGA_HOST_IMPORT("saga:assets", asset_get_size)
extern size_t saga_asset_get_size(int32_t handle);

SAGA_HOST_IMPORT("saga:assets", asset_read)
extern int32_t saga_asset_read(int32_t handle,
                               uint8_t *dest_ptr,
                               size_t length);

SAGA_HOST_IMPORT("saga:assets", asset_close)
extern void saga_asset_close(int32_t handle);


/* ---------- saga:storage ------------------------------------------------- */

SAGA_HOST_IMPORT("saga:storage", save_list)
extern int32_t saga_save_list(uint8_t *out_buf,
                              size_t max_len);

SAGA_HOST_IMPORT("saga:storage", save_read_meta)
extern int32_t saga_save_read_meta(const uint8_t *save_id_ptr,
                                   size_t save_id_len,
                                   uint8_t *meta_buf,
                                   size_t max_len);

SAGA_HOST_IMPORT("saga:storage", save_read)
extern int32_t saga_save_read(const uint8_t *save_id_ptr,
                              size_t save_id_len,
                              uint8_t *dest_ptr,
                              size_t max_len);

SAGA_HOST_IMPORT("saga:storage", save_write)
extern int32_t saga_save_write(const uint8_t *save_id_ptr,
                               size_t save_id_len,
                               const uint8_t *data_ptr,
                               size_t data_len,
                               const uint8_t *meta_ptr,
                               size_t meta_len);

SAGA_HOST_IMPORT("saga:storage", save_delete)
extern void saga_save_delete(const uint8_t *save_id_ptr,
                             size_t save_id_len);


#if defined(__cplusplus)
} /* extern "C" */
#endif

#endif /* SAGA_H_ */
