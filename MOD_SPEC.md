# Saga Platform Mod Specification

## 1. Scope and terminology

Saga is a WebAssembly mod platform. A mod is a directory containing a manifest and zero or more WebAssembly, JavaScript, and asset files.

This document is normative. **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, and **SHALL NOT** state requirements. **MAY** and **OPTIONAL** state permitted choices. No behavior outside this document is part of the Saga contract.

Saga does not identify, inspect, or special-case the source language, compiler, standard library, allocator, or runtime used to produce a mod. The launcher operates on archives, WebAssembly linking metadata, symbols, relocations, functions, globals, memories, tables, data, and imports.

A **link archive** is the file `module.a` shipped by a WebAssembly mod. It is an `ar` archive containing one or more relocatable WebAssembly object files. A **final module** is the single executable WebAssembly module produced by the Saga launcher after linking all active mod archives. A **host import** is an import implemented by the Saga host, including the `saga:*` APIs. A **peer import** is an undefined WebAssembly symbol resolved from another mod archive during the final link.

A **main instance** is the first instance of the final module, created by the launcher on the main thread. A **worker instance** is a second instance of the same final module, created by the launcher inside a Web Worker (or an equivalent host worker) for the purpose of executing a **worker entry**. A **worker entry** is a WebAssembly function, exported by the final module with signature `(i32) -> ()`, invoked on a worker instance. **Shared-memory mode** is the configuration of the final link and launcher described in §7, used whenever the final module may execute worker entries.

A `module.a` archive is not executable and MUST NOT be instantiated directly. Only the final module produced by the launcher is executable.

## 2. Mod layout

A valid mod has this layout:

```text
my-mod/
├── manifest.toml         REQUIRED
├── module.a              REQUIRED for a WebAssembly mod
├── module.js             OPTIONAL JavaScript module
├── README.md             OPTIONAL documentation
├── src/                  OPTIONAL source code
└── assets/               OPTIONAL asset files
```

A mod MUST contain `module.a`, `module.js`, or an `assets/` directory. A mod containing WebAssembly code MUST contain exactly one file named `module.a`. A WebAssembly mod MUST NOT ship `module.wasm`, `.o`, `.rlib`, or any other Wasm link artifact as part of the Saga package.

`module.a` MUST be a standard `ar` archive. Every payload member MUST be a relocatable WebAssembly object that contains the WebAssembly linking metadata required by `wasm-ld`, including the `linking` custom section. Standard `ar` symbol-index members are permitted and are not payload members. The launcher MUST reject an archive containing a non-WebAssembly payload member or a finalized executable WebAssembly module.

The package format exposes only `module.a`. The number of object members inside the archive is an implementation detail of the producing toolchain.

## 3. Manifest

Every `manifest.toml` MUST contain:

```toml
id          = "com.example.foo"
version     = "1.0.0"
name        = "Example Mod"
description = "A complete description of the mod."
```

`id` MUST use reverse-domain notation and contain only lowercase letters, digits, and hyphens in dot-separated segments. `version` MUST be a semantic version. `name` and `description` MUST be non-empty strings.

A WebAssembly mod MUST contain an `exports` array listing every public WebAssembly function that the launcher must retain and expose from the final module:

```toml
entrypoint = "com_example_foo_register"
exports = [
    "com_example_foo_register",
    "com_example_foo_tick",
]
```

Every name in `exports` MUST be a valid WebAssembly function symbol. Every listed symbol MUST be defined by exactly one member of `module.a`. The launcher MUST retain every listed symbol and MUST export it from the final module. `entrypoint`, when present, MUST appear in `exports`.

A mod that defines any of the shared-memory lifecycle exports of §7.1.1 (`saga_main_init`, `saga_worker_init`, `saga_worker_stack_size`, `saga_worker_tls_size`, `saga_worker_tls_align`) MUST list that symbol in `exports`. A mod that defines a worker entry (a public function with signature `(i32) -> ()` intended for `saga_thread_spawn`) MUST list that symbol in `exports`.

A JavaScript-only mod MAY contain an `entrypoint` string and MUST NOT contain an `exports` array. A pure-data mod MUST NOT contain either field.

`[dependencies]` is optional. Each key MUST be a valid mod ID and each value MUST be a supported semantic-version constraint. The launcher MUST resolve dependencies before linking and MUST execute registration in dependency-first topological order.

## 4. Language-neutral link contract

A mod author MAY use any compiler and language that can produce `module.a` satisfying §2. The launcher MUST NOT require a language-specific runtime, allocator, standard library, compiler plugin, source transformation, or replacement library.

The link archive MUST use the WebAssembly relocatable-object convention understood by `wasm-ld`. Relocatable members MUST preserve their `linking` and `reloc.*` custom sections. The linker MUST use symbol and relocation metadata rather than source-language rules.

Public peer functions MUST use unique RDN-derived symbols such as:

```text
com_example_arena_physics_tick
com_example_arena_physics_get_ball_x
com_example_arena_ai_tick
```

A public function imported by one mod MUST have the same WebAssembly function type as the definition supplied by another mod. The launcher MUST reject an unresolved peer symbol and MUST reject incompatible duplicate strong definitions.

Host imports MUST remain imports in the final module. The launcher MUST satisfy host imports at instantiation using the `saga:*` host namespaces in §9. The launcher MUST reject every unresolved import that is not a declared host import.

An ordinary direct call to a peer symbol MUST become a direct call to the resolved function in the final WebAssembly module. The launcher MUST NOT route that call through JavaScript or a host callback. A source function that explicitly uses an indirect call MUST retain indirect-call semantics, and the launcher MUST construct the final function table accordingly (§7.4).

## 5. Final linking and runtime architecture

For an active set containing WebAssembly mods, the launcher MUST perform these steps in order:

```text
1. Validate manifests and resolve dependencies.
2. Validate every module.a archive and every relocatable member.
3. Collect every active module.a as a wasm-ld input.
4. Root every symbol listed in every manifest's exports array.
5. Link all archives with wasm-ld into one final WebAssembly module.
6. Resolve peer imports and preserve declared saga:* host imports.
7. Produce one final linear memory and one deterministic final function-table layout.
8. Run the mandatory Binaryen optimization passes in §8.
9. Validate the optimized final WebAssembly module.
10. When shared-memory mode is enabled: validate the §7.3 shared-memory requirements,
    create one launcher-owned shared WebAssembly.Memory at its full declared size,
    create the main instance's function table, and instantiate exactly one main
    instance importing that memory and that table. Otherwise, instantiate exactly
    one main instance with a launcher-owned memory and table per §6.
11. Run main initialization exactly once (§7.1.2 in shared-memory mode; linker-generated
    initialization otherwise) and then run WebAssembly registration entrypoints (§9.6).
12. Run JavaScript registration functions (§9.7).
13. Invoke the root module's saga_start function exactly once (§9.8).
```

The launcher MUST use `wasm-ld` or a linker implementing the same WebAssembly relocatable-object and archive semantics. The launcher MUST pass `--shared-memory` when the final module exposes `saga_thread_spawn` or otherwise enables worker execution. Whenever `--shared-memory` is used, every input and the final module MUST support the WebAssembly `atomics` and `bulk-memory` features, and every producer toolchain MUST enable those features for its shared-memory objects. The exact flag set and final-module requirements for shared-memory mode are defined in §7.3.

The launcher MUST NOT use `wasm-merge` on finalized per-mod WebAssembly executables because `module.a` is the only WebAssembly artifact accepted in a Saga package.

The final module MUST contain exactly one linear memory and exactly one function table. In shared-memory mode, the memory MUST be imported and shared, and every worker instance MUST import that same memory; the table MUST be imported per §7.4. A worker is a Web Worker (or equivalent host worker) that instantiates the same final module; it MUST NOT create a private linear memory. JavaScript modules execute in the host JavaScript environment and are not converted into WebAssembly.

## 6. Memory, runtime, allocator, and worker-state semantics

The final linked module has one linear memory and one function table. Memory layout, stack layout, data placement, globals, function indices, table indices, and relocation are determined by the final linker. Saga does not assign a separate `max_memory` reservation to each mod and manifests MUST NOT contain a `max_memory` field.

In shared-memory mode, the main instance and every worker instance use the same shared linear memory; the main stack, every worker stack, and every thread-local-storage region MUST occupy disjoint regions (§7.2, §7.3). Main initialization and per-worker initialization are specified in §7.1.

A mod MAY contain its normal standard library, allocator, panic/exception machinery, garbage collector, startup code, and other runtime code inside `module.a`. Saga MUST NOT replace or configure those components based on the source language.

The linker MUST apply ordinary WebAssembly symbol, archive, weak, COMDAT, and relocation semantics. A duplicate strong definition is a link error. A weak or COMDAT definition is handled according to the WebAssembly linking convention. The final optimization pass MAY remove structurally identical functions after linking, but it MUST NOT merge mutable allocator state, writable data, mutable globals, stacks, TLS, or initialization state merely because their initial values or function bodies look similar.

Two modules MAY use the same runtime implementation. The linker and optimizer MUST retain one implementation when ordinary symbol/COMDAT resolution or exact function equality proves that one copy is sufficient. The final module MAY contain multiple implementations of semantically equivalent runtime behavior when the WebAssembly structure, symbols, or mutable state are not identical.

Saga MUST NOT require module authors to use a Saga allocator or a Saga runtime. A Rust module using `std` uses the Rust runtime emitted by its build. A C module uses the runtime emitted by its build. The same rule applies to every other supported language.

Pointers MUST NOT cross a mod boundary unless the participating public ABI defines pointer ownership, memory layout, lifetime, and allocator compatibility. A worker argument pointer MUST address the shared linear memory and remain valid for the worker's use (§7.6.2).

The launcher MUST reject a final link that produces more than one memory or more than one table. In shared-memory mode, memory growth is defined in §7.3.6; Saga does not provide per-mod memory quotas in this specification.

## 7. Worker threading contract

This section defines the complete contract for Web Worker execution in shared-memory mode. Where any other section conflicts with this section, this section wins. Shared-memory mode is REQUIRED whenever the final module exposes `saga_thread_spawn` (§9.2) or any worker entry, and OPTIONAL otherwise. In shared-memory mode the requirements of §7.1 through §7.9 MUST hold; a final link that cannot satisfy them MUST be rejected by the launcher before any instance is created.

### 7.1 Module lifecycle and initialization

#### 7.1.1 Required exports

In shared-memory mode, the final module MUST export all of the following functions with exactly these signatures:

```text
saga_main_init() -> i32
saga_worker_init(stack_base: i32, stack_size: i32, tls_base: i32, tls_size: i32) -> i32
saga_worker_stack_size() -> i32
saga_worker_tls_size() -> i32
saga_worker_tls_align() -> i32
```

Every one of these exports MUST be defined by exactly one member of some active mod's `module.a`, MUST be listed in that mod's manifest `exports` array (§3), and MUST survive the mandatory optimization passes (§8). The launcher MUST reject a shared-memory final module that is missing any of these exports, exports them with a different signature, or defines any of them more than once.

#### 7.1.2 Main instance initialization

`saga_main_init() -> i32` is the main-instance initialization export. It MUST be invoked by the launcher exactly once, on the main instance, after the main instance has been instantiated (§5 step 11) and before any WebAssembly registration entrypoint (§9.6), JavaScript registration function (§9.7), or `saga_start` (§9.8) runs.

It performs all non-idempotent application and runtime startup: linker-generated constructor chains, passive-segment memory initialization (§7.3.2), allocator bootstrap, and main-instance stack/TLS establishment. It MUST NOT be invoked on any worker instance, and no other code path MAY invoke it.

The return value MUST be `0` to indicate success. A non-zero return value, a trap, or a missing export MUST abort launch with a launcher error (§11).

#### 7.1.3 Worker instance initialization

`saga_worker_init(stack_base: i32, stack_size: i32, tls_base: i32, tls_size: i32) -> i32` is the worker-initialization export. It MUST be invoked by the launcher exactly once per worker, on that worker instance, after the worker instance has been instantiated and before any module code on that worker runs, including before the worker entry is invoked.

Argument meanings:

- `stack_base` — the byte address of the lowest addressable byte of this worker's stack region in the shared linear memory. MUST be a valid shared-memory address, aligned to 16 bytes, and non-zero.
- `stack_size` — the byte length of the stack region. MUST be greater than zero, a multiple of 16, and equal to the value returned by `saga_worker_stack_size()`.
- `tls_base` — the byte address of this worker's TLS region. MUST be aligned to the value returned by `saga_worker_tls_align()`. When `tls_size` is `0`, `tls_base` MUST be `0`; otherwise it MUST be a valid shared-memory address and non-zero.
- `tls_size` — the byte length of the TLS region. MUST be a multiple of 16 and equal to the value returned by `saga_worker_tls_size()`. MAY be `0`.

Alignment requirements: `stack_base` MUST be 16-byte aligned; `tls_base` MUST be aligned to `saga_worker_tls_align()`, which MUST be a power of two between 1 and 65536 inclusive. Ownership: the regions are owned by the launcher for the lifetime of the worker and MUST NOT be freed, reallocated, grown, or reused by the module; after worker exit the launcher MAY reuse them (§7.2.5). Valid ranges: all addresses MUST be within `[0, shared_memory_size)` and the region `[stack_base, stack_base + stack_size)` and `[tls_base, tls_base + tls_size)` MUST be disjoint from every other region in the shared memory (§7.2.3).

`saga_worker_init` MUST establish the worker's stack pointer and TLS base from its arguments before returning, and MUST NOT rerun, depend on, or repeat main-instance initialization. It MUST return `0` on success. A non-zero return value or a trap MUST cause the launcher to terminate the worker, mark it as failed with the worker-startup-failure condition (§7.5.3, §7.5.5), and free its regions.

#### 7.1.4 Avoiding repeated initialization across instances

A worker instance MUST be created by instantiating the exact same `WebAssembly.Module` object as the main instance, importing the same shared `WebAssembly.Memory` (§7.3). To guarantee that instantiation itself never repeats non-idempotent application initialization, data setup, or runtime startup:

- The final module MUST NOT declare a `start` section. The launcher MUST reject a shared-memory final module that declares one, because a start function would run once per instantiation (once per worker) and cannot be suppressed language-neutrally.
- All data segments MUST be passive (§7.3.2). Passive segments do not write to memory at instantiation; the memory writes they describe MUST be performed exactly once, from `saga_main_init`, on the main instance.
- Instantiation of the shared-memory final module MUST therefore have no observable side effects on the shared memory, globals, or tables. Worker instantiation performs no data setup and no runtime startup.
- Any module-internal one-time initialization that cannot be confined to `saga_main_init` MUST be protected by a shared-memory atomic one-time guard so that its effect runs at most once across all instances. The module MUST NOT rely on per-instance state alone for such initialization.

The module MUST NOT rely on a zero stack pointer or on implicit TLS initialization. The launcher does not and cannot zero a worker's stack pointer or TLS base before `saga_worker_init` runs, and module code MUST NOT execute on a worker before `saga_worker_init` returns `0`. A worker's stack pointer and TLS base have no meaningful value until `saga_worker_init` establishes them.

### 7.2 Worker stack and TLS allocation

#### 7.2.1 Ownership

The launcher owns all worker stack and TLS allocation. A module MUST NOT allocate worker stacks or TLS itself, MUST NOT carve them out of the shared memory heap, and MUST NOT call `memory.grow` to obtain them. Worker regions come exclusively from the launcher-reserved worker arena described below and are delivered to `saga_worker_init`.

#### 7.2.2 Size discovery

The launcher MUST discover the required per-worker sizes by calling, on the main instance exactly once before any worker is created:

```text
saga_worker_stack_size() -> i32
saga_worker_tls_size() -> i32
saga_worker_tls_align() -> i32
```

These MUST return the module's minimum requirements: the stack size (positive, multiple of 16, and at least 64 KiB unless the launcher documents a lower floor it supports), the TLS size (multiple of 16, may be 0), and the TLS alignment (power of two in `[1, 65536]`). The launcher MUST validate these values and reject the launch if they are out of range or inconsistent with the final module.

#### 7.2.3 Reserved worker arena

The launcher MUST reserve one contiguous region of the shared linear memory for all worker stacks and TLS, called the worker arena. The arena:

- MUST be disjoint from all linker-placed static data, the main stack, and the heap (§7.3.5).
- MUST be aligned to 16 bytes.
- MUST have byte size `A = R * W` where `R` is the per-region size defined below, `W` is the launcher's maximum simultaneous-worker capacity, and the regions are packed contiguously with no gaps beyond required padding. `W` is a launcher configuration constant; it MUST be at least 8 and MUST be documented by the launcher.
- MUST be placed so that every worker region it contains satisfies the alignment requirements of §7.1.3.

Each worker region is a contiguous sub-range of the arena containing, in order: the worker's control buffer (64 bytes, 16-byte aligned, §7.6.3), the worker's stack, optional padding, and the worker's TLS. Let `align = max(16, tls_align)` and `pad = (align - ((64 + S) mod align)) mod align`, where `S` is the stack size and `tls_align` the TLS alignment returned by `saga_worker_tls_align()`. The region MUST be sized `R = ceil_div(64 + S + pad + T, align) * align`, so that `stack_base` is 16-byte aligned and `tls_base = stack_base + S + pad` is aligned to `tls_align` (when `T > 0`). The arena size is `A = R * W`.

#### 7.2.4 Exhaustion

When the arena contains no free region (all `W` regions are in use), `saga_thread_spawn` MUST return the exhausted-worker-memory error code (§7.5.3) and MUST NOT create a worker. The launcher MUST NOT silently shrink or overlap regions, and MUST NOT create regions outside the arena.

#### 7.2.5 Lifetime, cleanup, and reuse

A region is live from the moment a worker is spawned until that worker has fully terminated (normal exit, trap, or termination) and the launcher has reaped it. On reaping, the launcher MUST:

- release the region back to the arena free list;
- zero the region's TLS portion before any future reuse, so that a new worker never observes stale TLS;
- mark the worker id as no longer valid for `saga_thread_status` (§9.2).

The launcher MAY reuse a released region for a subsequent worker. Each reuse MUST pass fresh, correct addresses through `saga_worker_init`; a module MUST NOT cache or assume stable stack or TLS addresses across workers.

### 7.3 Shared memory and final linking

#### 7.3.1 Feature requirements

Every relocatable object that participates in a shared-memory final link, and the final module itself, MUST be compiled for the WebAssembly `atomics` and `bulk-memory` features. The launcher MUST reject any input or final module that lacks either feature, and MUST reject a final module that requires any WebAssembly feature the launcher cannot provide.

#### 7.3.2 Link flags and final-module requirements

The launcher MUST link the shared-memory final module with `wasm-ld` (or a linker with equivalent semantics) using at minimum the equivalent of:

```text
--shared-memory
--import-memory
--import-table
--max-memory=<M>          # finite, page-aligned, see §7.3.4
--no-entry
```

and MUST ensure the final module contains passive data segments only (§7.1.4). Because `wasm-ld` emits active data segments by default, the launcher MUST use a linker option or post-link transformation that converts every data segment to passive form (for example `wasm-ld --passive-segments` where supported, or an equivalent Binaryen pass); the launcher MUST reject a final module that still contains any active data segment. The final module MUST therefore:

- import exactly one linear memory, and that memory MUST be shared (its type MUST declare `shared` and a finite maximum);
- import exactly one function table per §7.4;
- declare no `start` section;
- declare no active data segments — every data segment MUST be passive;
- be valid under the WebAssembly `atomics` and `bulk-memory` feature sets.

The launcher MUST reject a final module that defines (rather than imports) its linear memory or its function table, that declares active data segments, that declares a `start` section, or that fails any other §7.3 requirement.

#### 7.3.3 Required import names

The single imported shared linear memory MUST be imported with module name `"env"` and field name `"memory"`. The single imported function table MUST be imported with module name `"env"` and field name `"__indirect_function_table"`. The launcher MUST reject a final module whose memory or table import uses different module or field names. A worker instance MUST import the same memory and table names as the main instance.

#### 7.3.4 Memory limits and creation

Let `min_pages` and `max_pages` be the minimum and maximum of the final module's imported memory type. Both MUST be finite; `max_pages` MUST be at least `min_pages`; `min_pages` MUST be at least 1; `max_pages` MUST be at most 65536 (4 GiB). The launcher MUST create the shared memory as:

```text
new WebAssembly.Memory({ initial: max_pages, maximum: max_pages, shared: true })
```

That is, the launcher MUST create the shared memory at its full declared maximum, so that `memory.grow` can never succeed. The declared `min_pages` is advisory for creation; the arena of §7.2.3 MUST fit below `max_pages * 65536` (§7.3.5). The launcher MUST reject a final module whose memory type omits a maximum or is not shared.

#### 7.3.5 Arena placement

The worker arena of §7.2.3 MUST lie entirely within the shared memory at addresses below `max_pages * 65536`, and MUST NOT overlap the module's static data, main stack, or heap. The launcher MUST verify, after linking and after discovering sizes, that the arena fits; if the declared maximum is too small, the launcher MUST reject the final link (§11). The launcher MUST NOT grow the shared memory after creation, and MUST NOT permit `memory.grow` to succeed (§7.3.4).

#### 7.3.6 Memory growth

Memory growth is NOT permitted in shared-memory mode. The launcher MUST NOT grow the shared memory after creation. Because the memory is created at its maximum, any `memory.grow` executed by the module fails (returns `-1`), which MUST be treated as ordinary allocation failure by the module. No instance may grow the shared memory, and no worker region may ever be relocated.

### 7.4 Function table and worker entry ABI

#### 7.4.1 Imported table

The final module MUST import exactly one function table with module name `"env"` and field name `"__indirect_function_table"` (§7.3.3). Its element type MUST be `funcref`. Its declared minimum and maximum MUST be equal (a fixed-size table); the launcher MUST reject a table whose minimum differs from its maximum. The launcher MUST create each instance's table with exactly these limits.

#### 7.4.2 Deterministic element initialization

The final module's element segments MUST be static and identical for every instance: the launcher MUST finalize the table layout (length, element order, and element-to-function mapping) at link time, before the main instance or any worker is created, and MUST NOT reorder or renumber elements afterward. Every instance of the same final module is initialized from the same static element segments at instantiation, so every instance's table has identical length and identical element-to-function mapping. The launcher MUST reject a final module whose table initialization is not static (for example, one whose element segments depend on runtime input or on data not fixed at link time).

#### 7.4.3 Worker entry indices

A worker entry index is a table element index, in the range `[0, table_length)`, that maps to a retained manifest export with WebAssembly signature `(i32) -> ()`. The launcher MUST record the finalized index of every worker entry at link time. `saga_thread_spawn` receives an `entry_idx` and MUST validate it against that record:

- an index outside `[0, table_length)` is invalid;
- an index that does not map to a retained worker entry is invalid;
- an index whose target function's type is not `(i32) -> ()` is invalid.

Any invalid index MUST produce the invalid-entry-index error code (§7.5.3) and MUST NOT create a worker.

#### 7.4.4 Worker entry signature and argument meaning

Every worker entry MUST have the explicit signature `(i32) -> ()`. The single argument is the `arg_ptr` value passed to `saga_thread_spawn` (§7.6.2): it is passed unchanged and MUST be interpreted by the entry as an address in the shared linear memory; the entry MUST NOT treat it as a main-thread or JavaScript pointer. The entry MUST return no value; worker completion and failure are observed through `saga_thread_status` (§9.2).

#### 7.4.5 Forbidden table mutations

The following table mutations are FORBIDDEN in a shared-memory final module: `table.set`, `table.grow`, `table.fill`, `table.copy`, `table.init`, and any `elem.drop` that could affect table initialization. In short, NO table mutation is permitted at all after instantiation: the only table writes are the static element-segment initialization that the engine performs during instantiation (§7.4.2). The launcher MUST reject a final module that contains any reachable instance of `table.set`, `table.grow`, `table.fill`, `table.copy`, `table.init`, or `elem.drop`, and MUST reject any host request with an equivalent effect. Indirect calls through the finalized table remain deterministic.

### 7.5 saga_thread_spawn behavior

#### 7.5.1 Signature and semantics

```text
saga_thread_spawn(entry_idx: i32, arg_ptr: i32) -> i32
```

`saga_thread_spawn` MUST:

1. validate `entry_idx` per §7.4.3 and `arg_ptr` per §7.6.2;
2. check that the platform can provide a Worker and a SharedArrayBuffer (§7.8), and that the module was initialized per §7.1;
3. allocate a worker region from the arena (§7.2.3) or fail with the exhausted error;
4. create a Web Worker (or equivalent host worker), hand it the exact same `WebAssembly.Module`, the shared `WebAssembly.Memory`, and the worker's region addresses, and start it;
5. in the worker: instantiate the module, run `saga_worker_init` exactly once (§7.1.3), then invoke the worker entry via the worker instance's finalized table at `entry_idx` with `arg_ptr`.

Spawning is asynchronous with respect to the worker entry: `saga_thread_spawn` MUST await the worker's initialization handshake on the worker's control channel (§7.6) until the worker has instantiated the module and completed `saga_worker_init` successfully; it then returns the worker id. Because the browser main thread MUST NOT call `Atomics.wait`, the launcher's main-thread implementation of `saga_thread_spawn` MUST await this handshake by bounded busy-polling of the worker's control buffer (a loop over atomic loads with a launcher-configurable timeout). If the worker fails before its entry could run (instantiation failure, `saga_worker_init` returned non-zero or trapped, worker died during startup, or the handshake timed out), `saga_thread_spawn` MUST return `SAGA_THREAD_ERR_STARTUP` and the worker is terminated and reaped. The spawn return does NOT wait for the worker entry to complete: completion and later failures are observed through `saga_thread_status` (§9.2).

#### 7.5.2 Return values

A non-negative return value is a worker identifier. A negative return value is an error code.

#### 7.5.3 Error codes

```text
-1  SAGA_THREAD_ERR_UNSUPPORTED       Threading is not supported: the final module is not a shared-memory module, or the launcher does not support worker execution.
-2  SAGA_THREAD_ERR_UNAVAILABLE       A Worker or SharedArrayBuffer is unavailable in this environment (e.g., the page is not cross-origin isolated, §7.8).
-3  SAGA_THREAD_ERR_NO_WORKER_INIT    The final module is missing saga_worker_init, or main initialization did not run.
-4  SAGA_THREAD_ERR_BAD_ENTRY         entry_idx is missing, out of range, or does not map to a retained (i32) -> () worker entry.
-5  SAGA_THREAD_ERR_EXHAUSTED         The worker arena cannot provide another region (all W regions are in use).
-6  SAGA_THREAD_ERR_INVALID_ARG       An argument is invalid (e.g., arg_ptr is not a valid shared-memory address).
-7  SAGA_THREAD_ERR_STARTUP           The worker was created but failed before its entry ran (instantiation failed, saga_worker_init returned non-zero or trapped).
```

These exact codes are part of the contract. The launcher MUST NOT return any other negative value from `saga_thread_spawn`.

#### 7.5.4 Worker identifiers

Worker identifiers are non-negative `i32` values. They are assigned monotonically, MUST be unique among concurrently live workers, and MUST NOT be reused while a previous worker with that id has not been reaped (§7.2.5). An id becomes invalid once its worker has been reaped; `saga_thread_status` on an invalid id MUST return the invalid-worker-id error.

#### 7.5.5 Completion, traps, and termination

- Normal exit: when a worker entry returns, the worker is completed successfully, its regions are released for reuse, and its id remains queryable until reaped.
- Trap: if a worker entry traps, the launcher MUST mark the worker failed, release its regions, and report the failure through `saga_thread_status`. A trapping worker MUST NOT affect other workers or the main instance.
- Termination: the launcher MAY terminate a worker at any time (including on engine shutdown). Termination MUST release the worker's regions, mark it terminated, and unblock any pending §7.6 wait on that worker.

### 7.6 Typed worker-to-main host-import RPC

#### 7.6.1 Scope

`saga:assets` and `saga:storage` imports (§9) are main-thread-only: when a worker instance calls one, the launcher MUST forward the call to the main thread and return the result to the worker, using the synchronous typed protocol defined here. `saga:log` and `saga:time` are worker-safe and MAY be served directly in the worker (§7.9). The forwarding protocol MUST preserve the exact import module name, import name, parameter types, and result types of every forwarded import.

#### 7.6.2 Argument validation

`arg_ptr` MUST be a valid shared-memory address: `0 <= arg_ptr < shared_memory_size`, and the launcher MUST reject values outside the shared memory with the invalid-argument error. A worker entry receives exactly the value passed; the module is responsible for interpreting it (§7.4.4).

#### 7.6.3 Control-buffer layout

Each worker has a launcher-owned control buffer inside its arena region, aligned to 16 bytes, with this fixed little-endian layout:

```text
Offset  Width  Field        Meaning
0x00    u32    state        0 = idle, 1 = request-issued, 2 = response-ready
0x04    u32    slot         worker id of the requesting worker
0x08    u32    opcode       index of the forwarded host import (launcher-assigned, stable per final module)
0x0C    u32    argc         number of valid 32-bit argument slots (0..8)
0x10    4*u32  args[8]      argument slots
0x30    u32    result_type  0 = void, 1 = i32, 2 = i64, 3 = f32, 4 = f64, 5 = error
0x34    u32    result_lo    low 32 bits of the result
0x38    u32    result_hi    high 32 bits of the result (i64/f64)
0x3C    u32    status       0 = ok, 1 = host exception, 2 = invalid operation, 3 = unsupported import
```

Total control-buffer size is 64 bytes. The buffer is owned by the launcher and written by both the worker-side host shim and the main-thread dispatcher. Every field MUST be accessed with WebAssembly atomics semantics (`i32.atomic.*`-equivalent in the host).

The `opcode` value `0xFFFFFFFF` is reserved for the worker-initialization handshake (§7.5.1): the worker writes `opcode = 0xFFFFFFFF`, `result_lo = <saga_worker_init return value>`, then performs the same state transition as a forwarded call. The main-thread dispatcher recognizes it, records startup success or failure, and posts a response so the spawner's bounded poll can observe it. No forwarded host import may use this opcode.

#### 7.6.4 Argument and result encoding

Arguments are encoded into `args[0..argc]` in signature order: `i32` occupies one slot as its raw value; `i64` occupies two slots (low 32 bits first, then high 32 bits); `f32` occupies one slot holding the IEEE-754 bit pattern reinterpreted as `i32`; `f64` occupies two slots (low 32 bits of the IEEE-754 bit pattern first, then high 32 bits). `argc` is the number of slots used.

Results are encoded with `result_type` and `result_lo`/`result_hi`: `void` uses no result slots; `i32` uses `result_lo`; `i64` uses `result_lo`/`result_hi` (low first); `f32` uses `result_lo` holding the bit pattern; `f64` uses `result_lo`/`result_hi`. `result_type = 5` signals that no value result is valid and `status` carries the reason.

#### 7.6.5 State transitions and wake-up rules

The worker-side host shim for a forwarded import MUST:

1. write `slot`, `opcode`, `argc`, and `args` (relaxed ordering);
2. store `state = 1` with release ordering and `Atomics.notify(state, 1)`;
3. loop: `Atomics.wait(state, 1, timeout)` until `state != 1` (acquire) or the wait returns `timed-out`; the worker MUST NOT busy-spin;
4. when `state == 2`, read `result_type`, `result_lo`, `result_hi`, and `status`;
5. store `state = 0` (relaxed) before returning to the module.

The main-thread dispatcher MUST, on its event loop (the main thread MUST NOT call `Atomics.wait`), for each pending worker:

1. observe `state == 1` (acquire), validate `opcode`, decode arguments, and invoke the real main-thread host implementation;
2. encode the result and store `result_type`, `result_lo`, `result_hi`, `status = 0`; on host exception store `status = 1`, on invalid operation `status = 2`, on unsupported import `status = 3`;
3. store `state = 2` with release ordering and `Atomics.notify(state, 1)`.

The worker-side wait MUST use a finite launcher-configured timeout (default at least 1000 ms). On timeout, the shim MUST return the import's error value to the module and MUST reset `state` to `0` only if it still holds `1` (compare-and-swap); if the response has already been posted, the shim consumes it.

The main-thread dispatcher MUST service every control buffer whose `state` has transitioned to `1`, including the init handshake (§7.6.3), on every event-loop turn, and MUST post a response (or, for the init handshake, record the startup outcome) before returning control to the application. `saga_thread_spawn` observes the init handshake by bounded polling (§7.5.1).

#### 7.6.6 Guarantees

- A worker MUST NOT remain blocked indefinitely: every request is answered by the main thread, every wait has a finite timeout, and host failures are always posted back as responses.
- After a host-side failure, the main thread MUST still post a response (`status != 0`) so the worker can proceed.
- After a worker is terminated, any pending wait on that worker is released by the launcher (§7.5.5), and its control buffer is reset to idle before reuse.
- Host calls are serialized: the main-thread dispatcher processes one request at a time; a worker may have at most one outstanding request (its buffer is single-flight). Concurrent requests from different workers are serialized in the order the dispatcher observes their `state` transitions.

### 7.7 Imports and exports

#### 7.7.1 Required launcher-provided imports

The launcher MUST provide, to every instance (main and worker), the imports listed in §9 with exactly the module names, import names, parameter types, and result types given there: `saga:assets`, `saga:thread`, `saga:log`, `saga:time`, `saga:storage`. In addition it MUST provide the imported shared memory (`env.memory`) and imported table (`env.__indirect_function_table`).

#### 7.7.2 Required module exports

The final module MUST export: every name in each active mod's manifest `exports` array; in shared-memory mode, the §7.1.1 lifecycle exports; and the root module's `saga_start` (§9.8).

#### 7.7.3 Unresolved imports

The launcher MUST reject any import in the final module that is not one of the required imports above (§4, §11).

#### 7.7.4 Worker legality

All `saga:*` imports are legal in workers. Imports whose state lives on the main thread (`saga:assets`, `saga:storage`) MUST be forwarded through the §7.6 protocol. `saga:thread` imports are legal in any instance and behave identically; when called from a worker, `saga_thread_spawn` and `saga_thread_status` are forwarded to the main thread. `saga:log` and `saga:time` are worker-safe and MAY be served directly in the worker, with identical observable semantics.

### 7.8 Browser and deployment requirements

Every browser execution path of a shared-memory final module MUST run in a cross-origin isolated context:

- the top-level document MUST be served with response headers `Cross-Origin-Opener-Policy: same-origin` and `Cross-Origin-Embedder-Policy: require-corp` (or `credentialless`);
- `crossOriginIsolated` MUST be `true` in the main frame and in every worker;
- `SharedArrayBuffer` and `Worker` MUST be available.

These requirements apply to BOTH the normal application server and to any generated static HTML bundle the launcher produces: every document and script that participates in shared-memory execution MUST be served with the COOP/COEP headers above, and every resource loaded by a worker (the final module, the worker script, assets) MUST be same-origin or otherwise COEP-compliant.

The launcher MUST serve a worker bootstrap script at the fixed, same-origin URL `/saga-worker.js` and the final module at the fixed, same-origin URL `/saga-module.wasm` (both relative to the launcher origin; the launcher MAY document alternate paths, which MUST remain same-origin). The worker bootstrap script receives the final module bytes, the shared memory, the worker's region addresses, and the control-buffer address, and instantiates the module and runs the §7.6 host shims. The worker script and the final module MUST be packaged such that they are accessible same-origin.

When cross-origin isolation or the worker assets are unavailable, the launcher MUST report a clear error (§11) and MUST NOT create workers; `saga_thread_spawn` MUST return `SAGA_THREAD_ERR_UNAVAILABLE`.

### 7.9 Worker-safe imports

`saga:log` and `saga:time` are worker-safe: the launcher MAY implement them directly inside a worker (for example, logging to the worker console or reading the worker clock), provided the observable semantics match the main-thread implementations. `saga:assets`, `saga:storage`, and the `saga:thread` functions are main-thread-hosted and MUST use the §7.6 protocol when called from a worker.

## 8. Mandatory post-link optimization

After `wasm-ld` produces the final module and before instantiation, the launcher MUST run these passes in order:

1. **Duplicate function elimination.** Remove a defined function only when its WebAssembly type and complete normalized WebAssembly expression tree are identical to the surviving function. Rewrite every direct call, function reference, table element, export, and initialization reference to the survivor.
2. **Dead-code elimination.** Remove functions, globals, tables, and data segments unreachable from retained exports, retained worker entries, the §7.1.1 lifecycle exports, active table elements, host imports, or retained initialization functions.
3. **Validation.** Validate the resulting WebAssembly module and reject the final link if validation fails.

The duplicate-function pass MUST compare function type, operators, operand types, constants, immediates, referenced function identity, referenced global identity, referenced table identity, and control-flow structure. It MUST NOT merge imported functions. It MUST preserve the behavior and types of all manifest exports, worker entries, and table elements. It MUST NOT remove or reorder any table element, because table indices are part of the §7.4 worker-entry ABI.

The optimization pipeline MUST be language-neutral. It MUST NOT branch on Rust, C, C++, Zig, Go, AssemblyScript, or any other source-language identifier. Source-language names MUST NOT affect optimization decisions.

The launcher MUST NOT merge non-identical functions, writable data, mutable globals, allocator state, stack state, or initialization state. Semantically equivalent functions with different normalized WebAssembly structures MUST remain separate.

## 9. Host imports and lifecycle

Saga host APIs use these imports. A worker instance MUST receive the same host import module names, import names, and WebAssembly function types as the main instance. No worker-only ABI, alternate signature, JavaScript callback, or untyped host import is part of the Saga contract. Pointer arguments in worker calls refer to the shared linear memory. Imports marked main-thread-only in §7.9 MUST use the §7.6 forwarding protocol when called from a worker.

### 9.1 `saga:assets` (main-thread-only)

```text
saga_asset_open(uri_ptr: i32, uri_len: i32) -> i32
saga_asset_get_size(handle: i32) -> i32
saga_asset_read(handle: i32, dest_ptr: i32, length: i32) -> i32
saga_asset_close(handle: i32)
```

### 9.2 `saga:thread`

```text
saga_thread_spawn(entry_idx: i32, arg_ptr: i32) -> i32
saga_thread_status(worker_id: i32) -> i32
saga_thread_yield()
```

`saga_thread_spawn` MUST implement the behavior of §7.5, including all error codes of §7.5.3. `entry_idx` is a finalized table index of a retained worker entry with type `(i32) -> ()` (§7.4); `arg_ptr` is passed unchanged to the entry and addresses the shared linear memory (§7.6.2). It is callable from any initialized instance; when called from a worker it is forwarded to the main thread (§7.7.4). A non-negative return value is a worker identifier (§7.5.4).

`saga_thread_status(worker_id: i32) -> i32` reports the state of a worker: `0` running (spawned, still executing or initializing), `1` completed normally, `2` trapped during or after the worker entry ran, `3` terminated, `4` startup-failed (failed before the entry ran, per §7.5.1). A negative return value means the worker id is invalid or has been reaped (§7.2.5, §7.5.4). It MUST be callable from any initialized instance; when called from a worker it is forwarded to the main thread (§7.7.4).

`saga_thread_yield()` cooperatively yields the calling instance. It MUST return immediately and MUST NOT block. It is worker-safe and MAY be served directly in the worker.

### 9.3 `saga:log` (worker-safe)

```text
saga_log(level: i32, msg_ptr: i32, msg_len: i32)
```

`level` is `0` Trace, `1` Debug, `2` Info, `3` Warn, or `4` Error.

### 9.4 `saga:time` (worker-safe)

```text
saga_time_now() -> i64
saga_time_elapsed() -> f64
```

### 9.5 `saga:storage` (main-thread-only)

```text
saga_save_list(out_buf: i32, max_len: i32) -> i32
saga_save_read_meta(save_id_ptr: i32, save_id_len: i32, meta_buf: i32, max_len: i32) -> i32
saga_save_read(save_id_ptr: i32, save_id_len: i32, dest_ptr: i32, max_len: i32) -> i32
saga_save_write(save_id_ptr: i32, save_id_len: i32, data_ptr: i32, data_len: i32, meta_ptr: i32, meta_len: i32) -> i32
saga_save_delete(save_id_ptr: i32, save_id_len: i32) -> i32
```

### 9.6 WebAssembly registration

If a WebAssembly manifest has `entrypoint = "symbol"`, the final module MUST export `symbol` with signature:

```text
symbol() -> i32
```

The launcher MUST invoke each WebAssembly registration entrypoint exactly once, in dependency-first order, after main initialization has completed (§7.1.2). Registration MUST be non-blocking. A non-zero return code MUST abort launch.

### 9.7 JavaScript registration

A JavaScript module MUST export:

```javascript
export const imports = {};
export function <manifest entrypoint>(wasmExports, memory, table) {}
```

The launcher MUST call the JavaScript registration function exactly once after WebAssembly instantiation. It MUST pass the final WebAssembly exports object, final memory, and final table.

### 9.8 Engine launch

The root game module MUST export `saga_start() -> i32`. The launcher MUST invoke it exactly once after every registration function has returned `0`. `saga_start` MUST be non-blocking and MUST return `0` on success.

## 10. JavaScript and asset modules

JavaScript modules run in the host JavaScript environment. They MAY call final WebAssembly exports, host APIs, and `fetch("saga://<mod-id>/<path>")` for assets.

A JavaScript module MUST use the main instance's memory and finalized table passed to its registration function. It MUST NOT assume that a peer has a separate memory or table. Worker instances use the same shared memory and the identical table layout, but are not JavaScript registration instances.

Pure-data mods MUST place files under `assets/`. The launcher MUST expose each asset at `saga://<mod-id>/<relative-path>`.

## 11. Required launcher errors

The launcher MUST reject an active mod set when:

- a manifest is invalid;
- a dependency cannot be resolved;
- a WebAssembly mod does not contain exactly one `module.a`;
- a package contains a forbidden finalized `module.wasm`, `.o`, or `.rlib` artifact;
- an archive is not a valid `ar` archive;
- an archive member is not a valid relocatable WebAssembly object;
- an archive member lacks required linking metadata;
- a manifest export is missing or has the wrong type;
- a required peer import has no compatible definition;
- two incompatible strong definitions have the same symbol;
- an unresolved non-host import remains after linking;
- the final module has more than one memory or more than one table;
- optimization removes or changes a required export or table element;
- the final module fails WebAssembly validation; or
- a registration function returns a non-zero value;
- shared-memory mode is requested but an input or the final module lacks the `atomics` or `bulk-memory` feature, or the final memory is not shared, not imported, or has no valid finite maximum;
- the required shared-memory linker/toolchain support, including `--shared-memory`, is unavailable or the final module fails shared-memory validation;
- the shared-memory final module declares a `start` section, declares active data segments, or defines rather than imports its linear memory or function table;
- the shared-memory final module's memory or table import uses a module or field name other than `env.memory` and `env.__indirect_function_table`, or the table's minimum differs from its maximum;
- the shared-memory final module is missing `saga_main_init`, `saga_worker_init`, `saga_worker_stack_size`, `saga_worker_tls_size`, or `saga_worker_tls_align`, exports any of them with the wrong signature, or defines any of them more than once;
- the browser host is not a secure, cross-origin-isolated context with `Cross-Origin-Opener-Policy: same-origin`, `Cross-Origin-Embedder-Policy: require-corp` or `credentialless`, `crossOriginIsolated == true`, `SharedArrayBuffer`, and Web Worker support, or the launcher/worker resources are not permitted by those policies;
- a worker entry is not a retained manifest export of type `(i32) -> ()`, its table index is invalid, or its table mapping is not deterministic;
- main initialization (`saga_main_init`) returns a non-zero value or traps;
- the worker arena cannot fit inside the shared memory's declared maximum, or the discovered stack/TLS sizes are invalid (§7.2.2);
- retained code or a host request attempts unsupported function-table mutation, or a shared-memory final module contains any reachable `table.set`, `table.grow`, `table.fill`, `table.copy`, `table.init`, or `elem.drop`;
- a worker host import is missing or has a module name, import name, or WebAssembly type different from the main-instance ABI; or
- the shared-memory module's stack or TLS regions would overlap another instance's state.

## 12. Validation and conformance

An implementation claiming conformance with this specification MUST pass the following mechanical checks. Each item identifies the normative section it validates.

1. **Shared-memory module validation.** The final module is a valid WebAssembly module; in shared-memory mode it declares the `atomics` and `bulk-memory` features, imports a shared linear memory with a finite maximum, declares no `start` section, and declares only passive data segments (§7.3).
2. **Exact memory and table imports.** The memory import is `env.memory` and is shared; the table import is `env.__indirect_function_table` with element type `funcref` and equal minimum/maximum; no other memory or table is imported or defined (§7.3.3, §7.4.1).
3. **Required exports and signatures.** `saga_main_init() -> i32`, `saga_worker_init(i32, i32, i32, i32) -> i32`, `saga_worker_stack_size() -> i32`, `saga_worker_tls_size() -> i32`, and `saga_worker_tls_align() -> i32` exist with exactly these signatures; every manifest export exists with its declared type; worker entries have type `(i32) -> ()` (§7.1.1, §7.4.4).
4. **Deterministic table.** The table layout is fixed at link time; two instances of the same final module produce identical table length and identical element-to-function mapping; worker entry indices are recorded and stable (§7.4.2).
5. **Forbidden table operations.** The final module contains no reachable `table.set`, `table.grow`, `table.fill`, `table.copy`, `table.init`, or `elem.drop` (§7.4.5).
6. **Worker stack/TLS allocation.** Discovered sizes are valid; every worker region is disjoint, aligned, within the shared memory, and inside the arena; the arena fits below `max_pages * 65536`; exhaustion returns `SAGA_THREAD_ERR_EXHAUSTED` (§7.2, §7.3.5).
7. **One-time main initialization.** `saga_main_init` runs exactly once on the main instance, before registration and `saga_start`; a non-zero return aborts launch (§7.1.2).
8. **Per-worker initialization.** Each worker runs `saga_worker_init` exactly once, with valid disjoint region addresses, before any module code on that worker; a non-zero return or trap terminates the worker and reports startup failure (§7.1.3).
9. **Typed RPC round-trip.** A worker call to `saga:assets` or `saga:storage` is forwarded through the §7.6 control buffer with correct encodings for `i32`, `i64`, `f32`, `f64`, and `void` results; the state machine transitions `0 -> 1 -> 2 -> 0`; host exceptions and invalid operations are reported via `status`; waits are finite (§7.6).
10. **Worker failure and cleanup.** On normal exit, trap, and termination, the worker's status is observable via `saga_thread_status`, its regions are released and TLS zeroed before reuse, and a trapping worker does not affect other workers or the main instance (§7.5.5, §7.2.5).
11. **COOP/COEP and asset availability.** The document and every generated static bundle are served with `Cross-Origin-Opener-Policy: same-origin` and `Cross-Origin-Embedder-Policy: require-corp` (or `credentialless`); `crossOriginIsolated` is `true`; the worker bootstrap script and final module are same-origin; when unavailable, `saga_thread_spawn` returns `SAGA_THREAD_ERR_UNAVAILABLE` and a clear error is reported (§7.8).

## 13. License

This specification is dual-licensed under MIT or Apache-2.0, at your option.

