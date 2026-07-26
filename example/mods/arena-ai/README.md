# `com.example.arena-ai`

C → WebAssembly mod. Drives the AI opponent (top) paddle with two
complementary strategies:

- A **lightweight interceptor** invoked every frame: the
  orchestrator's `saga_start` loop pushes the latest ball state to
  `com_example_arena_ai_tick(bx, by, bvx, bvy, dt)`, which predicts
  where the ball will meet the AI line and smooth-slides `g_ai_x`
  toward that intercept.
- A **heavyweight worker** registered into the unified function table;
  a real Saga launcher dispatches it via `saga_thread_spawn(worker,
  arg_ptr)`. The worker demonstrates `saga_thread_yield` cooperation.

The mod uses two host-import namespaces:

- `saga:thread` — `saga_thread_spawn` / `saga_thread_yield` for the
  worker dispatch and cooperative preemption.
- `saga:log` — `saga_log(level, msg_ptr, msg_len)` writes a
  human-readable line into the engine log during the registration
  phase so a real launcher's diagnostic panel shows the mod coming
  online.

| Export                                | Owner    | Read by                          |
| ------------------------------------- | -------- | -------------------------------- |
| `com_example_arena_ai_register`       | this mod | Saga launcher once at boot       |
| `com_example_arena_ai_tick`           | this mod | `arena-renderer` (orchestrator)  |
| `com_example_arena_ai_get_ai_x`       | this mod | `arena-renderer` (orchestrator)  |
| `com_example_arena_ai_reset_ai`       | this mod | `arena-renderer` (orchestrator)  |
| `worker`                              | this mod | `saga:thread` dispatcher         |

No peer mod's exports are statically linked into this crate; the
orchestrator reads physics state via merged exports and ships it as
plain arguments to `tick`. That makes the build pipeline
self-contained — a per-mod `clang --target=wasm32-unknown-unknown`
does not need any cross-mod object files at all.
