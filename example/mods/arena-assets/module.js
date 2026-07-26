// arena-assets — Saga mod. Pure-data bundle: ships a couple of JSON
// blobs and hands them out by name through `getAsset(name)` to any
// JS peer without going through `saga:assets`.

const MOD_ID = "com.example.arena-assets";

const ASSETS = {
  "palette.json":
    JSON.stringify({
      bg:      "rgba(5,9,16,0)",
      grid:    "rgba(255,255,255,0.07)",
      ball:    "#f8d57f",
      player:  "#61e2a4",
      ai:      "#c08bff",
      accent:  "#6cc4ff",
      muted:   "rgba(255,255,255,0.30)",
    }, null, 2),

  "dimensions.json":
    JSON.stringify({
      field_w:        800,
      field_h:        500,
      paddle_w:       100,
      paddle_h:       10,
      paddle_y_bot:   478,
      paddle_y_top:   22,
      ball_radius:    9,
      serve_velocity: 240,
      max_ai_speed:   320,
      ball_friction:  0.9995,
    }, null, 2),
};

export const imports = {};

export function com_example_arena_assets_register(_exports, _memory, _table) {
  void _exports; void _memory; void _table;
  // Phase 1 (registration): publish the JSON to a module-level
  // registry on globalThis so the orchestrator's saga_start loop can
  // pick it up asynchronously without going through `saga:assets`.
  if (typeof globalThis !== "undefined") {
    globalThis.__arenaAssets = globalThis.__arenaAssets || {};
    globalThis.__arenaAssets[MOD_ID] = ASSETS;
  }
  if (typeof console !== "undefined") {
    console.log("arena-assets: registered " + Object.keys(ASSETS).length + " asset(s)");
  }
  return 0;
}

// Pure-JS convenience accessor any peer mod can use without going
// through `saga:assets`. Returns a parsed object or `null` if missing.
export function getAsset(name) {
  const raw = (typeof globalThis !== "undefined"
    && globalThis.__arenaAssets
    && globalThis.__arenaAssets[MOD_ID]
    && globalThis.__arenaAssets[MOD_ID][name]) || null;
  if (!raw) return null;
  try { return JSON.parse(raw); } catch (_e) { return null; }
}
