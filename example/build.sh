#!/usr/bin/env bash
# build.sh — compile every mod under ./mods/ and emit a shippable
# runtime tree at ./dist/<mod-name>/.
#
# Per-mod dist layout:
#   dist/<mod-name>/
#       manifest.toml          (copied verbatim)
#       README.md              (copied verbatim, if present)
#       module.wasm            (emitted by Rust/C build)
#       module.js              (copied verbatim for JS mods)
#       assets/...             (copied verbatim for data mods)
#
# Build contract:
#   1. The ONLY file outputs are under dist/<mod-name>/.
#      Anything else left behind on disk is a bug.
#   2. Intermediate Cargo target dirs live under build/<mod-name>/
#      and are wiped on every exit (success, error, or signal) so
#      they never accumulate between invocations.
#   3. This script does NOT run a Saga engine. It only produces
#      the per-mod artifacts the Saga Launcher would consume.
#      See ./README.md and the top-level MOD_SPEC.md for context.

set -euo pipefail

# Resolve paths up front so every later reference is non-empty and
# absolute. Bail with a clear error if ROOT can't be derived.
cd "$(dirname "$0")"
ROOT="$(pwd -P)"
MODS_DIR="$ROOT/mods"
DIST_DIR="$ROOT/dist"
SCRATCH_DIR="$ROOT/build"
# Top-level C/C++ bindings (saga.h). C mods `#include "saga.h"`
# against this path, replacing the old per-mod saga.h pattern
# that double-stringified and produced import names like
# `saga_"log"`.
C_BINDINGS_DIR="$(cd "$ROOT/.." && pwd -P)/c_bindings"

[ -n "$SCRATCH_DIR" ]    && [ "${SCRATCH_DIR#/}"    != "$SCRATCH_DIR"    ] \
  || { echo "SCRATCH_DIR '$SCRATCH_DIR' must be an absolute path" >&2; exit 2; }
[ -n "$DIST_DIR" ]       && [ "${DIST_DIR#/}"       != "$DIST_DIR"       ] \
  || { echo "DIST_DIR '$DIST_DIR' must be an absolute path"    >&2; exit 2; }
[ -n "$MODS_DIR" ]       && [ "${MODS_DIR#/}"       != "$MODS_DIR"       ] \
  || { echo "MODS_DIR '$MODS_DIR' must be an absolute path"    >&2; exit 2; }
[ -d "$C_BINDINGS_DIR" ] \
  || { echo "C_BINDINGS_DIR '$C_BINDINGS_DIR' missing; the repo's c_bindings/ should sit one level above example/" >&2; exit 2; }

# Trap is installed BEFORE any destructive op so even a failing
# `mkdir -p` or `rm -rf` participates in cleanup. EXIT alone fires on
# natural exits; INT/TERM cover interactive Ctrl-C and graceful kill.
cleanup_scratch() {
  if [ -n "${SCRATCH_DIR:-}" ] \
      && [ "${SCRATCH_DIR#/}" != "$SCRATCH_DIR" ] \
      && [ -d "$SCRATCH_DIR" ]; then
    rm -rf "$SCRATCH_DIR"
  fi
}
trap cleanup_scratch EXIT INT TERM

# Always start from a clean tree so any leftovers from a previous run
# (manual `cargo` invocations, dropped debug artifacts) are swept.
rm -rf "$DIST_DIR" "$SCRATCH_DIR"
mkdir -p "$DIST_DIR"

if [ -t 1 ]; then
  BOLD=$'\033[1m'; CYAN=$'\033[1;36m'; RESET=$'\033[0m'
else
  BOLD=''; CYAN=''; RESET=''
fi
log() { echo "${CYAN}==>${RESET} ${BOLD}$*${RESET}"; }
warn() { echo "${CYAN}==>${RESET} $*" >&2; }

have() { command -v "$1" >/dev/null 2>&1; }

HAS_RUST=0
if have rustc && have cargo; then
  HAS_RUST=1
else
  warn "rustc/cargo not found – Rust mods will be skipped"
fi

HAS_CLANG=0
if have clang; then
  if clang --target=wasm32-unknown-unknown --print-target-triple >/dev/null 2>&1; then
    HAS_CLANG=1
  else
    warn "clang lacks wasm32-unknown-unknown target – C mods will be skipped"
  fi
else
  warn "clang not found – C mods will be skipped"
fi

if [ "$HAS_RUST" = "1" ]; then
  if ! rustup target list --installed 2>/dev/null | grep -q '^wasm32-unknown-unknown$'; then
    warn "wasm32-unknown-unknown rust target missing – installing..."
    rustup target add wasm32-unknown-unknown || HAS_RUST=0
  fi
fi

shopt -s nullglob

if [ ! -d "$MODS_DIR" ]; then
  warn "$MODS_DIR does not exist"
  exit 1
fi

for mod_dir in "$MODS_DIR"/*/; do
  name="$(basename "$mod_dir")"
  manifest="$mod_dir/manifest.toml"

  if [ ! -f "$manifest" ]; then
    warn "skipping $name (no manifest.toml)"
    continue
  fi

  mod_dist="$DIST_DIR/$name"
  mkdir -p "$mod_dist"
  cp "$manifest" "$mod_dist/manifest.toml"
  if [ -f "$mod_dir/README.md" ]; then
    cp "$mod_dir/README.md" "$mod_dist/README.md"
  fi

  # -------- Rust mod ----------------------------------------------------
  if [ -f "$mod_dir/Cargo.toml" ]; then
    if [ "$HAS_RUST" != "1" ]; then
      warn "skipping $name build (no rust toolchain)"
      log "    → $mod_dist (manifest only)"
      continue
    fi
    log "build $name (rust → wasm)"
    cargo_scratch="$SCRATCH_DIR/$name/target"
    ( cd "$mod_dir" && cargo build --target wasm32-unknown-unknown --release --target-dir "$cargo_scratch" ) >&2
    built="$(ls -1 "$cargo_scratch/wasm32-unknown-unknown/release"/*.wasm 2>/dev/null | head -n1 || true)"
    if [ -z "${built:-}" ]; then
      warn "no .wasm artifact produced for $name"
      log "    → $mod_dist (manifest only)"
      continue
    fi
    cp "$built" "$mod_dist/module.wasm"
    log "    → $mod_dist/module.wasm"
    if [ -d "$mod_dir/assets" ]; then
      cp -r "$mod_dir/assets" "$mod_dist/assets"
    fi
    continue
  fi

  # -------- C mod -------------------------------------------------------
  if [ -d "$mod_dir/src" ] && compgen -G "$mod_dir/src/*.c" >/dev/null; then
    if [ "$HAS_CLANG" != "1" ]; then
      warn "skipping $name build (no clang/wasm32)"
      log "    → $mod_dist (manifest only)"
      continue
    fi
    log "build $name (c → wasm)"

    ep="$(grep -oE 'entrypoint[[:space:]]*=[[:space:]]*"[^"]+"' "$manifest" \
          | sed -E 's/.*"([^"]+)".*/\1/' | head -n1)"
    if [ -z "${ep:-}" ]; then
      ep="${name//-/_}_register"
    fi

    # Cross-mod `extern` declarations are not allowed; pass
    # `--allow-undefined` defensively so a leftover extern shows up
    # as a real WASM import rather than a toolchain error.
    case "$name" in
      arena-ai)
        export_flags=(
          "-Wl,--export=${ep}"
          "-Wl,--export=com_example_arena_ai_tick"
          "-Wl,--export=com_example_arena_ai_get_ai_x"
          "-Wl,--export=com_example_arena_ai_reset_ai"
          "-Wl,--export=worker"
        )
        ;;
      arena-physics)
        export_flags=(
          "-Wl,--export=${ep}"
          "-Wl,--export=com_example_arena_physics_tick"
          "-Wl,--export=com_example_arena_physics_serve"
          "-Wl,--export=com_example_arena_physics_set_input_dx"
          "-Wl,--export=com_example_arena_physics_set_ai_x"
          "-Wl,--export=com_example_arena_physics_set_state"
        )
        ;;
      *)
        export_flags=(
          "-Wl,--export=${ep}"
          "-Wl,--export=tick"
          "-Wl,--export=worker"
        )
        ;;
    esac

    src_files=( "$mod_dir/src"/*.c )
    clang --target=wasm32-unknown-unknown \
          -O2 \
          -nostdlib \
          -Wl,--no-entry \
          -Wl,--allow-undefined \
          -Wl,--import-memory \
          -Wl,--import-table \
          -I"$C_BINDINGS_DIR" \
          "${export_flags[@]}" \
          -o "$mod_dist/module.wasm" \
          "${src_files[@]}" >&2
    log "    → $mod_dist/module.wasm"
    continue
  fi

  # -------- JS mod ------------------------------------------------------
  if [ -f "$mod_dir/module.js" ]; then
    cp "$mod_dir/module.js" "$mod_dist/module.js"
    if command -v node >/dev/null 2>&1; then
      if node --check "$mod_dist/module.js" >/dev/null 2>&1; then
        log "copy $name (js; node --check ok)"
      else
        warn "$name module.js failed node --check (copied anyway)"
      fi
    else
      log "copy $name (js)"
    fi
    log "    → $mod_dist/module.js"
    if [ -d "$mod_dir/assets" ]; then
      cp -r "$mod_dir/assets" "$mod_dist/assets"
    fi
    continue
  fi

  # -------- Pure-data (assets) mod -------------------------------------
  if [ -d "$mod_dir/assets" ]; then
    cp -r "$mod_dir/assets" "$mod_dist/assets"
    log "copy $name (assets/)"
    log "    → $mod_dist/assets/"
    continue
  fi

  warn "$name has nothing to build (no src/, no module.js, no assets/)"
done

log "all mods processed"
log "shippable runtime tree: $DIST_DIR"
