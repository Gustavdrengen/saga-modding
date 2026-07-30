# `com.example.arena-physics`

Rust → WebAssembly mod. Owns the ball motion, the player paddle
(keyboard-driven by the orchestrator), the score, and the rally-hit
counter. Exports a small set of read-only getters and a few setters
so peer mods can read state through the merged WASM exports table
rather than via raw linear-memory offsets.

| Export name                                              | Owner    | Read by                                |
| -------------------------------------------------------- | -------- | -------------------------------------- |
| `com_example_arena_physics_register`                     | physics  | Saga launcher once at boot             |
| `com_example_arena_physics_tick(dt)`                     | physics  | orchestrator (`saga_start`) per frame  |
| `com_example_arena_physics_get_ball_x/y/vx/vy/r`         | physics  | arena-ai tick args, renderer paint     |
| `com_example_arena_physics_get_player_paddle_x/score`    | physics  | arena-ai, arena-renderer               |
| `com_example_arena_physics_get_ai_paddle_x/score`        | physics  | arena-renderer                         |
| `com_example_arena_physics_get_state/rally`              | physics  | arena-ai, arena-renderer               |
| `com_example_arena_physics_set_input_dx/ai_x/state/serve`| physics  | orchestrator from keyboard + AI poll   |

The registration entrypoint is non-blocking: it emits a `saga:log`
diagnostic line and returns `0` so the launcher can move on to the
next mod. Per-frame work happens in `tick(dt)`, called by the
orchestrator's `saga_start` loop; the `dt` argument is computed
by the orchestrator from its own local clock, since the runtime
does not own ticking.

The loader allocates **16 MiB** of linear memory for this mod via
`manifest.toml: max_memory` (`"16 MiB"` — see `MOD_SPEC.md` §3.1 and
§3.2). The mod's actual state (ball + two paddles + scores + rally
counter) is trivially small; the budget leaves comfortable headroom
for the linear-memory allocator.

The mod has no dependencies, so the launcher schedules it earliest.
