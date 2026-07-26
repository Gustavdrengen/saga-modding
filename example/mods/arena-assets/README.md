# `com.example.arena-assets`

A pure-data mod. No `tick` export — its job is to publish bytes
through `saga://` URIs:

| URI                                                                | File                       | Consumer                          |
| ------------------------------------------------------------------ | -------------------------- | --------------------------------- |
| `saga://com.example.arena-assets/palette.json`                     | `assets/palette.json`      | `arena-renderer` (browser paint) |
| `saga://com.example.arena-assets/dimensions.json`                 | `assets/dimensions.json`   | `arena-physics` (lazy on first tick) |

It declares no dependencies, so the Saga Launcher boots it at
the earliest possible dep level (alongside `arena-physics`,
which is also a dependency-free root). The Launcher invokes
`register_assets` exactly once on boot.
