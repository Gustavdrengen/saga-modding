/* arena-ai — Saga mod (C → WebAssembly).
 *
 * This module demonstrates direct cross-module calls. The linker/merger
 * resolves the physics imports to the Rust physics exports in the final
 * merged WebAssembly module. No JavaScript call is involved.
 */

#include "saga.h"

/* Public C ABI exported by arena-physics. The implementation happens to be
 * Rust, but this module only depends on WebAssembly function signatures. */
extern float com_example_arena_physics_get_ball_x(void);
extern float com_example_arena_physics_get_ball_y(void);
extern float com_example_arena_physics_get_ball_vx(void);
extern float com_example_arena_physics_get_ball_vy(void);
extern void com_example_arena_physics_set_ai_x(float value);

static float g_ai_x = 400.0f;

static unsigned int slen(const unsigned char *s) {
    unsigned int n = 0u;
    while (s[n] != 0u) ++n;
    return n;
}

static void say(unsigned int level, const char *msg) {
    saga_log(level, (const unsigned char *)msg, slen((const unsigned char *)msg));
}

__attribute__((export_name("com_example_arena_ai_register")))
int com_example_arena_ai_register(void) {
    g_ai_x = 400.0f;
    say(2u, "arena-ai registered");
    return 0;
}

__attribute__((export_name("com_example_arena_ai_tick")))
void com_example_arena_ai_tick(float bx, float by, float bvx, float bvy, float dt) {
    float landing;
    if (bvy >= -1.0f) {
        landing = 400.0f;
    } else {
        float t = (22.0f - by) / -bvy;
        if (t < 0.0f) t = 0.0f;
        landing = bx + bvx * t;
        if (landing < 54.0f) landing = 54.0f;
        if (landing > 746.0f) landing = 746.0f;
    }

    float step = dt > 0.05f ? 0.05f : dt;
    g_ai_x += (landing - g_ai_x) * 6.0f * step;
    com_example_arena_physics_set_ai_x(g_ai_x);
}

/* This entrypoint proves that peer calls are resolved in Wasm, not through
 * JavaScript. It reads the Rust physics state directly. */
__attribute__((export_name("com_example_arena_ai_direct_sample")))
float com_example_arena_ai_direct_sample(void) {
    return com_example_arena_physics_get_ball_x()
         + com_example_arena_physics_get_ball_y()
         + com_example_arena_physics_get_ball_vx()
         + com_example_arena_physics_get_ball_vy();
}

__attribute__((export_name("com_example_arena_ai_get_ai_x")))
float com_example_arena_ai_get_ai_x(void) {
    return g_ai_x;
}

__attribute__((export_name("com_example_arena_ai_reset_ai")))
void com_example_arena_ai_reset_ai(void) {
    g_ai_x = 400.0f;
    com_example_arena_physics_set_ai_x(g_ai_x);
}

__attribute__((export_name("com_example_arena_ai_worker")))
void com_example_arena_ai_worker(unsigned int arg_ptr) {
    (void)arg_ptr;
    saga_thread_yield();
}
