# `com.example.arena-ai`

C → WebAssembly mod. It drives the AI opponent paddle and emits one
`module.a` archive containing relocatable WebAssembly objects.

The source declares these physics functions as ordinary C ABI imports:

```c
extern float com_example_arena_physics_get_ball_x(void);
extern void com_example_arena_physics_set_ai_x(float value);
```

The producer build leaves those peer symbols unresolved in `module.a`. The
Saga launcher resolves them against `arena-physics/module.a` during the final
`wasm-ld` link. The resulting calls are direct WebAssembly calls and do not
cross JavaScript.

The module also imports:

- `saga:thread` for the worker and cooperative yield;
- `saga:log` for registration diagnostics.

| Export                                | Used by                          |
| ------------------------------------- | -------------------------------- |
| `com_example_arena_ai_register`       | launcher registration            |
| `com_example_arena_ai_tick`           | JavaScript orchestrator          |
| `com_example_arena_ai_direct_sample` | direct-call validation/demo      |
| `com_example_arena_ai_get_ai_x`       | JavaScript orchestrator          |
| `com_example_arena_ai_reset_ai`       | JavaScript orchestrator          |
| `com_example_arena_ai_worker`         | `saga:thread` dispatcher         |

The producer build never runs `wasm-ld`, `wasm-opt`, a Saga launcher, or a
WebAssembly runtime. It only creates the package artifact required by the
specification.
