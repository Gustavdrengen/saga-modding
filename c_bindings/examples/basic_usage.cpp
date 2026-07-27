// c_bindings/examples/basic_usage.cpp
//
// Same idea as basic_usage.c, written in C++, to demonstrate
// that the single c_bindings/saga.h header is consumable from
// both languages without any per-language boilerplate. The
// `extern "C"` block inside saga.h provides the right linkage
// for the WASM host imports.
//
// Build:
//   clang++ --target=wasm32-unknown-unknown -nostdlib \
//            -Wl,--no-entry -Wl,--allow-undefined \
//            -I ../c_bindings \
//            -o basic_usage.wasm basic_usage.cpp

#include "saga.h"

#include <cstddef>
#include <string_view>

// Saga entrypoint matching a manifest's `entrypoint = "..."` field.
// Non-blocking per MOD_SPEC.md §6.1 — emit one diagnostic line
// and return immediately.
extern "C" __attribute__((export_name("com_example_basic_register")))
int com_example_basic_register() {
    using namespace std::string_view_literals;
    constexpr std::string_view msg =
        "c_bindings/examples/basic_usage.cpp: register() called"sv;
    // Info level = 2 (see MOD_SPEC §4.3 / saga_log)
    saga_log(
        2u,
        reinterpret_cast<const uint8_t *>(msg.data()),
        static_cast<std::size_t>(msg.size()));
    return 0;
}
