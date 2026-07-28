# `com.example.arena-assets`

Pure-data mod. The on-disk layout is exactly what a Saga Launcher
needs to load it:

```
com.example.arena-assets/
├── manifest.toml
├── README.md
└── assets/
    ├── palette.json
    └── dimensions.json
```

There is no `module.js` and no `entrypoint` field in
`manifest.toml` — there is nothing to register. The Saga
Launcher auto-serves every file in `assets/` under the
convention `saga://com.example.arena-assets/<file>` (see
`MOD_SPEC.md` §3.3 and §4.1). Peer consumers fetch through
that URI only:

| Consumer path         | Mechanism                                                       |
| --------------------- | --------------------------------------------------------------- |
| WASM peer mod         | `saga_asset_open` host import → bytes in linear memory          |
| JS peer mod           | `fetch("saga://com.example.arena-assets/<file>")` → text()      |

## Assets

| Asset name         | File                       | Consumer                                |
| ------------------ | -------------------------- | --------------------------------------- |
| `palette.json`     | `assets/palette.json`      | `arena-renderer` (browser paint)        |
| `dimensions.json`  | `assets/dimensions.json`   | Tuning constants for AI/physics         |

The launcher schedules a pure-data mod earliest in dependency
order so consumers can fetch its assets during their own Phase 1.
