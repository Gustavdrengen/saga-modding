# Saga Example · Mod Collection

> A small set of Saga mods that demonstrate Rust, C, JavaScript, and asset
> packaging. This directory produces mod packages only; it is not a Saga
> launcher and never links, optimizes, instantiates, or executes WebAssembly.

```
example/
├── README.md
├── build.sh            ← producer-side package builder
└── mods/
    ├── arena-assets/   ← assets
    ├── arena-physics/  ← Rust → module.a
    ├── arena-ai/       ← C → module.a
    └── arena-renderer/ ← JavaScript
```

Every WebAssembly mod ships exactly one `module.a` archive. The archive
contains relocatable WebAssembly objects and is not executable. The Saga
launcher later links all active `module.a` archives with `wasm-ld`, resolves
peer imports, runs the required language-neutral optimization passes, and
instantiates the resulting final module.

## Composition

`arena-renderer` is the JavaScript base game. Its registration function
receives the final merged WebAssembly exports object and its `saga_start`
function owns the browser frame loop.

The C AI declares the physics functions as ordinary C extern declarations.
The C producer build leaves these functions unresolved inside `arena-ai/module.a`.
The Saga launcher resolves them against the Rust definitions in
`arena-physics/module.a`. The final linked WebAssembly contains direct peer
calls; no JavaScript call is used for the C-to-Rust boundary.

The renderer uses the final exports object for its JavaScript-to-Wasm calls:

```js
const bx = wasmExports.com_example_arena_physics_get_ball_x();
wasmExports.com_example_arena_ai_tick(bx, by, bvx, bvy, dt);
```

The C and Rust modules use ordinary language runtimes. Neither module
implements or imports a Saga-specific allocator. The linker applies normal
Wasm archive/symbol semantics; post-link optimization removes exact duplicate
Wasm functions and unreachable code without merging mutable runtime state.

## Building packages

```bash
./build.sh
```

`build.sh` only performs producer-side work:

- Rust `staticlib` output is copied to `module.a`.
- C sources are compiled to relocatable Wasm objects and wrapped as `module.a`.
- JavaScript is copied and syntax-checked.
- Asset directories are copied.

It does **not** run `wasm-ld`, `wasm-opt`, a Saga launcher, or any WebAssembly
runtime. The launcher is responsible for consuming the package tree.

Toolchain requirements:

| Tool | Needed by |
| --- | --- |
| `rustup target add wasm32-unknown-unknown` plus Cargo | `arena-physics` |
| Clang with the `wasm32-unknown-unknown` target | `arena-ai` |
| `ar` | All WebAssembly mods, to package C objects as `module.a` |
| `wasm-objdump` | Archive validation in `build.sh` |
| Node.js | JavaScript syntax checking |

## Output tree

```text
example/
├── build/                         ignored producer scratch directory
└── dist/                          ignored shippable mod tree
    ├── arena-assets/
    │   ├── manifest.toml
    │   ├── README.md
    │   └── assets/
    ├── arena-physics/
    │   ├── manifest.toml
    │   ├── README.md
    │   └── module.a
    ├── arena-ai/
    │   ├── manifest.toml
    │   ├── README.md
    │   └── module.a
    └── arena-renderer/
        ├── manifest.toml
        ├── README.md
        └── module.js
```

`dist/` is the package tree consumed by a Saga launcher. No final merged
No final `module.wasm` is produced by this repository.

## Per-mod README pointers

- `arena-physics/` — Rust `std` physics archive and public C ABI.
- `arena-ai/` — C relocatable archive with direct peer imports.
- `arena-renderer/` — JavaScript registration and frame-loop orchestrator.
- `arena-assets/` — pure-data asset package.
