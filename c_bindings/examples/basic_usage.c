/* c_bindings/examples/basic_usage.c
 *
 * Minimal C exercise of the Saga host-import binding.
 *
 * Building it produces a `wasm32-unknown-unknown` object that
 * imports `saga:log.log` — an import only the Saga Launcher
 * (or a test harness that supplies a stub for `saga:log.log`)
 * can resolve.
 *
 * Build:
 *   clang --target=wasm32-unknown-unknown -nostdlib \
 *         -Wl,--no-entry -Wl,--allow-undefined \
 *         -I ../c_bindings \
 *         -o basic_usage.wasm basic_usage.c
 */

#include "saga.h"

/* Saga entrypoint matching a manifest's `entrypoint = "..."` field.
 * Keep this non-blocking per MOD_SPEC.md §6.1 — just emit a
 * single diagnostic line and return. */
__attribute__((export_name("com_example_basic_register")))
int com_example_basic_register(void) {
    static const unsigned char msg[] =
        "c_bindings/examples/basic_usage.c: register() called";
    /* Info level = 2 (see MOD_SPEC §4.3 / saga_log) */
    saga_log(2u, msg, sizeof msg - 1u);
    return 0;
}
