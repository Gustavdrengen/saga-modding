# Saga Example · Mod Collection

> A small, self-contained set of Saga mods that compose into a
> Pong-style HTML-canvas game when loaded by a real Saga Launcher.
> No engine, no runtime harness lives here — just the mods themselves.

This directory is not a runnable demo. The Saga launcher that loads
and runs the mods lives elsewhere; here we focus on what a mod looks
like and how four mods in different languages wire into a game
together.

```
example/
├── README.md           ← this file
├── build.sh            ← compiles every mod under ./mods/ once per run
└── mods/
    ├── arena-assets/   ← JS + assets/   — palette.json + dimensions.json
    ├── arena-physics/  ← Rust → wasm    — ball + paddles + getters/setters
    ├── arena-ai/       ← C    → wasm    — AI paddle + worker
    └── arena-renderer/ ← JS            — Phase 1 register + Phase 2 saga_start
```

Each mod follows the file contract in `MOD_SPEC.md` and the
two-phase boot model: a uniquely-named Phase 1 registration entrypoint
that must return `0` immediately, plus the `saga_start` entrypoint
that the launcher invokes once on whichever mod is acting as the
base game for the session.

## Composition: how the four mods become a game

For this example, `arena-renderer` is the base game: its manifest
points `entrypoint` at its registration symbol, and it additionally
exports `saga_start()`. Phase 1 of the launch wires it up; Phase 2
hands it the frame loop.

Each animation frame `saga_start` does, in order:

1. **Reads** the live ball state from `arena-physics` via the
   merged WASM exports object the engine passed into Phase 1:
   ```js
   const bx  = g_pExports.com_example_arena_physics_get_ball_x();
   const by  = g_pExports.com_example_arena_physics_get_ball_y();
   const bvx = g_pExports.com_example_arena_physics_get_ball_vx();
   const bvy = g_pExports.com_example_arena_physics_get_ball_vy();
   ```
2. **Calls** the C AI with those values as plain arguments:
   ```js
   g_aiExports.com_example_arena_ai_tick(bx, by, bvx, bvy, dt);
   ```
3. **Pipes** the AI's predicted paddle position back into physics:
   ```js
   const ai_x = g_aiExports.com_example_arena_ai_get_ai_x();
   g_pExports.com_example_arena_physics_set_ai_x(ai_x);
   ```
4. **Advances** physics, which moves the ball and resolves the
   paddle collisions:
   ```js
   g_pExports.com_example_arena_physics_tick(dt);
   ```
5. **Paints** by re-reading everything and drawing to the canvas.

The C mod receives the ball state as arguments and never
`extern`'s into peer mods — that keeps the per-mod `clang` build
self-contained. See `example/mods/arena-ai/src/main.c` for details.

The renderer is the only mod that owns the frame loop. No
double-ticks: the other mods expose `tick(dt)` functions and the
orchestrator calls them once per frame.

### Cross-mods in detail

| From / To             | What flows                          | Surface                                                  |
| --------------------- | ----------------------------------- | -------------------------------------------------------- |
| assets → renderer     | palette colours                     | `getAsset("palette.json")` JS API / `fetch("saga://...")` |
| keyboard → physics    | ±1 input axis                       | `com_example_arena_physics_set_input_dx(f32)`             |
| physics → ai          | ball position + velocity            | args of `com_example_arena_ai_tick(...)`                 |
| ai → physics          | smoothed AI paddle x                | `get_ai_x()` then `set_ai_x(f32)`                        |
| physics → renderer    | ball + paddles + scores             | reads via `com_example_arena_physics_get_*`              |
| saga:log ↔ all        | diagnostic lines                    | `saga_log(level, msg)` (WASM), `console.log` (JS)        |
| saga:time ↔ physics   | clock probes during registration    | `delta()` / `elapsed()` / `ticks()`                      |

### Why a JS orchestrator instead of `extern` cross-mod?

Inside one merged WASM module, `extern` resolution across mod
boundaries can work, but each mod in this example is compiled in
isolation by `build.sh`. A standalone `clang` / `rustc` invocation
can't resolve peer-mod symbols at static-link time, so the
orchestrator pattern (the JS renderer reading merged exports and
handing values around as plain arguments) is the cheapest way to
demonstrate multi-language Saga composition without bringing in a
heavier tool (Binaryen / wasm-merge) just to build mods.

## Building

```bash
./build.sh
```

The script walks `example/mods/*`, inspects each mod, and produces
a `module.wasm` — or copies `module.js` / `assets/` for
non-compiled mods — next to its `manifest.toml`. Anything that
fails to build is reported and the rest still produce their
artifacts.

Toolchain requirements:

| Tool                                                         | Needed by          |
| ------------------------------------------------------------ | ------------------ |
| `rustup target add wasm32-unknown-unknown` + `cargo`/`rustc` | `arena-physics`    |
| `clang` with `wasm32-unknown-unknown` target                 | `arena-ai`         |
| `node` (optional, for `node --check`)                         | `arena-assets`, `arena-renderer` |
| (none)                                                       | the launcher       |

JS and asset mods don't need a toolchain — they can't even fail.

### Where the output lands

`build.sh` keeps the source tree under `./mods/` clean and emits a
shippable runtime tree at `./dist/`:

```
example/
├── README.md
├── build.sh
├── .gitignore              ← ignores dist/ + build/
├── mods/                   ← SOURCE (manifest + src + README + module.js)
│   ├── arena-assets/
│   ├── arena-physics/
│   ├── arena-ai/
│   └── arena-renderer/
├── build/                  ← cargo target-dir scratch (gitignored)
│   └── arena-physics/target/...
└── dist/                   ← SHIPPABLE runtime tree (gitignored)
    ├── arena-assets/
    │   ├── manifest.toml
    │   ├── README.md
    │   ├── module.js
    │   └── assets/
    ├── arena-physics/
    │   ├── manifest.toml
    │   ├── README.md
    │   └── module.wasm
    ├── arena-ai/
    │   ├── manifest.toml
    │   ├── README.md
    │   └── module.wasm
    └── arena-renderer/
        ├── manifest.toml
        ├── README.md
        └── module.js
```

Each `dist/<name>/` matches the file contract for a Saga mod, so a
launcher can drop the directory in wholesale. `README.md` rides
along for the publisher's documentation. The source tree under
`mods/` never carries `module.wasm`; `dist/` and `build/` are both
gitignored.

## Per-mod README pointers

- `arena-assets/` — data bundle (palette + dimensions).
- `arena-physics/` — Rust ball/paddle + long-prefixed cross-mod exports.
- `arena-ai/` — C AI; takes ball state as `tick` args, exports paddle via `get_ai_x`.
- `arena-renderer/` — JS Phase 1 register + Phase 2 `saga_start`.
