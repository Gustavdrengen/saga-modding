# Saga Platform Mod Specification

## 1. Scope and terminology

Saga is a WebAssembly mod platform. A mod is a directory containing a manifest and zero or more WebAssembly, JavaScript, and asset files.

This document is normative. **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, and **SHALL NOT** state requirements. **MAY** and **OPTIONAL** state permitted choices. No behavior outside this document is part of the Saga contract.

Saga does not identify, inspect, or special-case the source language, compiler, standard library, allocator, or runtime used to produce a mod. The launcher operates on archives, WebAssembly linking metadata, symbols, relocations, functions, globals, memories, tables, data, and imports.

A **link archive** is the file `module.a` shipped by a WebAssembly mod. It is an `ar` archive containing one or more relocatable WebAssembly object files. A **final module** is the single executable WebAssembly module produced by the Saga launcher after linking all active mod archives. A **host import** is an import implemented by the Saga host, including the `saga:*` APIs. A **peer import** is an undefined WebAssembly symbol resolved from another mod archive during the final link.

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

Host imports MUST remain imports in the final module. The launcher MUST satisfy host imports at instantiation using the `saga:*` host namespaces in §8. The launcher MUST reject every unresolved import that is not a declared host import.

An ordinary direct call to a peer symbol MUST become a direct call to the resolved function in the final WebAssembly module. The launcher MUST NOT route that call through JavaScript or a host callback. A source function that explicitly uses an indirect call MUST retain indirect-call semantics, and the launcher MUST construct the final function table accordingly.

## 5. Final linking and runtime architecture

For an active set containing WebAssembly mods, the launcher MUST perform these steps in order:

```text
1. Validate manifests and resolve dependencies.
2. Validate every module.a archive and every relocatable member.
3. Collect every active module.a as a wasm-ld input.
4. Root every symbol listed in every manifest's exports array.
5. Link all archives with wasm-ld into one final WebAssembly module.
6. Resolve peer imports and preserve declared saga:* host imports.
7. Produce one final linear memory and one final function table.
8. Run the mandatory Binaryen optimization passes in §7.
9. Validate the optimized final WebAssembly module.
10. Instantiate exactly one final WebAssembly instance.
11. Run WebAssembly registration entrypoints.
12. Run JavaScript registration functions.
13. Invoke the root module's saga_start function exactly once.
```

The launcher MUST use `wasm-ld` or a linker implementing the same WebAssembly relocatable-object and archive semantics. The launcher MUST NOT use `wasm-merge` on finalized per-mod WebAssembly executables because `module.a` is the only WebAssembly artifact accepted in a Saga package.

The final module MUST contain exactly one linear memory and exactly one function table. All linked WebAssembly functions execute in that final instance. JavaScript modules execute in the host JavaScript environment and are not converted into WebAssembly.

The launcher MUST run linker-generated initialization functions before WebAssembly registration entrypoints. Registration MUST NOT replace language-runtime initialization.

## 6. Memory, runtime, and allocator semantics

The final linked module has one linear memory and one function table. Memory layout, stack layout, data placement, globals, function indices, table indices, and relocation are determined by the final linker. Saga does not assign a separate `max_memory` reservation to each mod and manifests MUST NOT contain a `max_memory` field.

A mod MAY contain its normal standard library, allocator, panic/exception machinery, garbage collector, startup code, and other runtime code inside `module.a`. Saga MUST NOT replace or configure those components based on the source language.

The linker MUST apply ordinary WebAssembly symbol, archive, weak, COMDAT, and relocation semantics. A duplicate strong definition is a link error. A weak or COMDAT definition is handled according to the WebAssembly linking convention. The final optimization pass MAY remove structurally identical functions after linking, but it MUST NOT merge mutable allocator state, writable data, mutable globals, stacks, or initialization state merely because their initial values or function bodies look similar.

Two modules MAY use the same runtime implementation. The linker and optimizer MUST retain one implementation when ordinary symbol/COMDAT resolution or exact function equality proves that one copy is sufficient. The final module MAY contain multiple implementations of semantically equivalent runtime behavior when the WebAssembly structure, symbols, or mutable state are not identical.

Saga MUST NOT require module authors to use a Saga allocator or a Saga runtime. A Rust module using `std` uses the Rust runtime emitted by its build. A C module uses the runtime emitted by its build. The same rule applies to every other supported language.

Pointers MUST NOT cross a mod boundary unless the participating public ABI defines pointer ownership, memory layout, lifetime, and allocator compatibility. Scalar arguments and exported functions are the default cross-mod ABI.

The launcher MUST reject a final link that produces more than one memory or more than one table. Memory growth is growth of the single final memory; Saga does not provide per-mod memory quotas in this specification.

## 7. Mandatory post-link optimization

After `wasm-ld` produces the final module and before instantiation, the launcher MUST run these passes in order:

1. **Duplicate function elimination.** Remove a defined function only when its WebAssembly type and complete normalized WebAssembly expression tree are identical to the surviving function. Rewrite every direct call, function reference, table element, export, and initialization reference to the survivor.
2. **Dead-code elimination.** Remove functions, globals, tables, and data segments unreachable from retained exports, the start function, active table elements, host imports, or retained initialization functions.
3. **Validation.** Validate the resulting WebAssembly module and reject the final link if validation fails.

The duplicate-function pass MUST compare function type, operators, operand types, constants, immediates, referenced function identity, referenced global identity, referenced table identity, and control-flow structure. It MUST NOT merge imported functions. It MUST preserve the behavior and types of all manifest exports and table elements.

The optimization pipeline MUST be language-neutral. It MUST NOT branch on Rust, C, C++, Zig, Go, AssemblyScript, or any other source-language identifier. Source-language names MUST NOT affect optimization decisions.

The launcher MUST NOT merge non-identical functions, writable data, mutable globals, allocator state, stack state, or initialization state. Semantically equivalent functions with different normalized WebAssembly structures MUST remain separate.

## 8. Host imports and lifecycle

Saga host APIs use these imports:

### 8.1 `saga:assets`

```text
saga_asset_open(uri_ptr: i32, uri_len: i32) -> i32
saga_asset_get_size(handle: i32) -> i32
saga_asset_read(handle: i32, dest_ptr: i32, length: i32) -> i32
saga_asset_close(handle: i32)
```

### 8.2 `saga:thread`

```text
saga_thread_spawn(entry_idx: i32, arg_ptr: i32) -> i32
saga_thread_yield()
```

`entry_idx` identifies an entry in the final function table. The launcher MUST finalize the table before starting a worker.

### 8.3 `saga:log`

```text
saga_log(level: i32, msg_ptr: i32, msg_len: i32)
```

`level` is `0` Trace, `1` Debug, `2` Info, `3` Warn, or `4` Error.

### 8.4 `saga:time`

```text
saga_time_now() -> i64
saga_time_elapsed() -> f64
```

### 8.5 `saga:storage`

```text
saga_save_list(out_buf: i32, max_len: i32) -> i32
saga_save_read_meta(save_id_ptr: i32, save_id_len: i32, meta_buf: i32, max_len: i32) -> i32
saga_save_read(save_id_ptr: i32, save_id_len: i32, dest_ptr: i32, max_len: i32) -> i32
saga_save_write(save_id_ptr: i32, save_id_len: i32, data_ptr: i32, data_len: i32, meta_ptr: i32, meta_len: i32) -> i32
saga_save_delete(save_id_ptr: i32, save_id_len: i32) -> i32
```

### 8.6 WebAssembly registration

If a WebAssembly manifest has `entrypoint = "symbol"`, the final module MUST export `symbol` with signature:

```text
symbol() -> i32
```

The launcher MUST invoke each WebAssembly registration entrypoint exactly once, in dependency-first order, after linked initialization has completed. Registration MUST be non-blocking. A non-zero return code MUST abort launch.

### 8.7 JavaScript registration

A JavaScript module MUST export:

```javascript
export const imports = {};
export function <manifest entrypoint>(wasmExports, memory, table) {}
```

The launcher MUST call the JavaScript registration function exactly once after WebAssembly instantiation. It MUST pass the final WebAssembly exports object, final memory, and final table.

### 8.8 Engine launch

The root game module MUST export `saga_start() -> i32`. The launcher MUST invoke it exactly once after every registration function has returned `0`. `saga_start` MUST be non-blocking and MUST return `0` on success.

## 9. JavaScript and asset modules

JavaScript modules run in the host JavaScript environment. They MAY call final WebAssembly exports, host APIs, and `fetch("saga://<mod-id>/<path>")` for assets.

A JavaScript module MUST use the final instance's memory and table passed to its registration function. It MUST NOT assume that a peer has a separate instance, memory, or table.

Pure-data mods MUST place files under `assets/`. The launcher MUST expose each asset at `saga://<mod-id>/<relative-path>`.

## 10. Required launcher errors

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
- optimization removes or changes a required export;
- the final module fails WebAssembly validation; or
- a registration function returns a non-zero value.

## 11. License

This specification is dual-licensed under MIT or Apache-2.0, at your option.
