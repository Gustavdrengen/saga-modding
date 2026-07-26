# `com.example.arena-assets`

A pure-data mod. The launcher auto-serves every JSON file in `assets/`
under the convention `saga://com.example.arena-assets/<name>`; this
module.js additionally publishes the same JSON to a module-level
registry so a pure-JS peer mod can read it directly without going
through `saga:assets`.

| Asset name         | File                       | Consumer                          |
| ------------------ | -------------------------- | --------------------------------- |
| `palette.json`     | `assets/palette.json`      | `arena-renderer` (browser paint)  |
| `dimensions.json`  | `assets/dimensions.json`   | Tuning constants for AI/physics   |

The registration entrypoint is `com_example_arena_assets_register`;
it is dependency-free, so the launcher schedules it earliest.
