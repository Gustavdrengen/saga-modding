/* arena-ai — Saga mod (C → WebAssembly).
 *
 * Implements the AI opponent paddle for the Saga example game.
 *
 * Per MOD_SPEC §8.5, this mod has ZERO cross-module `extern` declarations.
 * Ball state is delivered as plain arguments to `tick(...)`, and the
 * orchestrator (arena-renderer) wires it up by reading the merged
 * `Saga.wasmExports` for physics and calling into this mod's exports.
 *
 * Exports (prefixed with the mod id per the §3.2 / §8.5 convention):
 *   - arena_ai_init           — §6 entry point.
 *   - com_example_arena_ai_tick          — per-frame advance.
 *   - com_example_arena_ai_get_ai_x      — read smoothed paddle x.
 *   - com_example_arena_ai_reset_ai      — re-centre.
 *   - worker                  — long-horizon lookahead (§4.2 worker body).
 *
 * Built by example/build.sh:
 *   clang --target=wasm32-unknown-unknown -O2 -nostdlib \
 *         -Wl,--no-entry -Wl,--import-memory -Wl,--import-table \
 *         -Wl,--allow-undefined \
 *         -Wl,--export=arena_ai_init \
 *         -Wl,--export=com_example_arena_ai_tick \
 *         -Wl,--export=com_example_arena_ai_get_ai_x \
 *         -Wl,--export=com_example_arena_ai_reset_ai \
 *         -Wl,--export=worker \
 *         -o ../module.wasm main.c
 */

#include "saga.h"

/* ------- module state ---------------------------------------------------- */
static float g_ai_x = 400.0f;   /* protected via WASM single-threaded guest */

/* -- §6 entry point. ----------------------------------------------------- */
__attribute__((export_name("arena_ai_init")))
int arena_ai_init(void) {
    g_ai_x = 400.0f;
    return 1;
}

/* -- §8.1 / §8.5 per-frame advance. -------------------------------------
 *
 * Ball state comes in as arguments — no `extern` peer-mod calls. Returns
 * nothing; the orchestrator polls `get_ai_x()` after the call and pipes
 * the value into physics via the `set_ai_x` setter.
 */
__attribute__((export_name("com_example_arena_ai_tick")))
void com_example_arena_ai_tick(float bx, float by, float bvx, float bvy, float dt) {
    /* Predict landing on the AI line at y = 22. */
    float landing;
    if (bvy >= -1.0f) {
        landing = 400.0f;                 /* ball moving away / flat — re-centre */
    } else {
        float t = (22.0f - by) / -bvy;
        if (t < 0.0f) t = 0.0f;
        landing = bx + bvx * t;
        if (landing < 54.0f)  landing = 54.0f;
        if (landing > 746.0f) landing = 746.0f;
    }

    /* Smooth-slide toward the predicted intercept (~6/s). */
    float step = dt;
    if (step > 0.05f) step = 0.05f;
    g_ai_x += (landing - g_ai_x) * 6.0f * step;

    /* Occasional worker spawn — the worker body is a real
     * `worker(arg_ptr)` symbol that runs on a launcher Worker. In this
     * example standalone build it's never actually invoked because no
     * `saga_thread_spawn` host is wired; the symbol just has to exist
     * so the engine's table-merger sees it. */
    static int last_spawn_ticks = 0;
    last_spawn_ticks += 1;
    if ((last_spawn_ticks & 0x3F) == 0) {
        saga_thread_spawn(0u, 0u);
    }
}

/* -- Read-only getter used by the orchestrator. ------------------------ */
__attribute__((export_name("com_example_arena_ai_get_ai_x")))
float com_example_arena_ai_get_ai_x(void) {
    return g_ai_x;
}

/* -- Reset (called by orchestrator when a new round starts). ------------ */
__attribute__((export_name("com_example_arena_ai_reset_ai")))
void com_example_arena_ai_reset_ai(void) {
    g_ai_x = 400.0f;
}

/* -- §4.2 worker body. Demonstrates `saga_thread_yield` cooperation. ----
 * The wrapper is invoked on a Worker thread when the host dispatches a
 * `saga_thread_spawn(worker_idx, arg_ptr)`. The arg blob is opaque here.
 */
__attribute__((export_name("worker")))
void worker(unsigned int arg_ptr) {
    (void)arg_ptr;
    /* Cooperatively interleaved micro-tasks. Real Saga engines would
     * forward `arg_ptr` to a struct in linear memory here. */
    saga_thread_yield();
    saga_thread_yield();
    saga_thread_yield();
}
