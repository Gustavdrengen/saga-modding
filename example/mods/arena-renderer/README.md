# `com.example.arena-renderer`

Pure-JS mod. Doubles as a renderer and as the example's pragmatic
"`saga_start` base game": the launcher invokes this mod twice —
once during the Phase 1 registration pass, once during the Phase 2
launch pass.

| Export name                                       | Phase | Caller                |
| ------------------------------------------------- | ----- | --------------------- |
| `com_example_arena_renderer_register`             | 1     | Saga launcher once    |
| `saga_start`                                      | 2     | Saga launcher once    |

## What each phase does

- **Phase 1 (`register`)**: stash the final linked WASM exports the engine
  passed in, locate a canvas (or create one), install keyboard
  listeners, kick off an async palette fetch through the saga://
  asset scheme (`fetch("saga://com.example.arena-assets/palette.json")`).
  Return `0` immediately without starting the frame loop.
- **Phase 2 (`saga_start`)**: boot the `requestAnimationFrame` loop.
  Each frame: read the live ball state from `arena-physics` via the
  merged exports, hand those values to `arena-ai` as plain
  `tick(bx, by, bvx, bvy, dt)` arguments, pipe the AI's predicted
  paddle position back into physics, advance physics, then paint.

Because the orchestrator pattern is private to this mod, every peer
mod's Phase 1 is genuinely non-blocking, regardless of how complex
the runtime frame pipeline turns out to be.

## Asset access

The palette (and any other asset this mod wants) is loaded by
fetching the canonical `saga://` URI the Saga Launcher's asset
layer exposes. The launcher resolves the URI to bytes; this mod
just `JSON.parse`s the response and overlays the colours onto a
hard-coded `defaultPalette()` so a missing/empty fetch still
renders something.
