//! `arena-physics` — Saga mod (Rust → WebAssembly).
//!
//! Ball + player-paddle physics for the Saga example game.
//!
//! Per `MOD_SPEC.md` §8.5, this mod exports a small set of long-prefixed
//! read-only getters plus setters for the orchestrator (the JS renderer).
//! All inter-module reads/writes go through function calls: the C mod
//! takes ball state as `tick` arguments; the JS mod reads via
//! `Saga.wasmExports.com_example_arena_physics_*`.
//!
//! Built by `example/build.sh`:
//!     cargo build --target wasm32-unknown-unknown --release
//! → `./module.wasm`.
//!
//! We deliberately stay `extern crate alloc;`-free so we don't have to
//! ship a `#[global_allocator]` in this crate. Saga's stdlib brings one
//! when it's needed; for the example we use only stack locals.

#![no_std]

// =============================================================================
// Game state. Owned exclusively by this mod. Cross-mod access is via the
// getter/setter exports declared below — never via direct linear-memory
// arithmetic.
// =============================================================================

static mut STATE:           u32   = 0; // 0=idle/serving, 1=rally, 2=paused
static mut BALL_X:          f32   = 400.0;
static mut BALL_Y:          f32   = 250.0;
static mut BALL_VX:         f32   = 0.0;
static mut BALL_VY:         f32   = 0.0;
static mut BALL_R:          f32   = 9.0;

static mut PLAYER_X:        f32   = 400.0;
static mut PLAYER_SCORE:    u32   = 0;
static mut RALLY_HITS:      u32   = 0;

static mut AI_X:            f32   = 400.0; // written by orchestrator after polling arena-ai.
static mut AI_SCORE:        u32   = 0;

static mut INPUT_DX:        f32   = 0.0;  // -1 .. +1, written by orchestrator from keyboard.

// =============================================================================
// §6 entrypoint.
// =============================================================================

#[unsafe(no_mangle)]
pub extern "C" fn arena_physics_init() -> i32 {
    // No-op boot. Game state falls back to compiled-in defaults that
    // match `arena-assets/dimensions.json`. The orchestrator discovers
    // our exports automatically because §8.5 wires merged exports into
    // Saga.wasmExports.
    1
}

// =============================================================================
// §8.1 per-frame `tick`. The orchestrator calls us once per animation
// frame, after it has updated INPUT_DX + AI_X from peer mods.
// =============================================================================

#[unsafe(no_mangle)]
pub extern "C" fn com_example_arena_physics_tick(dt: f32) {
    // SAFETY: single-threaded guest; the orchestrator gates the order
    // between our `tick(dt)` call and the per-frame work of peer mods.
    unsafe {
        if STATE == 2 { return; }            // paused
        if STATE == 0 { return; }            // waiting for serve (renderer's SPACE key)

        let bw = 800.0_f32;
        let phw = 50.0_f32;
        let r  = BALL_R;

        // Ball motion.
        BALL_X += BALL_VX * dt;
        BALL_Y += BALL_VY * dt;
        BALL_VX *= 0.9995;
        BALL_VY *= 0.9995;

        // Left/right walls.
        if BALL_X < r       { BALL_X = r;       BALL_VX = -BALL_VX; }
        if BALL_X > bw - r  { BALL_X = bw - r;  BALL_VX = -BALL_VX; }

        // Player paddle (bottom).
        let new_px = clamp(
            PLAYER_X + INPUT_DX * 520.0 * dt,
            phw + 4.0,
            bw - phw - 4.0,
        );
        PLAYER_X = new_px;

        // Player-vs-ball collision.
        if BALL_VY > 0.0
            && BALL_Y + r >= 478.0
            && (BALL_X - PLAYER_X).abs() <= phw + r
        {
            BALL_Y = 478.0 - r;
            let hit_centre = (BALL_X - PLAYER_X) / phw;
            BALL_VY = -BALL_VY.abs();
            BALL_VX += hit_centre * 80.0;
            PLAYER_SCORE = PLAYER_SCORE.wrapping_add(1);
            RALLY_HITS   = RALLY_HITS.wrapping_add(1);
        }

        // AI-vs-ball collision. AI_X comes from the orchestrator's
        // last poll of arena-ai.
        if BALL_VY < 0.0
            && BALL_Y - r <= 22.0 + 10.0
            && (BALL_X - AI_X).abs() <= phw + r
        {
            BALL_Y = 22.0 + 10.0 + r;
            let hit_centre = (BALL_X - AI_X) / phw;
            BALL_VY = BALL_VY.abs();
            BALL_VX += hit_centre * 60.0;
            RALLY_HITS = RALLY_HITS.wrapping_add(1);
        }

        // Lose condition: ball passed the player paddle down.
        if BALL_Y > 500.0 + r {
            AI_SCORE = AI_SCORE.wrapping_add(1);
            RALLY_HITS = 0;
            BALL_X = 400.0; BALL_Y = 250.0;
            BALL_VX = 0.0;  BALL_VY = 0.0;
            STATE = 0;
        }
    }
}

#[inline]
fn clamp(x: f32, lo: f32, hi: f32) -> f32 {
    if x < lo { lo } else if x > hi { hi } else { x }
}

// =============================================================================
// Cross-mod getters (reads). Long-prefixed per §8.5 so they merge-clean
// with peer mods' exports.
// =============================================================================

macro_rules! getter_f32 { ($name:ident, $body:expr) => {
    #[unsafe(no_mangle)] pub extern "C" fn $name() -> f32 { unsafe { $body } }
}; }
macro_rules! getter_u32 { ($name:ident, $body:expr) => {
    #[unsafe(no_mangle)] pub extern "C" fn $name() -> u32 { unsafe { $body } }
}; }

getter_f32!(com_example_arena_physics_get_ball_x,          BALL_X);
getter_f32!(com_example_arena_physics_get_ball_y,          BALL_Y);
getter_f32!(com_example_arena_physics_get_ball_vx,         BALL_VX);
getter_f32!(com_example_arena_physics_get_ball_vy,         BALL_VY);
getter_f32!(com_example_arena_physics_get_ball_r,          BALL_R);

getter_f32!(com_example_arena_physics_get_player_paddle_x, PLAYER_X);
getter_u32!(com_example_arena_physics_get_player_score,    PLAYER_SCORE);

getter_f32!(com_example_arena_physics_get_ai_paddle_x,     AI_X);
getter_u32!(com_example_arena_physics_get_ai_score,        AI_SCORE);

getter_u32!(com_example_arena_physics_get_state,           STATE);
getter_u32!(com_example_arena_physics_get_rally,           RALLY_HITS);

// =============================================================================
// Cross-mod setters (writes). The orchestrator (arena-renderer) uses
// these to pipe upstream mods' outputs into our state before each tick.
// =============================================================================

#[unsafe(no_mangle)]
pub extern "C" fn com_example_arena_physics_set_input_dx(v: f32) {
    unsafe { INPUT_DX = clamp(v, -1.0, 1.0); }
}

#[unsafe(no_mangle)]
pub extern "C" fn com_example_arena_physics_set_ai_x(v: f32) {
    unsafe { AI_X = v; }
}

#[unsafe(no_mangle)]
pub extern "C" fn com_example_arena_physics_set_state(v: u32) {
    unsafe { STATE = v; }
}

/// §8.1 / orchestrator hook: kick off a serve from rest.
/// The renderer calls this on the SPACE key (after `set_state(1)`).
/// `vx`/`vy` are in pixels-per-second.
#[unsafe(no_mangle)]
pub extern "C" fn com_example_arena_physics_serve(vx: f32, vy: f32) {
    unsafe {
        BALL_X = 400.0;
        BALL_Y = 250.0;
        BALL_VX = vx;
        BALL_VY = vy;
        STATE = 1;
    }
}

// =============================================================================
// Static panic handler — required for any `no_std` wasm32 crate.
// =============================================================================

#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    loop {}
}
