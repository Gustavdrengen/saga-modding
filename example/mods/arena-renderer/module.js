// arena-renderer — Saga mod (pure JS).
//
// Doubles as the example's per-frame base game. Exports two top-level
// entrypoints:
//
//   * com_example_arena_renderer_register — Phase 1 (registration).
//     Non-blocking: stashes the merged WASM exports, locates a
//     canvas, and installs keyboard listeners. Returns 0.
//
//   * saga_start — Phase 2 (engine launch). Boots the
//     requestAnimationFrame loop and orchestrates per-frame work:
//     read physics state via merged exports, hand ball state to the C
//     AI, pipe the AI's paddle back into physics, advance physics,
//     then paint.

let g_wasmExports  = null;
let g_ctx          = null;
let g_canvas       = null;
let g_pExports     = null;
let g_aiExports    = null;
let g_rafId        = 0;
let g_prevMs       = 0;
let g_initialised  = false;
let g_palette      = defaultPalette();
let g_keys         = { left: false, right: false };

// Imports map: host functions handed to the merged WASM namespace so a
// peer WASM mod can call back into JS. Peer WASM mods in this example
// never need JS, so the map is intentionally empty.
export const imports = {};

// =============================================================================
// Phase 1: Registration. Non-blocking. Returns 0.
// =============================================================================
export function com_example_arena_renderer_register(wasmExports, memory, table) {
  // Re-invocation guard. A real Saga launcher calls register exactly
  // once, but the saga_start fallback below defensively re-runs it,
  // and a hosted harness may also double-invoke; without this short-
  // circuit bindKeys() would attach duplicate event listeners on
  // window and leak them across runs.
  if (g_initialised) return 0;

  // The engine hands us the merged export namespace, the unified linear
  // memory, and the unified indirect-call table. We stash the first for
  // the per-frame work in saga_start; the latter two are kept for
  // completeness even though this mod does not register new host
  // bindings during Phase 1.
  void memory; void table;
  g_wasmExports = wasmExports || null;

  if (typeof document !== "undefined") {
    g_canvas = document.querySelector("canvas");
    if (!g_canvas) {
      g_canvas = document.createElement("canvas");
      g_canvas.width  = 800;
      g_canvas.height = 500;
      if (document.body) document.body.appendChild(g_canvas);
    }
    g_ctx = g_canvas.getContext("2d");
  }

  const e = g_wasmExports || {};
  g_pExports = {
    get_ball_x:          e.com_example_arena_physics_get_ball_x,
    get_ball_y:          e.com_example_arena_physics_get_ball_y,
    get_ball_vx:         e.com_example_arena_physics_get_ball_vx,
    get_ball_vy:         e.com_example_arena_physics_get_ball_vy,
    get_ball_r:          e.com_example_arena_physics_get_ball_r,
    get_player_paddle_x: e.com_example_arena_physics_get_player_paddle_x,
    get_player_score:    e.com_example_arena_physics_get_player_score,
    get_ai_paddle_x:     e.com_example_arena_physics_get_ai_paddle_x,
    get_ai_score:        e.com_example_arena_physics_get_ai_score,
    get_state:           e.com_example_arena_physics_get_state,
    set_input_dx:        e.com_example_arena_physics_set_input_dx,
    set_ai_x:            e.com_example_arena_physics_set_ai_x,
    set_state:           e.com_example_arena_physics_set_state,
    serve:               e.com_example_arena_physics_serve,
    tick:                e.com_example_arena_physics_tick,
  };
  g_aiExports = {
    tick:     e.com_example_arena_ai_tick,
    get_ai_x: e.com_example_arena_ai_get_ai_x,
    reset_ai: e.com_example_arena_ai_reset_ai,
  };

  bindKeys();
  maybeFetchPalette();
  console.log("arena-renderer registered");

  if (!g_pExports || !g_pExports.get_ball_x || !g_pExports.tick) {
    // No physics exports available; the engine may retry registration
    // after Binaryen merge, or this mod simply won't paint.
    return 0;
  }

  g_initialised = true;
  return 0;
}

// =============================================================================
// Phase 2: saga_start. Called once by the engine's launch pass after every
// mod's Phase 1 entrypoint has returned 0. Starts the RAF loop.
// =============================================================================
export function saga_start() {
  if (!g_initialised) {
    // A real engine guarantees registration completes before launch,
    // but a hosted harness may invoke saga_start before register
    // (or skip register entirely). Re-run registration defensively.
    com_example_arena_renderer_register({}, null, null);
  }
  if (typeof requestAnimationFrame === "undefined") return 0;

  g_prevMs = nowMs();
  g_rafId  = requestAnimationFrame(frameLoop);
  return 0;
}

// =============================================================================
// Keyboard input → physics set_input_dx / set_state.
// Bound on `window` so arrow / WASD keys fire regardless of canvas focus.
// =============================================================================
function bindKeys() {
  if (typeof window === "undefined") return;

  window.addEventListener("keydown", (e) => {
    if (e.repeat) return;
    switch (e.key) {
      case "ArrowLeft":  case "a": case "A":
        g_keys.left = true;  pushInputAxis(); e.preventDefault(); break;
      case "ArrowRight": case "d": case "D":
        g_keys.right = true; pushInputAxis(); e.preventDefault(); break;
      case " ":
        /* SPACE → serve. The renderer passes a small downward
         * velocity so the ball actually moves; physics can't infer
         * a launch from STATE alone. */
        if (g_pExports && g_pExports.serve) {
          g_pExports.serve(60.0, 240.0);
        } else if (g_pExports && g_pExports.set_state) {
          g_pExports.set_state(1);
        }
        if (g_aiExports && g_aiExports.reset_ai) g_aiExports.reset_ai();
        e.preventDefault();
        break;
      case "r": case "R":
        if (g_pExports && g_pExports.set_state) g_pExports.set_state(0);
        if (g_aiExports && g_aiExports.reset_ai) g_aiExports.reset_ai();
        break;
      case "p": case "P":
        if (!g_pExports || !g_pExports.get_state || !g_pExports.set_state) return;
        const cur = g_pExports.get_state();
        g_pExports.set_state(cur === 2 ? 1 : 2);
        break;
    }
  });

  window.addEventListener("keyup", (e) => {
    switch (e.key) {
      case "ArrowLeft":  case "a": case "A":
        g_keys.left = false;  pushInputAxis(); break;
      case "ArrowRight": case "d": case "D":
        g_keys.right = false; pushInputAxis(); break;
    }
  });
}

function pushInputAxis() {
  let dx = 0;
  if (g_keys.left)  dx -= 1;
  if (g_keys.right) dx += 1;
  if (g_pExports && g_pExports.set_input_dx) g_pExports.set_input_dx(dx);
}

// =============================================================================
// Per-frame orchestrator. Reads ball state from merged exports, ships it
// to the C AI as plain arguments, pumps the AI paddle back into physics,
// advances physics, then paints.
// =============================================================================
function frameLoop(now) {
  g_rafId = requestAnimationFrame(frameLoop);

  const dtMs = now - g_prevMs;
  g_prevMs  = now;
  const dt   = Math.min(0.05, dtMs / 1000);

  try { orchestratorTick(dt); }
  catch (e) { console.warn("arena-renderer frame error:", e && e.message); }
}

function orchestratorTick(dt) {
  if (!g_initialised || !g_pExports || !g_pExports.tick) return;

  const get = (k, fb) => (g_pExports[k] ? g_pExports[k]() : fb);

  const bx  = get("get_ball_x",  400);
  const by  = get("get_ball_y",  250);
  const bvx = get("get_ball_vx", 0);
  const bvy = get("get_ball_vy", 0);

  if (g_aiExports && g_aiExports.tick) {
    g_aiExports.tick(bx, by, bvx, bvy, dt);
  }

  const ai_x = (g_aiExports && g_aiExports.get_ai_x)
    ? g_aiExports.get_ai_x()
    : 400;
  if (g_pExports.set_ai_x) g_pExports.set_ai_x(ai_x);

  g_pExports.tick(dt);

  paint();
}

// =============================================================================
// Per-frame paint. Pure DOM draw.
// =============================================================================
function paint() {
  if (!g_ctx) return;
  const W = g_canvas ? g_canvas.width  : 800;
  const H = g_canvas ? g_canvas.height : 500;

  const gbx  = g_pExports.get_ball_x          ? g_pExports.get_ball_x()          : 400;
  const gby  = g_pExports.get_ball_y          ? g_pExports.get_ball_y()          : 250;
  const gbr  = g_pExports.get_ball_r          ? g_pExports.get_ball_r()          : 9;
  const gpx  = g_pExports.get_player_paddle_x ? g_pExports.get_player_paddle_x() : 400;
  const gax  = g_pExports.get_ai_paddle_x     ? g_pExports.get_ai_paddle_x()     : 400;
  const gpsc = g_pExports.get_player_score    ? g_pExports.get_player_score()    : 0;
  const gasc = g_pExports.get_ai_score        ? g_pExports.get_ai_score()        : 0;
  const gst  = g_pExports.get_state           ? g_pExports.get_state()           : 0;

  g_ctx.fillStyle = g_palette.bg;
  g_ctx.fillRect(0, 0, W, H);

  g_ctx.strokeStyle = g_palette.grid;
  g_ctx.setLineDash([6, 8]);
  g_ctx.beginPath();
  g_ctx.moveTo(0, H / 2);
  g_ctx.lineTo(W, H / 2);
  g_ctx.stroke();
  g_ctx.setLineDash([]);

  drawRect(g_ctx, gpx - 50, H - 22, 100, 10, 5, g_palette.player);
  drawRect(g_ctx, gax - 50, 12,    100, 10, 5, g_palette.ai);

  g_ctx.fillStyle = g_palette.ball;
  g_ctx.beginPath();
  g_ctx.arc(gbx, gby, gbr, 0, Math.PI * 2);
  g_ctx.fill();

  g_ctx.fillStyle = "rgba(0,0,0,0.45)";
  g_ctx.fillRect(0, 0, W, 24);
  g_ctx.fillStyle = g_palette.player;
  g_ctx.font = "bold 14px ui-monospace, Menlo, monospace";
  g_ctx.textAlign = "left";
  g_ctx.fillText("PLAYER " + (gpsc | 0), 10, 18);
  g_ctx.fillStyle = g_palette.ai;
  g_ctx.textAlign = "right";
  g_ctx.fillText("AI " + (gasc | 0), W - 10, 18);
  g_ctx.textAlign = "start";

  if (gst === 0) drawBanner(g_ctx, W, H, g_palette.accent, "PRESS SPACE TO SERVE");
  if (gst === 2) drawBanner(g_ctx, W, H, g_palette.accent, "PAUSED — press P to resume");
}

function drawBanner(c, W, H, color, text) {
  c.fillStyle = "rgba(0,0,0,0.55)";
  c.fillRect(0, 0, W, H);
  c.fillStyle = color;
  c.font = "bold 18px ui-monospace, Menlo, monospace";
  c.textAlign = "center";
  c.fillText(text, W / 2, H / 2);
  c.textAlign = "start";
}

function drawRect(c, x, y, w, h, r, fill) {
  c.fillStyle = fill;
  c.beginPath();
  c.moveTo(x + r, y);
  c.lineTo(x + w - r, y);
  c.quadraticCurveTo(x + w, y, x + w, y + r);
  c.lineTo(x + w, y + h - r);
  c.quadraticCurveTo(x + w, y + h, x + w - r, y + h);
  c.lineTo(x + r, y + h);
  c.quadraticCurveTo(x, y + h, x, y + h - r);
  c.lineTo(x, y + r);
  c.quadraticCurveTo(x, y, x + r, y);
  c.closePath();
  c.fill();
}

// =============================================================================
// Palette fetch through the saga:// asset scheme (MOD_SPEC.md §3.3, §4.1).
// The Saga Launcher (or a hosted Service Worker mocking its asset layer)
// resolves `saga://com.example.arena-assets/palette.json` to the bytes
// of that file. If the fetch rejects we keep defaultPalette() so the
// game still renders.
// =============================================================================
function maybeFetchPalette() {
  if (typeof fetch === "undefined") return;
  fetch("saga://com.example.arena-assets/palette.json")
    .then((r) => (r && typeof r.text === "function") ? r.text() : null)
    .then((txt) => {
      if (!txt) return;
      try { g_palette = Object.assign({}, defaultPalette(), JSON.parse(txt)); }
      catch (_e) { /* fall back to defaults */ }
    })
    .catch(() => { /* fall back to defaults */ });
}

function defaultPalette() {
  return {
    bg:     "rgba(5,9,16,0)",
    grid:   "rgba(255,255,255,0.07)",
    ball:   "#f8d57f",
    player: "#61e2a4",
    ai:     "#c08bff",
    accent: "#6cc4ff",
    muted:  "rgba(255,255,255,0.30)",
  };
}

function nowMs() {
  if (typeof performance !== "undefined" && performance.now) return performance.now();
  return Date.now ? Date.now() : 0;
}
