# `saga-stdlib` C / C++ bindings

Header-only binding for the Saga platform standard library, exposing
every host import declared in [`MOD_SPEC.md` §4](../MOD_SPEC.md) to
module authors who target `wasm32-unknown-unknown` from C or C++.

`example/mods/arena-ai` consumes this header to compile against the
`saga:log` and `saga:thread` host namespaces without needing any
per-mod import declarations.

---

## Single header, two languages

`c_bindings/saga.h` is the only public artifact. It is `#pragma
once`-guarded and wraps every declaration in `extern "C"` so a
single source file is consumable from both C and C++:

```c
#include "saga.h"   // C
```

```cpp
#include "saga.h"   // C++
```

A separate `saga.hpp` is **not** published alongside the C header
because every entry point is already a plain `extern "C"` function
and clang accepts the `[import_module]`/`[import_name]` attribute
syntax in both languages. Keeping a single source of truth means
the C and C++ bindings cannot drift apart — and if a future
need arises for a C++-only ergonomic wrapper (namespaces, RAII,
`std::string_view` adapters, …), it can be added later as an
additive `saga.hpp` _on top of_ this header rather than as a
parallel declaration set.

This mirrors the canonical pattern used by zlib, libpng,
SQLite, curl, and SDL2 for the same reason.

---

## Compiling against it

A C or C++ source file becomes a `wasm32-unknown-unknown` guest
by pointing clang at the binding and disabling the host runtime:

```bash
clang --target=wasm32-unknown-unknown -nostdlib \
      -Wl,--no-entry -Wl,--allow-undefined \
      -I path/to/c_bindings \
      -c my_module.c -o my_module.o
```

`example/build.sh` already does this for the C mods under
`example/mods/`.

---

## The `SAGA_HOST_IMPORT` macro

Each declaration in `saga.h` is preceded by the macro so the
WebAssembly import-module and import-name attributes animate
correctly:

```c
SAGA_HOST_IMPORT("saga:log", log)
extern void saga_log(uint32_t level, const uint8_t *msg_ptr, size_t msg_len);
```

**Critical contract:** the second argument is a **bare
identifier**, not a string literal.

| Invocation                            | Import name produced              |
| ------------------------------------- | --------------------------------- |
| `SAGA_HOST_IMPORT("saga:log", log)`   | `saga_log` ✓                      |
| `SAGA_HOST_IMPORT("saga:log", "log")` | `saga_"log"` ✗ — leads with a `"` |

The macro works by stringifying `name` with the `#` preprocessor
operator and then letting adjacent string-literal concatenation
assemble the canonical `saga_<name>` form:

```
"saga_" #log   →  "saga_" "log"   →  "saga_log"
```

If you instead pass `"log"` (a string literal), `#name` escapes
the surrounding quotes _into the string itself_, producing
`"\"log\""` — and after concatenation with `"saga_"` you get
the import name `saga_"log"` (literal quote characters baked in).
The Saga Launcher's host-binding map will not have that key, so
instantiation will fail.

The bug appeared in `example/mods/arena-ai/src/saga.h` (now
removed) and produced wasm text-format imports like:

```wat
(import "saga:log" "saga_\22log\22" (func $saga_log (type 0)))
```

— `\22` is the WASM-text encoding of a literal `"` byte, so the
_real_ import name is `saga_"log"`.

---

## Examples

- [`examples/basic_usage.c`](examples/basic_usage.c) — C program
  that emits one `saga:log` line.
- [`examples/basic_usage.cpp`](examples/basic_usage.cpp) — C++
  program that does the same through `std::string_view`.

Both compile to a `wasm32-unknown-unknown` object that imports
`saga:log.log`. They do not link into a fully-instantiable game
on their own — instantiate them with a host environment that
provides that import.

---

## Layout

```
c_bindings/
├── README.md
├── saga.h              ← the binding (only public artifact)
└── examples/
    ├── basic_usage.c
    └── basic_usage.cpp
```

Parallel to `rust_bindings/` at the repo root.
