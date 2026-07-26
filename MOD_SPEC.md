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

# Dependencies mapping required Mod IDs to semantic version constraints
[dependencies]
"com.saga.official.core" = "^1.0.0"
"org.community.math-utils" = ">=2.1.0"

```

#### Fields Schema

| Field          | Type     | Required | Description                                                     |
| -------------- | -------- | -------- | --------------------------------------------------------------- |
| `name`         | `string` | **Yes**  | Human-readable display name.                                    |
| `description`  | `string` | **Yes**  | Detailed description of the mod's function.                     |
| `dependencies` | `table`  | No       | Map of Mod ID keys (`string`) to SemVer rule values (`string`). |

---

### 3.2 Code Modules (`module.wasm` and `module.js`)

Both code files are optional individually, but a functional mod will typically contain at least one.

#### `module.wasm` Specification

- **Target:** `wasm32-unknown-unknown` (or similar bare-metal WASM target).
- **Imports:** Must import shared linear memory (`env.memory`) and indirect function table (`env.__indirect_function_table`).
- **Export Naming:** Symbols exported by WASM should be uniquely prefixed using the C ABI (e.g., `extern "C" fn com_company_mod_init()`) to prevent Binaryen symbol collision during runtime merging.

#### `module.js` Specification

`module.js` must be a valid ES Module exporting two primary constructs:

1. `imports`: An object containing functions exposed to the unified WASM environment.
2. `init(wasmExports, memory, table)`: A lifecycle function called by the engine after all WASM binaries are merged and instantiated.

```javascript
// Example module.js
export const imports = {
  // Saga standard host function extension or peer API
  com_company_mod_custom_log: (ptr, len) => {
    // Read string from shared memory
  },
};

export function init(exports, memory, table) {
  // Save global WASM export references
  console.log("Mod com.company.mod initialized!");
}
```

---

### 3.3 `assets/` Directory

Contains arbitrary files (models, textures, audio files, JSON data). The directory structure inside `assets/` is left entirely to the discretion of the mod author.

Assets are accessed across mods using the Saga Asset Protocol (URI syntax):
`saga://<mod-id>/<path-to-asset>`

- **Example (Internal):** `saga://self/textures/grass.png` (resolves to the current mod's assets).
- **Example (Cross-Mod):** `saga://com.saga.official.core/audio/click.wav`.

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

---

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
                                 Invoke `init()` on all `module.js` files

```

1. **Manifest Validation:** Reads `manifest.toml` files, resolves dependency trees, and builds an execution graph.
2. **Binaryen Synthesis:** Reads all `module.wasm` binaries, merges them into a single `WebAssembly.Module` using Binaryen's AST merger, and links shared atomic memory.
3. **JS Aggregation:** Dynamically imports all `module.js` files, merging their `imports` objects into a single global `importObject`.
4. **Unified Instantiation:** Instantiates the merged WASM binary with the unified `importObject`, shared `SharedArrayBuffer`, and shared `WebAssembly.Table`.
5. **Initialization Hook:** Iterates over each mod's `module.js`, calling `init(exports, memory, table)` to pass global control down to all active mods.
