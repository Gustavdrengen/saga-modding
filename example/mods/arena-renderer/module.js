// example/mods/arena-renderer/module.js
//
// Pure-JS orchestrator mod for the Saga example game. Per MOD_SPEC §8.5,
// this mod is the single per-frame dispatcher: it reads ball state from
// `arena-physics` via the merged `Saga.wasmExports`, calls `arena-ai.tick`
// with those values as arguments, pipes the AI's predicted paddle position
// back into physics, advances physics, then paints.
//
// All four mods compose into a game because the renderer orchestrates
// the cross-mod data flow each animation frame.
//
// Compiled: NO. `example/build.sh` only verifies `node --check` syntax.
//
// Keyboard input is bound to `window` (not the canvas), so arrow / WASD
// keys fire whether or not the page-host gave the <canvas> tabindex.

const MOD_ID = "com.example.arena-renderer";

let g_Saga        = null;
let g_ctx         = null;
let g_canvas      = null;
let g_pExports    = null;     // arena-physics merged exports (or null if not merged)
let g_aiExports   = null;     // arena-ai merged exports (or null)
let g_rafId       = 0;
let g_prevMs      = 0;
let g_initialised = false;

let g_keys = { left: false, right: false };
let g_palette = {
  bg:      "rgba(5,9,16,0)",
  grid:    "rgba(255,255,255,0.07)",
  ball:    "#f8d57f",
  player:  "#61e2a4",
  ai:      "#c08bff",
  accent:  "#6cc4ff",
  muted:   "rgba(255,255,255,0.30)"
};

// -----------------------------------------------------------------------------
// §6 entrypoint declared in `manifest.toml` as `arena_renderer_init`.
// -----------------------------------------------------------------------------
export function arena_renderer_init() {
  const Saga = (typeof globalThis !== "undefined") ? globalThis.Saga : undefined;
  if (!Saga) return 0;
  g_Saga   = Saga;
  g_ctx    = Saga.ctx || (Saga.canvas ? Saga.canvas.getContext("2d") : null);
  g_canvas = Saga.canvas || null;

  // Long-prefixed per §8.5. If the engine hasn't merged wasmExports yet,
  // gracefully degrade (the launcher will re-call entrypoint, or this mod
  // will simply not paint).
  const e = Saga.wasmExports || {};
  g_pExports  = {
    get_ball_x:          e["com_example_arena_physics_get_ball_x"],
    get_ball_y:          e["com_example_arena_physics_get_ball_y"],
    get_ball_vx:         e["com_example_arena_physics_get_ball_vx"],
    get_ball_vy:         e["com_example_arena_physics_get_ball_vy"],
    get_ball_r:          e["com_example_arena_physics_get_ball_r"],
    get_player_paddle_x: e["com_example_arena_physics_get_player_paddle_x"],
    get_player_score:    e["com_example_arena_physics_get_player_score"],
    get_ai_paddle_x:     e["com_example_arena_physics_get_ai_paddle_x"],
    get_ai_score:        e["com_example_arena_physics_get_ai_score"],
    get_state:           e["com_example_arena_physics_get_state"],
    set_input_dx:        e["com_example_arena_physics_set_input_dx"],
    set_ai_x:            e["com_example_arena_physics_set_ai_x"],
    set_state:           e["com_example_arena_physics_set_state"],
    serve:               e["com_example_arena_physics_serve"],
    tick:                e["com_example_arena_physics_tick"],
  };
  g_aiExports = {
    tick:    e["com_example_arena_ai_tick"],
    get_ai_x:e["com_example_arena_ai_get_ai_x"],
    reset_ai:e["com_example_arena_ai_reset_ai"],
  };

  if (g_ctx && (!g_pExports.get_ball_x || !g_pExports.tick)) {
    // No physics exports — we can't drive the game. Stay silent.
    return 0;
  }

  bindKeys();

  // Fetch the palette from arena-assets (async, optional). Fall back to
  // the inlined defaults if absent so the user never sees a black canvas.
  if (Saga.assets && typeof Saga.assets.fetchBuffer === "function") {
    Saga.assets
      .fetchBuffer("saga://com.example.arena-assets/palette.json")
      .then((bytes) => {
        try { g_palette = Object.assign(g_palette, JSON.parse(new TextDecoder().decode(bytes))); }
        catch (_e) { /* keep defaults */ }
      })
      .catch(() => { /* keep defaults */ });
  }

  if (Saga.host && typeof Saga.host.log === "function") {
    Saga.host.log("arena-renderer: enter " + MOD_ID);
  }
  g_initialised = true;
  return 1;
}

// -----------------------------------------------------------------------------
// Keyboard input → physics.set_input_dx / set_state. We bind to `window`
// because the <canvas> typically has no tabindex; window-level capture
// works in fullscreen and embedded layouts alike.
// -----------------------------------------------------------------------------
function bindKeys() {
  const target = (typeof window !== "undefined") ? window : null;
  if (!target) return;

  target.addEventListener("keydown", (e) => {
    if (e.repeat) return;
    switch (e.key) {
      case "ArrowLeft":  case "a": case "A":
        g_keys.left = true;  pushInputAxis(); e.preventDefault(); break;
      case "ArrowRight": case "d": case "D":
        g_keys.right = true; pushInputAxis(); e.preventDefault(); break;
      case " ":
        /* SPACE → serve. We pass a small downward velocity so the ball
         * actually moves: physics can't infer a launch from STATE alone. */
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

  target.addEventListener("keyup", (e) => {
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

// -----------------------------------------------------------------------------
// Per-frame orchestrator loop. Runs exclusively on the requestAnimationFrame
// clock we started in arena_renderer_init; we never double-tick because we
// do NOT register a `tick` callback anywhere else.
// -----------------------------------------------------------------------------
function frameLoop(nowMs) {
  g_rafId = requestAnimationFrame(frameLoop);

  const dtMs = nowMs - g_prevMs;
  g_prevMs  = nowMs;
  const dt   = Math.min(0.05, dtMs / 1000);

  try {
    orchestratorTick(dt);
  } catch (e) {
    if (g_Saga && g_Saga.host && typeof g_Saga.host.log === "function") {
      g_Saga.host.log("arena-renderer frame error: " + (e && e.message));
    }
  }
}

function orchestratorTick(dt) {
  if (!g_initialised || !g_pExports || !g_pExports.tick) return;

  // 1. Read physics ball state via merged exports (uses getter exports).
  const get = (k, fb) => (g_pExports[k] ? g_pExports[k]() : fb);
  const bx  = get("get_ball_x",  400);
  const by  = get("get_ball_y",  250);
  const bvx = get("get_ball_vx", 0);
  const bvy = get("get_ball_vy", 0);

  // 2. Push ball state into the C mod as plain arguments.
  if (g_aiExports && g_aiExports.tick) {
    g_aiExports.tick(bx, by, bvx, bvy, dt);
  }

  // 3. Read the AI's response and pipe it back into physics.
  const ai_x = (g_aiExports && g_aiExports.get_ai_x)
    ? g_aiExports.get_ai_x()
    : 400;
  if (g_pExports.set_ai_x) g_pExports.set_ai_x(ai_x);

  // 4. Advance physics. INPUT_DX was already set from key listeners.
  g_pExports.tick(dt);

  // 5. Paint.
  paint(dt);
}

// -----------------------------------------------------------------------------
// Per-frame paint. Pure DOM draw. Reads the latest physics state (which
// the orchestrator just advanced) and the latest ai paddlex (also just
// pumped into physics).
// -----------------------------------------------------------------------------
function paint(/* dt */) {
  if (!g_ctx) return;
  const W = g_canvas ? g_canvas.width  : 800;
  const H = g_canvas ? g_canvas.height : 500;

  const gbx  = g_pExports.get_ball_x          ? g_pExports.get_ball_x()             : 400;
  const gby  = g_pExports.get_ball_y          ? g_pExports.get_ball_y()             : 250;
  const gbr  = g_pExports.get_ball_r          ? g_pExports.get_ball_r()             : 9;
  const gpx  = g_pExports.get_player_paddle_x ? g_pExports.get_player_paddle_x()    : 400;
  const gax  = g_pExports.get_ai_paddle_x     ? g_pExports.get_ai_paddle_x()        : 400;
  const gpsc = g_pExports.get_player_score    ? g_pExports.get_player_score()       : 0;
  const gasc = g_pExports.get_ai_score        ? g_pExports.get_ai_score()           : 0;
  const gst  = g_pExports.get_state           ? g_pExports.get_state()              : 0;

  // Bg.
  g_ctx.fillStyle = g_palette.bg;
  g_ctx.fillRect(0, 0, W, H);

  // Centre dashed line.
  g_ctx.strokeStyle = g_palette.grid;
  g_ctx.setLineDash([6, 8]);
  g_ctx.beginPath();
  g_ctx.moveTo(0, H / 2);
  g_ctx.lineTo(W, H / 2);
  g_ctx.stroke();
  g_ctx.setLineDash([]);

  // Player paddle (bottom).
  drawRect(g_ctx, gpx - 50, H - 22, 100, 10, 5, g_palette.player);
  // AI paddle (top).
  drawRect(g_ctx, gax - 50, 12, 100, 10, 5, g_palette.ai);

  // Ball.
  g_ctx.fillStyle = g_palette.ball;
  g_ctx.beginPath();
  g_ctx.arc(gbx, gby, gbr, 0, Math.PI * 2);
  g_ctx.fill();

  // Score chip.
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
