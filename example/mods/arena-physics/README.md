# `com.example.arena-physics`

Rust → WebAssembly mod. Owns the ball motion, the player paddle
(controlled by the keyboard input that `arena-renderer` writes
in `globalThis.Saga.input`), the score, and the rally-hit
counter. Exports a small set of read-only getters that
`arena-ai` (a C mod) imports as `extern "C" { fn get_ball_x() -> f32; … }`
and that `arena-renderer` (JS) reads through
`Saga.wasmExports.get_ball_x()`.

| Export name                                  | Owner    | Read by                                |
| -------------------------------------------- | -------- | -------------------------------------- |
| `tick(dt)`                                   | physics  | engine frame-loop calls each frame     |
| `get_ball_x`, `get_ball_y`, `get_ball_vx`, `get_ball_vy`, `get_ball_r` | physics | arena-ai (C extern), arena-renderer (JS) |
| `get_player_paddle_x`, `get_player_score`    | physics  | arena-ai, arena-renderer              |
| `get_ai_paddle_x`, `get_ai_score`             | physics  | arena-renderer                        |
| `get_state` (0=idle, 1=rally, 2=paused)       | physics  | arena-ai, arena-renderer              |

The entrypoint `arena_physics_init` is a no-op stub (state is
zero-initialised); `tick(dt)` does all the work each frame.

It uses `saga-stdlib` for safe saga:asset fetches.
