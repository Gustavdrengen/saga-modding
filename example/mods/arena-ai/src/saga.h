/* arena-ai/src/saga.h — Host-import declarations for the arena-ai mod.
 *
 * This mod does NOT include cross-mod extern declarations for peer
 * mods' exports. The Phase 1 base game (arena-renderer) reads physics
 * state via the merged WASM exports and ships those values to this
 * mod as plain arguments to `com_example_arena_ai_tick(...)`.
 *
 * Only the relevant host-import declarations live here. The
 * WASM-import attribute `import_module` binds the symbol to the
 * matching engine namespace on instantiation.
 */

#pragma once

#if defined(__wasm32__) || defined(__wasm__)
  #define SAGA_IMPORT_ATTR(module, name) \
      __attribute__((import_module(module), import_name(name)))
#else
  #define SAGA_IMPORT_ATTR(module, name)
#endif

#define HOST_IMPORT(module, name) SAGA_IMPORT_ATTR(module, "saga_" #name)

HOST_IMPORT("saga:thread", "thread_spawn")
extern int  saga_thread_spawn(unsigned int entry_idx, unsigned int arg_ptr);

HOST_IMPORT("saga:thread", "thread_yield")
extern void saga_thread_yield(void);

HOST_IMPORT("saga:log", "log")
extern void saga_log(unsigned int level, const unsigned char *msg_ptr, unsigned int msg_len);

#undef HOST_IMPORT
#undef SAGA_IMPORT_ATTR
