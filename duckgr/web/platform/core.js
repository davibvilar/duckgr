// Bridge to the DuckGR simulation core, written in Rust
// (crates/duckgr-sim) and compiled to WebAssembly (web/platform/duckgr.wasm).
//
// The core doesn't know that forests, furnaces or pollution exist: all
// content comes from the world package (see web/games/). It is
// deterministic: same package + same configuration + same commands = same
// hash.
//
// The state lives inside the wasm module; the S object the interface gets
// is a window into it: it reads the usual fields (S.tick, S.cov, S.jog,
// S.inv...), but it only changes through commands and steps, never by
// assignment. The field names are the save format's, so they stay as the
// first version wrote them.

export const TPS = 30; // ticks per second
// the same limits as the core (crates/duckgr-sim/src/player.rs)
export const QUEUE_MAX = 50; // orders in the queue
export const TIMES_MAX = 9999; // repetitions of one order

const W = await load();

async function load() {
  const url = new URL("./duckgr.wasm", import.meta.url);
  let bytes;
  if (typeof process !== "undefined" && process.versions?.node) {
    const { readFile } = await import("node:fs/promises");
    bytes = await readFile(url).catch(() => {
      throw new Error("web/platform/duckgr.wasm is missing. Run `npm run wasm` to build the core.");
    });
  } else {
    const r = await fetch(url);
    if (!r.ok) throw new Error(`Could not download the game core (${r.status}).`);
    bytes = await r.arrayBuffer();
  }
  const { instance } = await WebAssembly.instantiate(bytes, {});
  return instance.exports;
}

// ---------- talking to the module: JSON text in, JSON text out
const encoder = new TextEncoder(), decoder = new TextDecoder();
function withText(txt, f) {
  const b = encoder.encode(txt), p = W.alloc(b.length);
  new Uint8Array(W.memory.buffer, p, b.length).set(b);
  try { return f(p, b.length); } finally { W.dealloc(p, b.length); }
}
const output = () => decoder.decode(new Uint8Array(W.memory.buffer, W.out_ptr(), W.out_len()));
function create(f, data) {
  const h = withText(JSON.stringify(data), f);
  if (h < 0) throw new Error(output());
  return h;
}

// ---------- package
const handles = new WeakMap(); // compiled package → its number inside the module

// Checks the package's references and converts seconds to ticks. Returns
// the converted data (the interface takes names, colors and descriptions
// from here).
export function compile(pkg) {
  const h = create(W.package_compile, pkg);
  const C = JSON.parse(output());
  handles.set(C, h);
  return C;
}
function handleOf(C) {
  const h = handles.get(C);
  if (h === undefined) throw new Error("Use the package returned by compile().");
  return h;
}

// ---------- state
const release = new FinalizationRegistry(h => W.world_free(h));
const FIELDS = ["rng", "L", "A", "CL", "CA", "sub", "dep", "depQtd", "cov", "cres", "occ", "ents", "prox",
  "inv", "prod", "campos", "techs", "era", "liberados", "log", "ultimoDano", "missao", "jog"];

class State {
  constructor(h) {
    this._h = h;
    this._view = null;
    this.events = []; // notices for the interface, outside the hash
    release.register(this, h, this);
  }
  get tick() { return W.world_tick(this._h); }
  // the state in JSON, kept until the next step or command
  view() { return (this._view ||= this._op({ op: "state" })); }
  _op(op) {
    const r = withText(JSON.stringify(op), (p, n) => W.world_op(this._h, p, n));
    const txt = output();
    if (r < 0) throw new Error(txt);
    return JSON.parse(txt);
  }
  _changed(events = 1) {
    this._view = null;
    if (events) this.events.push(...this._op({ op: "events" }));
  }
  _swap(h) {
    release.unregister(this);
    W.world_free(this._h);
    this._h = h;
    release.register(this, h, this);
    this._changed(0);
  }
}
for (const k of FIELDS) Object.defineProperty(State.prototype, k, { get() { return this.view()[k]; }, enumerable: true });

export function newState(C, cfg = {}) {
  return new State(create((p, n) => W.world_new(handleOf(C), p, n), cfg));
}

// The world state, ready to become JSON (the save's checkpoint).
export function snapshot(S) { return S._op({ op: "state" }); }
export function restore(C, snap) {
  return new State(create((p, n) => W.world_restore(handleOf(C), p, n), snap));
}

// Changes the state by hand, outside the commands. Only for tests and
// tools: in a game, every effect on the world comes from a command.
export function edit(S, C, f) {
  const snap = snapshot(S);
  f(snap);
  S._swap(create((p, n) => W.world_restore(handleOf(C), p, n), snap));
  return S;
}

// ---------- simulation step
export function step(S, C, n = 1) {
  const events = W.world_step(S._h, n);
  S._changed(events);
}

// ---------- commands: the only way to change the world
export function apply(S, C, cmd) {
  const r = S._op({ op: "apply", cmd });
  S._changed();
  return r;
}

export function stateHash(S) { return S._op({ op: "hash" }); }
export function missionProgress(S) { return S._op({ op: "progress" }); }
export function countMark(S, C, mark) { return S._op({ op: "countMark", mark }); }

// ---------- replay: rebuilds the world from the configuration and the command log
export function replay(C, cfg, cmds, untilTick) {
  return new State(create((p, n) => W.world_replay(handleOf(C), p, n), { cfg, cmds, until: untilTick }));
}
// Redoes the commands from a state, up to the given tick.
export function advance(S, C, cmds, untilTick) {
  S._op({ op: "advance", cmds, until: untilTick });
  S._changed(0);
  S.events = [];
  return S;
}

// ---------- the player
// What can be done on a cell, with labels (for the board menu).
export function possibleActions(S, C, x, y) { return S._op({ op: "actions", x, y }); }
// How much it costs (in multiples of a step) to enter a cell; -1 blocks.
export function passageCost(S, C, x, y) { return W.world_passage_cost(S._h, x, y); }
// Path to where the order can be done: cell indices, [] if already there, null if there is none.
export function path(S, C, P, order) { return S._op({ op: "path", x: P.x, y: P.y, order }); }
export const cellsPerSecond = C => TPS / C.jogador.ticksPorCelula;

// ---------- reading helpers (for the interface)
export const idx = (S, x, y) => x + y * S.L;
export const chunkOf = (S, C, x, y) => Math.floor(x / C.mapa.chunk) + Math.floor(y / C.mapa.chunk) * S.CL;
export const total = o => Object.values(o).reduce((a, b) => a + b, 0);
export function entityAt(S, x, y) {
  if (x < 0 || y < 0 || x >= S.L || y >= S.A) return null;
  const id = S.occ[idx(S, x, y)];
  return id ? S.ents.find(e => e.id === id) : null;
}
