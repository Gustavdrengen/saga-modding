/* arena-ai — Saga mod (C → WebAssembly).
 *
 * Drives the AI opponent (top) paddle for the example game:
 *
 *   - A lightweight interceptor that takes the ball state as plain
 *     `tick(bx, by, bvx, bvy, dt)` arguments and smooth-slides `g_ai_x`
 *     toward the predicted intercept.
 *   - A `worker(arg_ptr)` body released into the WASM function
 *     table for any launcher Worker to dispatch via `saga_thread_spawn`.
 *
 * The mod declares ZERO cross-mod `extern` bindings; the orchestrator
 * (arena-renderer) reads physics state through the engine's merged
 * exports table and arrives at this mod's exports as plain arguments.
 *
 * Exports:
 *   com_example_arena_ai_register           — Phase 1 entrypoint.
 *   com_example_arena_ai_tick               — per-frame advance.
 *   com_example_arena_ai_get_ai_x           — read smoothed paddle x.
 *   com_example_arena_ai_reset_ai           — re-centre.
 *   worker                                  — saga:thread worker body.
 */

#include "saga.h"

/* ------- module state ---------------------------------------------------- */
static float g_ai_x = 400.0f;

/* ------- tiny helpers ---------------------------------------------------- */
static unsigned int slen(const unsigned char *s) {
    unsigned int n = 0u;
    while (s[n] != 0u) ++n;
    return n;
}
static void say(unsigned int level, const char *msg) {
    saga_log(level, (const unsigned char *)msg, slen((const unsigned char *)msg));
}

/* ------- Phase 1 registration entrypoint -------------------------------- */
__attribute__((export_name("com_example_arena_ai_register")))
int com_example_arena_ai_register(void) {
    /* Phase 1 must be non-blocking. The only work is resetting state
     * and emitting a diagnostic through saga:log so a real Saga
     * engine's launcher log shows this mod wired up successfully. */
    g_ai_x = 400.0f;
    say(2u /* Info */, "arena-ai registered");
    return 0;
}

/* ------- per-frame advance. Called by the orchestrator's saga_start ----- */
__attribute__((export_name("com_example_arena_ai_tick")))
void com_example_arena_ai_tick(float bx, float by, float bvx, float bvy, float dt) {
    float landing;
    if (bvy >= -1.0f) {
        landing = 400.0f;
    } else {
        float t = (22.0f - by) / -bvy;
        if (t < 0.0f) t = 0.0f;
        landing = bx + bvx * t;
        if (landing < 54.0f)  landing = 54.0f;
        if (landing > 746.0f) landing = 746.0f;
    }

    float step = dt;
    if (step > 0.05f) step = 0.05f;
    g_ai_x += (landing - g_ai_x) * 6.0f * step;

    /* The worker body lives in this module's WASM function table. The
     * host dispatches it via `saga_thread_spawn(worker, arg_ptr)`. In
     * this standalone build no host is wired, so this call is a
     * defensive staff-and-straw exercise — table-resident symbols
     * must exist for the engine's table-merge pass to see them. */
    static unsigned int last_spawn_ticks = 0u;
    last_spawn_ticks += 1u;
    if ((last_spawn_ticks & 0x3Fu) == 0u) {
        saga_thread_spawn(0u, 0u);
    }
}

/* ------- read-only getter exposed to the orchestrator ------------------- */
__attribute__((export_name("com_example_arena_ai_get_ai_x")))
float com_example_arena_ai_get_ai_x(void) {
    return g_ai_x;
}

/* ------- reset (called by orchestrator on a new round) ------------------ */
__attribute__((export_name("com_example_arena_ai_reset_ai")))
void com_example_arena_ai_reset_ai(void) {
    g_ai_x = 400.0f;
}

/* ------- saga:thread worker body --------------------------------------- */
__attribute__((export_name("worker")))
void worker(unsigned int arg_ptr) {
    (void)arg_ptr;
    /* Cooperative yield only; the engine's runtime forwards `arg_ptr`
     * to a struct in linear memory in real Saga deployments. */
    saga_thread_yield();
    saga_thread_yield();
    saga_thread_yield();
}
