# Saga Platform Mod Specification

---

## 1. Overview & Architecture

The **Saga Platform** uses a modular, multi-target execution model. A **Mod** (short for both _Module_ and _Modification_) is the fundamental unit of code, assets, and content extension within Saga.

Mods run in a web-native environment (WASM + JavaScript). Rather than treating JavaScript purely as glue code for WebAssembly, Saga treats **JavaScript modules (`module.js`) and WebAssembly modules (`module.wasm`) as peer execution units**.

At runtime, the Saga Engine dynamically merges all active `.wasm` files into a single, optimized WebAssembly instance while aggregating all corresponding `.js` code modules into a single execution environment sharing a unified linear memory space and symbol table.

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

> **Note:** Mod IDs and release versions are managed by the Saga Launcher registry during distribution and publishing.

---

## 3. Directory Layout & File Contracts

A valid Saga mod directory must conform to the following layout:

```text
my-saga-mod/
├── manifest.toml         [REQUIRED] Mod metadata & dependency declaration
├── module.wasm           [OPTIONAL] Compiled WebAssembly binary
├── module.js             [OPTIONAL] Companion JavaScript module
├── README.md             [OPTIONAL] Human-readable documentation
├── src/                  [OPTIONAL] Source code (Rust, C, etc.)
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

# Optional: unique WASM symbol or named JS export called during Phase 1 (Registration).
entrypoint = "com_company_mod_register"

# Dependencies mapping required Mod IDs to semantic-version constraints
[dependencies]
"com.saga.official.core" = "^1.0.0"
"org.community.math-utils" = ">=2.1.0"

```

#### Fields Schema

| Field          | Type     | Required | Description                                                            |
| -------------- | -------- | -------- | ---------------------------------------------------------------------- |
| `name`         | `string` | **Yes**  | Human-readable display name.                                           |
| `description`  | `string` | **Yes**  | Detailed description of the mod's function.                            |
| `dependencies` | `table`  | No       | Map of Mod ID keys (`string`) to SemVer rule values (`string`).        |
| `entrypoint`   | `string` | No       | Unique symbol name of a registration function called on boot (see §6). |

---

### 3.2 Code Modules (`module.wasm` and `module.js`)

#### `module.wasm` Specification & Relocation Rules

- **Target:** `wasm32-unknown-unknown` (or dynamic linking side-module targets).
- **Shared Memory & Table:** Must import shared linear memory (`env.memory`) and indirect function table (`env.__indirect_function_table`).
- **Symbol Uniqueness:** Because Binaryen merges all `.wasm` binaries into a single global symbol table, exported functions **must not** use generic names like `mod_init`. Symbols must be explicitly unique (e.g., RDN-prefixed: `com_company_mod_register`) or mapped via `manifest.toml`.
- **Memory & Table Relocation:** To prevent linear memory corruption during binary synthesis:

1. Compilers must output relocatable modules that import runtime base offsets (`env.__memory_base` and `env.__table_base`), **OR**
2. The Saga Launcher synthesis pass uses Binaryen AST relocation passes to re-base static data segment offsets and function pointers into disjoint memory blocks prior to instantiation.

#### `module.js` Specification & Security Scope

`module.js` runs directly within the standard browser execution environment alongside the Saga host engine. Standard browser context security applies.

`module.js` must be an ES Module exporting:

1. `imports`: An object containing host functions exposed to the unified WASM environment.
2. An optional registration entrypoint matching `manifest.toml`.

```javascript
// Example module.js
export const imports = {
  com_company_mod_custom_log: (ptr, len) => {
    // Read string from shared WASM linear memory
  },
};

// Unique registration function exported to match manifest.toml
export function com_company_mod_register(exports, memory, table) {
  console.log("Mod registered!");
}
```

---

### 3.3 `assets/` Directory

Assets are accessed across mods using the Saga Asset Protocol (URI syntax):
`saga://<mod-id>/<path-to-asset>`

- **Internal:** `saga://self/textures/grass.png` (resolves to current mod).
- **Cross-Mod:** `saga://com.saga.official.core/audio/click.wav`.

---

## 4. Pre-defined System Imports (Saga Standard Library)

Saga provides host-level bindings under standardized `saga:*` namespaces.

### 4.1 Asset Management (`saga:assets`)

```rust
extern "C" {
    /// Requests an asset buffer handle from storage.
    fn saga_asset_open(uri_ptr: *const u8, uri_len: usize) -> i32;

    /// Queries byte length of an asset handle.
    fn saga_asset_get_size(handle: i32) -> usize;

    /// Reads asset bytes into linear memory.
    fn saga_asset_read(handle: i32, dest_ptr: *mut u8, length: usize) -> i32;

    /// Closes an asset handle.
    fn saga_asset_close(handle: i32);
}

```

---

### 4.2 Multithreading (`saga:thread`)

```rust
extern "C" {
    /// Spawns a Web Worker executing a WASM table function pointer index.
    fn saga_thread_spawn(entry_idx: usize, arg_ptr: usize) -> i32;

    /// Yields execution on the current thread.
    fn saga_thread_yield();
}

```

---

### 4.3 Structured Logging (`saga:log`)

Provides structured, engine-tagged logging output separated by level.

```rust
extern "C" {
    /// Writes a log message to the host engine log.
    ///
    /// - level: 0 = Trace, 1 = Debug, 2 = Info, 3 = Warn, 4 = Error
    fn saga_log(level: u32, msg_ptr: *const u8, msg_len: usize);
}

```

---

### 4.4 Engine Clock & Time (`saga:time`)

Provides high-precision time and frame delta data.

```rust
extern "C" {
    /// Returns time elapsed since the last frame (in seconds).
    fn saga_time_delta() -> f32;

    /// Returns total engine execution time since boot (in seconds).
    fn saga_time_elapsed() -> f64;

    /// Returns total fixed engine ticks executed.
    fn saga_time_ticks() -> u64;
}

```

---

### 4.5 Save File System (`saga:storage`)

`saga:storage` exposes a complete save file subsystem allowing mods to inspect, read, write, and delete save files and metadata records.

```rust
extern "C" {
    /// Writes a JSON-formatted list of all available save files and metadata
    /// into `out_buf`. Returns actual byte length written (or negative error code).
    fn saga_save_list(out_buf: *mut u8, max_len: usize) -> i32;

    /// Reads metadata JSON for a specific save identifier.
    fn saga_save_read_meta(
        save_id_ptr: *const u8, save_id_len: usize,
        meta_buf: *mut u8, max_len: usize
    ) -> i32;

    /// Reads binary/text save payload into linear memory.
    fn saga_save_read(
        save_id_ptr: *const u8, save_id_len: usize,
        dest_ptr: *mut u8, max_len: usize
    ) -> i32;

    /// Writes/overwrites a save payload and its metadata record.
    fn saga_save_write(
        save_id_ptr: *const u8, save_id_len: usize,
        data_ptr: *const u8, data_len: usize,
        meta_ptr: *const u8, meta_len: usize
    ) -> i32;

    /// Deletes a save file entry.
    fn saga_save_delete(save_id_ptr: *const u8, save_id_len: usize) -> i32;
}

```

---

## 5. Runtime Resolution & Loading Pipeline

When Saga launches an instance, it executes the following load pipeline:

```text
   [ 1. Manifest Resolution & Dependency Graph Sort ]
                           │
                           ▼
   [ 2. Binaryen Synthesis & Static Relocation Pass ]
   Merges `.wasm` modules; rebases data/table segments
                           │
                           ▼
   [ 3. WebAssembly Instantiation ]
   Allocates SharedArrayBuffer & instantiates unified module
                           │
                           ▼
   [ 4. Phase 1: Registration Pass (`entrypoint`) ]
   Executes mod registration symbols in dependency-first order
                           │
                           ▼
   [ 5. Phase 2: Engine Launch Pass (`saga_start`) ]
   Launcher invokes root base game launch entrypoint

```

1. **Manifest Validation:** Reads `manifest.toml` files, resolves dependency trees, and establishes execution order.
2. **Binaryen Synthesis & Relocation:** Combines all `.wasm` modules into a single `WebAssembly.Module`. Static data segment offsets (`__memory_base`) and indirect call indices (`__table_base`) are assigned non-overlapping memory regions.
3. **JS Aggregation & Instantiation:** Merges JS `imports` objects and instantiates the merged WASM binary with unified `SharedArrayBuffer` memory and `WebAssembly.Table`.
4. **Phase 1 (Registration Pass):** Invokes mod entrypoints declared in `manifest.toml` in dependency-first order to register content and function table hooks.
5. **Phase 2 (Engine Launch Pass):** Invokes `saga_start()` on the base game to initiate the active game loop.

---

## 6. Two-Phase Lifecycle & Entrypoint Contracts

To prevent a base game or complex mod from blocking dependent mods during boot (e.g., entering a continuous loop during initialization), Saga enforces a **Two-Phase Boot Architecture**.

```text
                                 BOOT LIFECYCLE
                                        │
    ┌───────────────────────────────────┴───────────────────────────────────┐
    │                                                                       │
    ▼                                                                       ▼
[ Phase 1: Registration ]                                  [ Phase 2: Engine Launch ]
• Executed for ALL mods in dependency-first order          • Executed ONCE on root base game
• MUST be non-blocking (setup, function table hooks)       • Starts `requestAnimationFrame` / workers
• Function symbol specified in `manifest.toml`             • Standardized symbol name: `saga_start`

```

---

### 6.1 Phase 1: Registration Pass (`entrypoint`)

- **Execution Order:** Dependency-first topological order (Base Game $\rightarrow$ Core Libraries $\rightarrow$ High-level Mods). A dependency's entrypoint is called **before** any mod that relies on it.
- **Non-Blocking Rule:** Every `entrypoint` function must perform memory allocations, register callback function pointers into shared tables, hook game functions, and return immediately (`0`). **It must not enter a blocking game loop.**
- **Symbol Naming:** The entrypoint symbol name is defined in `manifest.toml` (`entrypoint = "com_example_register"`). To avoid Binaryen merge collisions, entrypoint function names must be unique across all active mods.

#### C-ABI Entrypoint Contract

```rust
// Exported C-ABI function matching `entrypoint` in manifest.toml
#[no_mangle]
pub extern "C" fn com_company_mod_register(
    wasm_exports: *const c_void,
    memory_handle: *const c_void,
    table_handle: *const c_void
) -> i32;

```

- **Parameters:**
- `wasmExports`: Reference to the global export table.
- `memory`: Pointer handle to the unified linear memory.
- `table`: Pointer handle to the shared indirect function table.

- **Return Code:** `0` indicates success; non-zero indicates registration failure.

---

### 6.2 Phase 2: Engine Launch Pass (`saga_start`)

Once the Saga Launcher confirms that **every active mod's registration entrypoint has returned `0**`, Phase 2 begins.

- **Single Execution:** The launcher calls `saga_start()` **only once** on the primary base game module (`net.saga.official.base-game` or equivalent root engine).
- **Execution Contract:** `saga_start` initiates the primary game execution loop using non-blocking browser mechanics (e.g., `requestAnimationFrame` callbacks or worker event ticks).
- **Symbol Contract:**

```rust
// Standardized single entrypoint symbol exported by the base game engine
#[no_mangle]
pub extern "C" fn saga_start() -> i32;

```

---

## 7. License

This specification is dual-licensed under MIT or Apache-2.0.
