# `com.example.arena-ai`

C → WebAssembly mod. Drives the AI opponent (top) paddle with
two strategies that combine:

- A **lightweight interceptor** that each `tick(dt)` updates
  `set_ai_x(target)` with a linear-projection of where the ball
  will meet the AI's line.
- A **heavyweight lookahead worker** that does a longer
  trajectory scan. The worker is registered into the unified
  function table and dispatched via `saga_thread_spawn(worker,
  arg_ptr)` from the main `tick`. It writes its refined target
  into the same slot, which the engine's next frame reads.

The mod demonstrates the entire saga:host interface:

- §4.1 `saga:assets` — `saga_asset_open / get_size / close` for
  fetching the AI personality JSON the worker consults.
- §4.2 `saga:thread` — `saga_thread_spawn` to fork the lookahead
  worker, and `saga_thread_yield` from inside the worker for
  cooperative preemption.

| Export                            | Owner    | Read by                          |
| --------------------------------- | -------- | -------------------------------- |
| `tick(dt)`                        | this mod | engine frame loop                |
| `worker(arg_ptr)`                 | this mod | saga:thread dispatcher in real Saga |
| `get_ai_x()`                      | this mod | arena-renderer (JS)              |

The mod depends on `arena-physics` for the ball-state getters it
needs. The dep-first entrypoint invocation order means
`arena-physics.phys_init` runs before this mod's
`arena_ai_init` — so by the time `tick` first fires, physics
has already registered its getters into the merged WASM exports.
