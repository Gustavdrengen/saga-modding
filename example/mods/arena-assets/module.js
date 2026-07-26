// example/mods/arena-assets/module.js
//
// Pure-data mod. On boot, publishes two JSON blobs under
// `saga://com.example.arena-assets/<name>.json` so other mods
// (renderer / physics / ai) can fetch them at runtime through
// the Saga Asset Protocol.

const MOD_ID = "com.example.arena-assets";

// Pure strings inlined so the mod needs no `fetch()` at boot. In a real
// Saga deployment the Launcher would also auto-discover the files in
// `assets/` and register them on disk; this module.js keeps the demo
// self-contained.
const ASSETS = {
  "palette.json":
    JSON.stringify({
      bg:      "rgba(5,9,16,0)",
      grid:    "rgba(255,255,255,0.07)",
      ball:    "#f8d57f",
      player:  "#61e2a4",
      ai:      "#c08bff",
      accent:  "#6cc4ff",
      muted:   "rgba(255,255,255,0.30)"
    }, null, 2),

  "dimensions.json":
    JSON.stringify({
      field_w:        800,
      field_h:        500,
      paddle_w:       100,
      paddle_h:       10,
      paddle_y_bot:   478,   // player's paddle baseline y
      paddle_y_top:   22,    // AI's paddle baseline y
      ball_radius:    9,
      serve_velocity: 240,
      max_ai_speed:   320,
      ball_friction:  0.9995
    }, null, 2),
};

// Mod entrypoint declared in `manifest.toml` as `register_assets`.
// Returns 1 on success, 0 if the host surface is missing (then the
// launcher treats this as a soft-skip).
export function register_assets() {
  const Saga = (typeof globalThis !== "undefined") ? globalThis.Saga : undefined;
  if (!Saga || !Saga.assets || typeof Saga.assets.register !== "function") {
    return 0;
  }
  let count = 0;
  for (const [path, str] of Object.entries(ASSETS)) {
    Saga.assets.register("saga://" + MOD_ID + "/" + path, new TextEncoder().encode(str));
    count++;
  }
  if (Saga.host && typeof Saga.host.log === "function") {
    Saga.host.log("arena-assets: registered " + count + " asset(s) under " + MOD_ID);
  }
  return 1;
}
