# `com.example.arena-physics`

Rust → WebAssembly mod. It owns ball motion, the player paddle, scores, and
the rally counter. The producer build emits one `module.a` archive containing
relocatable WebAssembly objects and Rust runtime members.

`module.a` is a link input, not an executable module. The Saga launcher links
it with the other active mod archives. The launcher then exposes the listed
functions from `manifest.toml` in the final WebAssembly module.

| Export name                                              | Used by                              |
| -------------------------------------------------------- | ------------------------------------ |
| `com_example_arena_physics_register`                     | launcher registration                |
| `com_example_arena_physics_tick(dt)`                     | JavaScript frame orchestrator        |
| `com_example_arena_physics_get_ball_x/y/vx/vy/r`         | arena-ai and renderer                |
| `com_example_arena_physics_get_player_paddle_x/score`    | renderer                             |
| `com_example_arena_physics_get_ai_paddle_x/score`        | renderer                             |
| `com_example_arena_physics_get_state/rally`              | renderer                             |
| `com_example_arena_physics_set_input_dx/ai_x/state/serve`| renderer and arena-ai                |

The module uses ordinary Rust `std`. The module author does not write a
Saga-specific allocator. Rust's normal runtime and allocator remain in the
archive; Saga's language-neutral linker and optimizer operate on the Wasm
objects without source-language-specific treatment.

The module imports `saga-stdlib` from the repository's Git remote. Cargo
finds it under `rust_bindings/` and records the resolved commit in
`Cargo.lock`.
