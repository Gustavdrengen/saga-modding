//! `arena-physics` — Saga mod (Rust → WebAssembly).
//!
//! Ball + player-paddle physics for the example game. The module uses
//! ordinary Rust `std`; Cargo packages the resulting relocatable Wasm objects
//! in `module.a` for the Saga launcher.

use saga_stdlib::{log, LogLevel};

// =============================================================================
// Game state. Owned exclusively by this mod; cross-mod access is always via
// the getter/setter exports below, never via direct linear-memory arithmetic.
// =============================================================================

static mut STATE: u32 = 0; // 0=idle, 1=rally, 2=paused
static mut BALL_X: f32 = 400.0;
static mut BALL_Y: f32 = 250.0;
static mut BALL_VX: f32 = 0.0;
static mut BALL_VY: f32 = 0.0;
static mut BALL_R: f32 = 9.0;

static mut PLAYER_X: f32 = 400.0;
static mut PLAYER_SCORE: u32 = 0;
static mut RALLY_HITS: u32 = 0;

static mut AI_X: f32 = 400.0;
static mut AI_SCORE: u32 = 0;

static mut INPUT_DX: f32 = 0.0;

#[unsafe(no_mangle)]
pub extern "C" fn com_example_arena_physics_register() -> i32 {
    log(LogLevel::Info, "arena-physics registered");
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn com_example_arena_physics_tick(dt: f32) {
    unsafe {
        if STATE == 2 || STATE == 0 {
            return;
        }

        let bw = 800.0_f32;
        let phw = 50.0_f32;
        let r = BALL_R;

        BALL_X += BALL_VX * dt;
        BALL_Y += BALL_VY * dt;
        BALL_VX *= 0.9995;
        BALL_VY *= 0.9995;

        if BALL_X < r {
            BALL_X = r;
            BALL_VX = -BALL_VX;
        }
        if BALL_X > bw - r {
            BALL_X = bw - r;
            BALL_VX = -BALL_VX;
        }

        PLAYER_X = clamp(PLAYER_X + INPUT_DX * 520.0 * dt, phw + 4.0, bw - phw - 4.0);

        if BALL_VY > 0.0 && BALL_Y + r >= 478.0 && (BALL_X - PLAYER_X).abs() <= phw + r {
            BALL_Y = 478.0 - r;
            let hit_centre = (BALL_X - PLAYER_X) / phw;
            BALL_VY = -BALL_VY.abs();
            BALL_VX += hit_centre * 80.0;
            PLAYER_SCORE = PLAYER_SCORE.wrapping_add(1);
            RALLY_HITS = RALLY_HITS.wrapping_add(1);
        }

        if BALL_VY < 0.0 && BALL_Y - r <= 32.0 && (BALL_X - AI_X).abs() <= phw + r {
            BALL_Y = 32.0 + r;
            let hit_centre = (BALL_X - AI_X) / phw;
            BALL_VY = BALL_VY.abs();
            BALL_VX += hit_centre * 60.0;
            RALLY_HITS = RALLY_HITS.wrapping_add(1);
        }

        if BALL_Y > 500.0 + r {
            AI_SCORE = AI_SCORE.wrapping_add(1);
            RALLY_HITS = 0;
            BALL_X = 400.0;
            BALL_Y = 250.0;
            BALL_VX = 0.0;
            BALL_VY = 0.0;
            STATE = 0;
        }
    }
}

#[inline]
fn clamp(x: f32, lo: f32, hi: f32) -> f32 {
    if x < lo { lo } else if x > hi { hi } else { x }
}

macro_rules! getter_f32 {
    ($name:ident, $body:expr) => {
        #[unsafe(no_mangle)]
        pub extern "C" fn $name() -> f32 {
            unsafe { $body }
        }
    };
}
macro_rules! getter_u32 {
    ($name:ident, $body:expr) => {
        #[unsafe(no_mangle)]
        pub extern "C" fn $name() -> u32 {
            unsafe { $body }
        }
    };
}

getter_f32!(com_example_arena_physics_get_ball_x, BALL_X);
getter_f32!(com_example_arena_physics_get_ball_y, BALL_Y);
getter_f32!(com_example_arena_physics_get_ball_vx, BALL_VX);
getter_f32!(com_example_arena_physics_get_ball_vy, BALL_VY);
getter_f32!(com_example_arena_physics_get_ball_r, BALL_R);
getter_f32!(com_example_arena_physics_get_player_paddle_x, PLAYER_X);
getter_u32!(com_example_arena_physics_get_player_score, PLAYER_SCORE);
getter_f32!(com_example_arena_physics_get_ai_paddle_x, AI_X);
getter_u32!(com_example_arena_physics_get_ai_score, AI_SCORE);
getter_u32!(com_example_arena_physics_get_state, STATE);
getter_u32!(com_example_arena_physics_get_rally, RALLY_HITS);

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
