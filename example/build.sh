#!/usr/bin/env bash
# build.sh — compile every example mod into a shippable Saga package.
#
# This script is a producer-side build only. It MUST NOT merge, link,
# optimize, instantiate, or execute WebAssembly modules. The Saga launcher
# consumes each module.a archive and performs final linking/optimization.
#
# Per-mod dist layout:
#   dist/<mod-name>/
#       manifest.toml
#       README.md                    (if present)
#       module.a                     (WebAssembly mod link input)
#       module.js                    (if present)
#       assets/...                   (if present)

set -euo pipefail

cd "$(dirname "$0")"
ROOT="$(pwd -P)"
MODS_DIR="$ROOT/mods"
DIST_DIR="$ROOT/dist"
SCRATCH_DIR="$ROOT/build"
C_BINDINGS_DIR="$(cd "$ROOT/.." && pwd -P)/c_bindings"

[ -d "$MODS_DIR" ] || { echo "missing mods directory: $MODS_DIR" >&2; exit 2; }
[ -d "$C_BINDINGS_DIR" ] || { echo "missing c_bindings directory: $C_BINDINGS_DIR" >&2; exit 2; }

cleanup() {
  rm -rf "$SCRATCH_DIR"
}
trap cleanup EXIT INT TERM

rm -rf "$DIST_DIR" "$SCRATCH_DIR"
mkdir -p "$DIST_DIR" "$SCRATCH_DIR"

have() { command -v "$1" >/dev/null 2>&1; }

if ! have ar; then
  echo "ar is required to package WebAssembly objects as module.a" >&2
  exit 2
fi
if ! have wasm-objdump; then
  echo "wasm-objdump is required to validate module.a payload members" >&2
  exit 2
fi

HAS_RUST=0
if have rustc && have cargo; then
  HAS_RUST=1
fi

HAS_CLANG=0
if have clang && clang --target=wasm32-unknown-unknown --print-target-triple >/dev/null 2>&1; then
  HAS_CLANG=1
fi

if [ "$HAS_RUST" = 1 ] && ! rustup target list --installed 2>/dev/null | grep -q '^wasm32-unknown-unknown$'; then
  echo "wasm32-unknown-unknown Rust target is required" >&2
  exit 2
fi

[ "$HAS_RUST" = 1 ] || { echo "rustc and cargo are required to build this example" >&2; exit 2; }
[ "$HAS_CLANG" = 1 ] || { echo "clang with the wasm32-unknown-unknown target is required to build this example" >&2; exit 2; }

log() { printf '==> %s\n' "$*"; }
warn() { printf 'warning: %s\n' "$*" >&2; }

validate_archive() {
  local archive="$1"
  local inspect_dir="$2"
  mkdir -p "$inspect_dir"
  ( cd "$inspect_dir" && ar x "$archive" )
  local member
  local found=0
  while IFS= read -r -d '' member; do
    found=1
    wasm-objdump -h "$member" | grep -q '"linking"' \
      || { echo "archive member lacks Wasm linking metadata: $member" >&2; return 1; }
  done < <(find "$inspect_dir" -type f -print0)
  [ "$found" = 1 ] || { echo "archive contains no Wasm object payloads: $archive" >&2; return 1; }
}

shopt -s nullglob

for mod_dir in "$MODS_DIR"/*/; do
  name="$(basename "$mod_dir")"
  manifest="$mod_dir/manifest.toml"
  [ -f "$manifest" ] || { warn "skipping $name: missing manifest.toml"; continue; }

  mod_dist="$DIST_DIR/$name"
  mkdir -p "$mod_dist"
  cp "$manifest" "$mod_dist/manifest.toml"
  [ -f "$mod_dir/README.md" ] && cp "$mod_dir/README.md" "$mod_dist/README.md"

  # -------- Rust mod: Cargo staticlib -> module.a ----------------------
  if [ -f "$mod_dir/Cargo.toml" ]; then
    [ "$HAS_RUST" = 1 ] || { warn "skipping $name: Rust toolchain unavailable"; continue; }
    log "build $name (Rust relocatable archive)"
    cargo_target="$SCRATCH_DIR/$name/target"
    ( cd "$mod_dir" && cargo build --target wasm32-unknown-unknown --release --target-dir "$cargo_target" ) >&2
    built="$cargo_target/wasm32-unknown-unknown/release/lib${name//-/_}.a"
    [ -f "$built" ] || { echo "Rust archive missing: $built" >&2; exit 1; }
    cp "$built" "$mod_dist/module.a"
    [ -s "$mod_dist/module.a" ] || { echo "empty Rust archive: $mod_dist/module.a" >&2; exit 1; }
    validate_archive "$mod_dist/module.a" "$SCRATCH_DIR/$name/archive-check"
    log "    -> $mod_dist/module.a"
    [ -d "$mod_dir/assets" ] && cp -r "$mod_dir/assets" "$mod_dist/assets"
    continue
  fi

  # -------- C mod: clang object -> module.a -----------------------------
  if [ -d "$mod_dir/src" ] && compgen -G "$mod_dir/src/*.c" >/dev/null; then
    [ "$HAS_CLANG" = 1 ] || { warn "skipping $name: clang wasm32 target unavailable"; continue; }
    log "build $name (C relocatable archive)"
    c_scratch="$SCRATCH_DIR/$name"
    mkdir -p "$c_scratch"
    src_files=( "$mod_dir/src"/*.c )
    object_files=()
    for src in "${src_files[@]}"; do
      stem="$(basename "$src" .c)"
      object="$c_scratch/$stem.o"
      clang --target=wasm32-unknown-unknown \
            -O2 \
            -ffunction-sections \
            -fdata-sections \
            -I"$C_BINDINGS_DIR" \
            -c "$src" \
            -o "$object"
      object_files+=("$object")
    done
    ar rcs "$mod_dist/module.a" "${object_files[@]}"
    [ -s "$mod_dist/module.a" ] || { echo "empty C archive: $mod_dist/module.a" >&2; exit 1; }
    validate_archive "$mod_dist/module.a" "$SCRATCH_DIR/$name/archive-check"
    log "    -> $mod_dist/module.a"
    continue
  fi

  # -------- JS mod ------------------------------------------------------
  if [ -f "$mod_dir/module.js" ]; then
    cp "$mod_dir/module.js" "$mod_dist/module.js"
    if have node; then
      node --check "$mod_dist/module.js"
    fi
    log "copy $name (JavaScript)"
    [ -d "$mod_dir/assets" ] && cp -r "$mod_dir/assets" "$mod_dist/assets"
    continue
  fi

  # -------- Pure-data mod ----------------------------------------------
  if [ -d "$mod_dir/assets" ]; then
    cp -r "$mod_dir/assets" "$mod_dist/assets"
    log "copy $name (assets)"
    continue
  fi

  warn "$name has no module.a, module.js, or assets directory"
done

log "all mods processed"
log "shippable mod tree: $DIST_DIR"
