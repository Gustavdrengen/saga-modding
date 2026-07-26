# `com.example.arena-renderer`

Pure-JS mod. Renders ball + paddles + score onto
`Saga.canvas` and captures keyboard input. Because it owns the
DOM, it also **owns the requestAnimationFrame loop** – it's the
mod that calls `Saga.runtime.fireEachFrame(dt)` once per RAF
tick to drive every other mod's per-frame `tick(dt)` export.

The renderer's `tick(dt)` is *also* registered – it paints the
canvas after the physics + AI `tick`s have updated state in
shared memory / unified WASM exports.

| Export                                | Owner    | Consumer                              |
| ------------------------------------- | -------- | ------------------------------------- |
| `arena_renderer_init` (entrypoint)    | renderer | saga:runtime engine once on boot     |
| `tick(dt)`                            | renderer | saga:runtime engine once per frame  |

## What's in `module.js`

- On init: install key listeners that pipe `←/→/A/D/Space/R/P`
  into `set_input_dx` and `set_state` exports on the physics
  mod, and into the renderer's local `state` (paused toggle).
- On RAF: compute `dt`, call `Saga.runtime.fireEachFrame(dt)`,
  then update `ctx.fillRect` / `ctx.arc` from the unified
  exports. Circle for ball, rectangles for the player (green)
  and AI (purple via the palette fetched from `arena-assets`).
