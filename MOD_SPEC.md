# Saga Platform Mod Specification

**Platform Name:** Saga / Saga Launcher

**Document Status:** Standard Draft

---

## 1. Overview & Architecture

The **Saga Platform** uses a modular, multi-target execution model. A **Mod** (short for both _Module_ and _Modification_) is the fundamental unit of code, assets, and content extension within Saga.

Mods are designed to run in a web-native environment (WASM + JavaScript). Rather than treating JavaScript purely as glue code for WebAssembly, Saga treats **JavaScript modules (`module.js`) and WebAssembly modules (`module.wasm`) as peer execution units**.

At runtime, the Saga Engine dynamically merges all active `.wasm` files using Binaryen into a single, optimized WebAssembly instance while aggregating all corresponding `.js` code modules into a single execution environment sharing a unified memory space and symbol table.

---

## 2. Mod Identification & Namespace Conventions

### 2.1 Mod Identifier (ID)

A Mod ID uniquely identifies a mod across the entire Saga ecosystem.

- **Format:** Reverse Domain Notation (RDN).
- **Syntax:** `[domain-reverse].[project].[modname]` (arbitrary dot-separated segments allowed).
- **Rules:** Lowercase alphanumeric characters and hyphens only (`a-z`, `0-9`, `-`).
- **Examples:**
- `com.company.project.core-physics`
- `io.github.developer.lighting-pack`
- `net.saga.official.base-game`

> **Note:** The Mod ID and the Mod's published version are **not** present in the local `manifest.toml`. Mod IDs and release versions are publishing/distribution concerns managed by the Saga Launcher registry.

---

## 3. Directory Layout & File Contracts

A valid Saga mod directory must conform to the following file layout:

```text
my-saga-mod/
├── manifest.toml         [REQUIRED] Mod metadata & dependency declaration
├── module.wasm           [OPTIONAL] Compiled WebAssembly binary
├── module.js             [OPTIONAL] Companion JavaScript module
├── README.md             [OPTIONAL] Human-readable documentation
├── src/                  [OPTIONAL] Source code (Rust, C, etc.) – built to module.wasm
└── assets/               [OPTIONAL] Arbitrary assets (textures, audio, data)
    ├── textures/
    └── audio/

```

### 3.1 `manifest.toml` (Required)

The manifest specifies human-readable metadata and dependency declarations required by the Saga loader to resolve execution order.

```toml
# Display details for Saga Launcher / In-game UI
name = "My Custom Rendering Mod"
description = "Adds real-time particle rendering and custom shader pipelines."

# Optional: name of the export (WASM symbol) or named JS export that the
# launcher calls once on boot. See §6.
entrypoint = "my_mod_init"

# Dependencies mapping required Mod IDs to semantic-version constraints
[dependencies]
"com.saga.official.core" = "^1.0.0"
"org.community.math-utils" = ">=2.1.0"

# Optional: build hints consumed by external build tools (see §3.1.1).
[build]
type     = "rust"           # "rust" | "c" | "js" | "assets"
output   = "module.wasm"    # canonical output filename, default depends on `type`
command  = "cargo build --target wasm32-unknown-unknown --release"

```

#### Fields Schema

| Field          | Type     | Required | Description                                                     |
| -------------- | -------- | -------- | --------------------------------------------------------------- |
| `name`         | `string` | **Yes**  | Human-readable display name.                                    |
| `description`  | `string` | **Yes**  | Detailed description of the mod's function.                     |
| `dependencies` | `table`  | No       | Map of Mod ID keys (`string`) to SemVer rule values (`string`). |
| `entrypoint`   | `string` | No       | Bare symbol name of a function to call on boot (see §6).        |
| `build`        | `table`  | No       | Build-tool hints (see §3.1.1).                                  |

#### 3.1.1 `[build]` table

The `[build]` table is purely informational for the Saga Launcher; the
launcher itself does not compile mods. External build tools (such as the
`build.sh` shipped with the repo's `example/` directory) read these
fields to decide how to turn `src/` into `module.wasm`/`module.js`.

| Sub-field  | Type     | Required | Description                                                                 |
| ---------- | -------- | -------- | --------------------------------------------------------------------------- |
| `type`     | `string` | No       | Discriminator: `"rust"`, `"c"`, `"js"`, or `"assets"`. Defaults by detection. |
| `output`   | `string` | No       | Filename of the produced artifact. Defaults to `module.wasm` (Rust/C) or `module.js` (JS). |
| `command`  | `string` | No       | Free-form hint of the command used to build. Documentation only.            |

### 3.2 Code Modules (`module.wasm` and `module.js`)

Both code files are optional individually, but a functional mod will typically contain at least one.

#### `module.wasm` Specification

- **Target:** `wasm32-unknown-unknown` (or similar bare-metal WASM target).
- **Imports:** Must import shared linear memory (`env.memory`) and indirect function table (`env.__indirect_function_table`).
- **Host imports:** Must import the standard namespaces `saga:assets` and `saga:thread` (see §4) for any asset / threading functionality. Imports use the `extern "C"` ABI and each function name is significant inside the namespace.
- **Export Naming:** Symbols exported by WASM should be uniquely prefixed using the C ABI (e.g., `extern "C" fn com_company_mod_init()`) to prevent Binaryen symbol collision during runtime merging.

#### `module.js` Specification

`module.js` must be a valid ES Module exporting two primary constructs:

1. `imports`: An object containing functions exposed to the unified WASM environment.
2. An entrypoint — see §6 for the canonical declaration and the precedence rules. New mods should declare the entrypoint in `manifest.toml` (`entrypoint = "..."`); for backwards compatibility, an `init(wasmExports, memory, table)` default export on `module.js` will be invoked *only* if the manifest does not declare an entrypoint.

```javascript
// Example module.js
export const imports = {
  // Saga standard host function extension or peer API
  com_company_mod_custom_log: (ptr, len) => {
    // Read string from shared memory
  },
};

// Either an `init(wasmExports, memory, table)` fallback (when the
// manifest does not declare an `entrypoint`):
export function init(exports, memory, table) {
  // Save global WASM export references
  console.log("Mod com.company.mod initialized!");
}
```

> **Rust authors:** the safe `fetch_buffer`, `spawn_thread`, etc. wrappers live in the
> `saga-stdlib` crate shipped at `rust_bindings/` in this repository. Always
> prefer those over raw `extern "C"` blocks — they handle sentinel-checking,
> arity, and ownership for you. See `example/mods/hello-rust/src/lib.rs` for a
> complete worked example.

#### 3.3 `assets/` Directory

Contains arbitrary files (models, textures, audio files, JSON data). The directory structure inside `assets/` is left entirely to the discretion of the mod author.

Assets are accessed across mods using the Saga Asset Protocol (URI syntax):
`saga://<mod-id>/<path-to-asset>`

- **Example (Internal):** `saga://self/textures/grass.png` (resolves to the current mod's assets).
- **Example (Cross-Mod):** `saga://com.saga.official.core/audio/click.wav`.

A mod that ships only data may have nothing but `assets/` (plus a
`manifest.toml`). To make those bytes visible to the runtime, the mod
must either include a tiny `module.js` that registers the bytes with the
host on boot, or rely on the Saga Launcher's auto-discovery of the
`assets/` directory.

---

## 4. Pre-defined System Imports (Saga Standard Library)

Saga provides host-level system bindings under standardized module import namespaces. All WASM and JS modules can import these natively.

### 4.1 Asset Management (`saga:assets`)

Provides low-level and high-level mechanisms to load assets asynchronously from linear memory or JS.

#### WASM C-ABI Interface

```rust
extern "C" {
    /// Requests an asset buffer from Saga Launcher storage.
    ///
    /// - uri_ptr / uri_len: Pointer and length of the 'saga://...' string.
    /// - handle_out: Returns an integer asset handle (> 0 if success, <= 0 on error).
    fn saga_asset_open(uri_ptr: *const u8, uri_len: usize) -> i32;

    /// Queries the byte length of an opened asset handle.
    fn saga_asset_get_size(handle: i32) -> usize;

    /// Reads asset data into a WASM linear memory pointer.
    fn saga_asset_read(handle: i32, dest_ptr: *mut u8, length: usize) -> i32;

    /// Closes an asset handle freeing its host resources.
    fn saga_asset_close(handle: i32);
}

```

#### JS Interface

```javascript
import { Saga } from "saga:engine";

// Asynchronously fetch raw ArrayBuffer or Blob from any mod
const audioData = await Saga.assets.fetchBuffer(
  "saga://org.community.audio/effects/boom.mp3",
);
```

### 4.2 Multithreading & Worker Spawning (`saga:thread`)

Saga supports multithreading via Web Workers sharing `SharedArrayBuffer` memory and the WebAssembly Table.

#### WASM C-ABI Interface

```rust
extern "C" {
    /// Spawns a Web Worker to execute a WASM function via function pointer index.
    ///
    /// - entry_idx: Table index of the function pointer to run on the worker.
    /// - arg_ptr: Pointer to memory containing arguments for the worker task.
    /// - returns: Thread ID (>0) or error code (<0).
    fn saga_thread_spawn(entry_idx: usize, arg_ptr: usize) -> i32;

    /// Yields execution on the current thread.
    fn saga_thread_yield();
}

```

#### Thread Execution Flow

1. WASM calls `saga_thread_spawn(entry_idx, arg_ptr)`.
2. Saga Launcher Runtime intercepts the call, spawns a `Worker`, and sends the compiled `WebAssembly.Module`, `SharedArrayBuffer` memory, and `WebAssembly.Table`.
3. The Worker calls `wasm_worker_entry(entry_idx, arg_ptr)` on the WASM instance.

---

## 5. Runtime Resolution & Loading Pipeline

When Saga Launcher launches an instance containing multiple mods, it executes the following load pipeline:

```text
       [ Manifest Resolution & Topological Dependency Sort ]
                                 │
                                 ▼
       ┌─────────────────────────┴─────────────────────────┐
       │                                                   │
       ▼                                                   ▼
[ Binaryen Assembly Pass ]                     [ JS Module Pass ]
Read all `module.wasm` files                   Import all `module.js` files
Merge AST into unified WASM Module             Aggregate `imports` objects
                                 │                         │
                                 └────────────┬────────────┘
                                              │
                                              ▼
                                 [ Single WASM Instance ]
                                 Instantiate via WebAssembly
                                              │
                                              ▼
                                 [ Post-Init Wiring ]
                                 Invoke entrypoints in dep order (see §6)

```

1. **Manifest Validation:** Reads all `manifest.toml` files, resolves dependency trees, and builds an execution graph.
2. **Binaryen Synthesis:** Reads all `module.wasm` binaries, merges them into a single `WebAssembly.Module` using Binaryen's AST merger, and links shared atomic memory.
3. **JS Aggregation:** Dynamically imports all `module.js` files, merging their `imports` objects into a single global `importObject`.
4. **Unified Instantiation:** Instantiates the merged WASM binary with the unified `importObject`, shared `SharedArrayBuffer`, and shared `WebAssembly.Table`.
5. **Boot / Entrypoint Pass:** Iterates every mod's `manifest.toml`. If the mod declares an `entrypoint` (see §6), the launcher invokes it with the unified `wasmExports`, the unified `memory`, and the unified `indirect function table` (`table`). If a JS-only mod has no `entrypoint` but does have a default `init(wasmExports, memory, table)` export, that is used instead.

---

## 6. Entrypoints

A mod MAY declare an entrypoint in its `manifest.toml`:

```toml
entrypoint = "mod_init"
```

The value is the **bare symbol name** of an export.

| Mod source type | How the entrypoint is declared                                                                                       |
| --------------- | -------------------------------------------------------------------------------------------------------------------- |
| Rust `src/`     | `#[no_mangle] pub extern "C" fn mod_init() -> i32 { … }`                                                            |
| C `src/`        | The function marked `__attribute__((export_name("mod_init")))` and linked with `-Wl,--export=mod_init`              |
| `module.js`     | A named ES export: `export function mod_init() { … }`                                                               |

A mod without an `entrypoint` field MAY still expose an `init(wasmExports, memory, table)`
default export from `module.js`; the launcher will then call that
fallback. The two conventions exist for historical reasons; new mods
should prefer the explicit `entrypoint` declaration in the manifest.

### 6.1 Execution Order

When multiple active mods declare entrypoints, the Saga Launcher calls
them in **dependency-first order**: a mod's entrypoint is called *before*
the entrypoints of any mod it depends on. The phrase "upper levels first,
then their dependencies, and so on" captures this – the entry points of
mods at the top of the dependency graph (those with fewest in-deps) run
first, and the chain proceeds down the dep tree:

```
   app   ───────► runs first
    │
    ├─► lib-a ──► runs after `app`
    │
    └─► lib-b ──► runs after `app`
         │
         └─► lib-c ──► runs after `lib-b`
```

Mods without an `entrypoint` and without an `init()` default export are
**skipped** during the boot pass – they participate in the runtime only
through their exports.

### 6.2 Entrypoint Contract

Each entrypoint is called exactly once on boot with the arguments:

- `wasmExports`: the merged export object of all WASM mods.
- `memory`: an `ArrayBuffer`-shaped handle to the unified linear memory.
- `table`: the unified indirect-call function table.

Entrypoints MAY register asset URIs, hook host imports, schedule async
work, or spawn worker threads. They MUST NOT block indefinitely; the
launcher is expected to return control to the engine promptly.

The entrypoint's return value is interpreted as an `i32` status code. By
convention, `0` indicates "nothing to do", non-zero indicates success.
Errors are expected to be logged through the host's logging surface, not
raised as exceptions.

---

## 7. License

This specification is dual-licensed under MIT or Apache-2.0, at your
option. The `saga-stdlib` reference implementation under
`../rust_bindings/` uses the same license.

---

## 8. Frame Loop, Per-Mod `tick`, and the `Saga` Page-Side Surface

The §3.2 lifecycle hooks (*entrypoint*, `init`) describe **boot-time**
behaviour. Many mods also need to do work **every frame** (game
physics, AI ticks, canvas redraws, input polling). This section
documents the convention the Saga Launcher uses to drive that
work, and the page-side `Saga.*` surface that hosts provide to
source-language modules.

### 8.1 Per-frame `tick` exports

Every mod MAY export a per-frame function:

```toml
# (no manifest entry needed — the convention is the contract)
```

| Mod source type | How the `tick` function is declared                                            |
| --------------- | ----------------------------------------------------------------------------- |
| Rust `src/`     | `#[unsafe(no_mangle)] pub extern "C" fn tick(dt: f32) { ... }`                |
| C `src/`        | `int tick(float dt) { ... }` exported via `-Wl,--export=tick`                 |
| `module.js`     | `export function tick(dt) { ... }`                                             |

The Saga Launcher calls each mod's `tick` **once per animation
frame**, in **dependency order** (the same dep-first order as
§6.1's entrypoint invocation). The argument `dt` is the wall-clock
time elapsed since the previous frame, in seconds (clamped to a
sane maximum by the launcher so a tab-switch doesn't cause
timesteps-of-doom).

A mod that has boot-time work but no per-frame work (e.g. the
asset-bundle mod in `example/`) DOES NOT need to export `tick`.
A mod that only does per-frame work (e.g. a renderer) MAY set
its manifest `entrypoint` field to a no-op function and rely on
its `tick` export for everything else.

### 8.2 The page-side `Saga.*` surface

For pure-JavaScript mods (`module.js` writers), the launcher
publishes a global `Saga` object on `globalThis` BEFORE the
first entrypoint is invoked. The surface has at least the
following namespaces; future versions may add more.

```javascript
globalThis.Saga = {
  host: {
    log(...args): void   // route to the host-side logger / dev console
  },
  runtime: {
    registerTick(fn: (dt: number) => void): void  // §8.1 alternative: explicit per-frame hook
    fireEachFrame(dt: number): void              // the mod that owns the RAF loop calls this once per frame
  },
  canvas: HTMLCanvasElement | null,    // the page canvas, when the launcher is the canvas-bearer
  ctx:    CanvasRenderingContext2D | null,
  hud:    HTMLElement | null,          // <div> overlay, for overlay-style renderers
  memory: { buffer: ArrayBuffer, f32: Float32Array, u32: Uint32Array, u8: Uint8Array },
  table:  { get(idx): Function, add(fn): number, size(): number },
  wasmExports: { [name: string]: Function | WebAssembly.Global },  // merged exports of all WASM mods
  assets: {
    register(uri: string, bytes: Uint8Array): void,  // publish bytes under a saga:// URI
    fetchBuffer(uri: string): Promise<Uint8Array>,
  },
  thread: {
    spawn(entryIdx: number, argPtr: number): number,  // returns thread id
    yield(): void,                                    // saga_thread_yield equivalent
  },
};
```

Mods that wish to drive the frame themselves (rather than rely
on the launcher to call `tick`) can opt out of §8.1 by skipping
the `tick` export and instead doing, in `module.js`:

```javascript
export function init() {
  Saga.runtime.registerTick(myTickFn);   // the launcher collates them
}

function myTickFn(dt) { ... }
```

### 8.3 Cross-module communication

Within a single merged WASM instance (the spec's §5 load
pipeline), modules see each other's exports via standard
extern-function calls. A Rust mod that declares a getter:

```rust
#[unsafe(no_mangle)] pub extern "C" fn get_ball_x() -> f32 { BALL_X }
```

is callable from a C mod that imports it:

```c
extern float get_ball_x(void);   /* resolves against merged exports */
```

and from a JS mod via `Saga.wasmExports.get_ball_x()`. No
linear-memory offset arithmetic is needed by the caller.

> **Note:** the bare-extern / merged-exports pattern above is the
> "fits-in-one-link" form. The standalone per-mod build pipeline
> (`example/build.sh`) described in §8.5 does NOT support it, because
> each mod compiles to its own `module.wasm` and cannot resolve
> peer-mod symbols at static-link time. Engines that merge via a
> monolithic Binaryen pass can take either path; standalone builds
> MUST use the orchestrator pattern (§8.5).

### 8.4 Asset-registration timing

A "data-only" mod (`assets/` directory and a one-line `init`
that calls `Saga.assets.register(...)`) runs its entrypoint in
the same dep-first order as everything else. A mod whose
boot-time work needs to *fetch* an asset declared by such a
mod SHOULD either:

* declare a dependency on the data mod and **lazy-load the
  asset on its first `tick`**, tolerating "asset not registered"
  failure on early frames, OR
* elide the dependency and just hard-code defaults.

This avoids the chicken-and-egg "physics depends on assets, but
physics's entrypoint must run before assets' entrypoint" pattern
that motivated §6.1's dep-first order.

### 8.5 Cross-mod communication and the *no static linking* rule

Mods are compiled in **isolation**: every `module.wasm` under
`example/mods/<name>/module.wasm` is built by `example/build.sh`
without any peer-mod symbols in scope. **Mods MUST NOT introduce
`extern` declarations for another mod's exports** — the resulting
`module.wasm` would not link, and even if it did the static linker
can't resolve cross-`module.wasm` references anyway.

The Saga Engine communicates across mods at **runtime** through
two surfaces:

1. **The merged `Saga.wasmExports` table** (visible from JS).
   After the engine merges every active mod's `.wasm` into a
   single composite WASM module (per §5), every export becomes
   visible at a stable name — namely the mod-id-prefixed form
   `<mod-id-with-dots-as-underscores>_<export-name>`. Example:
   `com.example.arena-physics.get_ball_x` → JS object key
   `com_example_arena_physics_get_ball_x`.
2. **`Saga.runtime.invokeFn(targetModId, fnName, ...args)`** —
   page-side dispatcher for wasm-to-wasm calls. The engine
   maintains the unified indirect-call function table and this
   dispatcher routes through it.

Because of #1 and #2, **one canonical pattern** is enough for
the example game (and for any Saga deployment): the mod that
owns the `requestAnimationFrame` loop (typically a renderer) is
the **orchestrator**. Each frame it:

1. Reads upstream mod state via `Saga.wasmExports.<prefix>_*()`,
2. Calls downstream mod functions explicitly:
   `Saga.wasmExports.<prefix>_tick(args, dt)`,
3. Pipes the response back into physics via
   `Saga.wasmExports.<prefix>_set_<x>(value)`, then
4. Paints.

The C mod does NOT `extern float com_example_arena_physics_get_ball_x(void);`
— instead its `arena-ai.tick(bx, by, bvx, bvy, dt)` takes the
ball state as arguments, and `arena-renderer` calls it with the
values it just read from physics. This eliminates the static
linker problem at the cost of an extra JS-level function call.

The convention enforced by `example/build.sh` is therefore:

* C mods: pass `-Wl,--allow-undefined` defensively so a stray
  cross-mod `extern` left over from a refactor doesn't surface as
  a toolchain error. Standalone `module.wasm` is still valid —
  the symbol becomes a real WASM import that the engine's
  merger wires up to the right export at instantiation time.
* Rust mods: export each cross-mod boundary as a `#[unsafe(no_mangle)]`
  `pub extern "C" fn`, prefixed with the mod id.
* JS mods: read via `Saga.wasmExports[<long-name>]()`.
