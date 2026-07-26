# Saga Example · Mod Collection

> A small, self-contained set of Saga mods that **compose into a Pong-style
> game running on an HTML canvas** when loaded by a real Saga Launcher.
> No engine, no runtime harness lives here — just the mods themselves.

This directory is *not* a runnable demo. The
[engine that loads and runs mods](../MOD_SPEC.md) lives elsewhere. Here
we focus on what a mod *looks* like — and how four mods in different
languages wire into a game together.

```
example/
├── README.md           ← this file
├── build.sh            ← compiles every mod under ./mods/ once per run
└── mods/
    ├── arena-assets/   ← JS + assets/   — palette.json + dimensions.json
    ├── arena-physics/  ← Rust → wasm    — ball + player paddle + getters
    ├── arena-ai/       ← C    → wasm    — AI paddle + saga:thread worker
    └── arena-renderer/ ← JS            — canvas paint + keyboard + RAF loop
```

Each mod follows the file contract in `MOD_SPEC.md` §3 and the per-frame
`tick(dt)` convention documented in §8.1 / §8.5.



## Composition: how the four mods become a game

Per **`MOD_SPEC.md` §8.5`, one mod owns the `requestAnimationFrame` loop
and orchestrates the data flow between the others. In this example that's
`arena-renderer`. Each animation frame it does, in order:

1. **Reads** live ball state from `arena-physics` via the merged
   `Saga.wasmExports`:
   ```js
   const bx  = Saga.wasmExports.com_example_arena_physics_get_ball_x();
   const by  = Saga.wasmExports.com_example_arena_physics_get_ball_y();
   const bvx = Saga.wasmExports.com_example_arena_physics_get_ball_vx();
   const bvy = Saga.wasmExports.com_example_arena_physics_get_ball_vy();
   ```
2. **Calls** the C AI mod with those values as plain function arguments:
   ```js
   Saga.wasmExports.com_example_arena_ai_tick(bx, by, bvx, bvy, dt);
   ```
3. **Pipes** the AI's predicted paddle position back into physics:
   ```js
   const ai_x = Saga.wasmExports.com_example_arena_ai_get_ai_x();
   Saga.wasmExports.com_example_arena_physics_set_ai_x(ai_x);
   ```
4. **Advances** physics, which moves the ball and resolves paddle collisions:
   ```js
   Saga.wasmExports.com_example_arena_physics_tick(dt);
   ```
5. **Paints** by re-reading everything and drawing to the canvas.

The C mod's `tick` takes ball state **as arguments** — it never
`extern`'s into peer mods. That's the §8.5 *no static linking* rule, and
it's what makes `clang` standalone build work cleanly per mod. See
`example/mods/arena-ai/src/main.c` for details.

The renderer is the only mod that owns the frame loop. No double-ticks —
the other mods expose `tick(dt)` but the orchestrator calls them once each.

### Cross-mods in detail

| From / To       | What flows                                          | Surface                                     |
| --------------- | --------------------------------------------------- | ------------------------------------------- |
| assets → renderer | palette colours                                   | `Saga.assets.fetchBuffer("saga://...")`     |
| keyboard input → physics | ±1 input axis                   | `set_input_dx(f32)`                         |
| physics → ai    | ball position + velocity                            | args of `com_example_arena_ai_tick(...)`    |
| ai → physics    | smoothed AI paddle x                                | `get_ai_x()` then `set_ai_x(f32)`            |
| physics → renderer | ball + paddles + scores                          | reads via `com_example_arena_physics_get_*` |

### Why a JS orchestrator instead of `extern` cross-mod?

Inside one merged WASM module (per §5), `extern` resolution across
mod boundaries CAN work — but each mod is built in **isolation** by
`build.sh`, and one `module.wasm` doesn't reference its peers at
static-link time. The §8.5 pattern is the cheapest way to demonstrate
multi-language Saga composition without bringing in a heavier tool
(Binaryen / wasm-merge) just to build mods.

The merged `Saga.wasmExports` table that the engine emits post-merge
is exactly the same dispatcher view that `arena-renderer` already
uses, so the cross-mod surface is **identical**: one JS object, one
set of prefixed keys, one set of function calls.



## Building

```bash
./build.sh
```

The script walks `example/mods/*`, inspects each mod, and produces a
`module.wasm` — or copies `module.js` / `assets/` for non-compiled
mods — next to its `manifest.toml`. Anything that fails to build is
reported and the rest still produce their artifacts.

Toolchain requirements:

| Tool                                                            | Needed by          |
| --------------------------------------------------------------- | ------------------ |
| `rustup target add wasm32-unknown-unknown` + `cargo`/`rustc`    | `arena-physics`    |
| `clang` with `wasm32-unknown-unknown` target                    | `arena-ai`         |
| (none)                                                          | `arena-assets`, `arena-renderer` |

JS and asset mods don't need a toolchain — they can't even fail.

### Where the output lands

`build.sh` keeps the source tree under `./mods/` clean and emits a
**shippable runtime tree** at `./dist/`:

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

Each `dist/<name>/` matches the §3 directory contract (manifest + code
module + assets), so a launcher can drop the directory in wholesale.
`README.md` rides along for the publisher's documentation. The source
tree under `mods/` never carries `module.wasm`; the `dist/` and `build/`
trees are both gitignored.



## Per-mod README pointers

- `arena-assets/` — data bundle (palette + dimensions).
- `arena-physics/` — Rust ball/paddle + long-prefixed cross-mod exports.
- `arena-ai/` — C AI; takes ball state as `tick` args, exports paddle via `get_ai_x`.
- `arena-renderer/` — JS orchestrator: RAF, keyboard, paint.

For the §8.1 `tick` convention, the §8.2 `Saga.*` page-side surface,
the §8.3 cross-module reads via merged exports, the §8.4 asset-timing
hedge, and the §8.5 *no static linking* rule, see the top-level
[`MOD_SPEC.md`](../MOD_SPEC.md).
