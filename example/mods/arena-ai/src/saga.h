/* saga.h — Saga host-import declarations used by `arena-ai/src/main.c`.
 *
 * Per MOD_SPEC §8.5, this mod does NOT include cross-module `extern`
 * declarations for peer mods' exports. The orchestrator (arena-renderer)
 * reads physics state via the merged `Saga.wasmExports` and feeds the
 * values as plain arguments to this mod's `tick(bx, by, bvx, bvy, dt)`.
 *
 * Only the §4.2 saga:thread host import is bound here so the mod can
 * demonstratively `saga_thread_spawn` a worker body in the §4.2 flow.
 */

#pragma once

#if defined(__wasm32__) || defined(__wasm__)
  #define SAGA_IMPORT_ATTR(module, name) \
      __attribute__((import_module(module), import_name(name)))
#else
  #define SAGA_IMPORT_ATTR(module, name)
#endif

#define SAGA_THREAD_IMPORT(name) SAGA_IMPORT_ATTR("saga:thread", "saga_" name)

/* -- §4.2 saga:thread ----------------------------------------------------- */
SAGA_THREAD_IMPORT("thread_spawn")
extern int  saga_thread_spawn(unsigned int entry_idx, unsigned int arg_ptr);
SAGA_THREAD_IMPORT("thread_yield")
extern void saga_thread_yield(void);

#undef SAGA_THREAD_IMPORT
#undef SAGA_IMPORT_ATTR
