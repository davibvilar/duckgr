// Helpers for the tests: building small worlds and finding cells.
import * as N from "../web/platform/core.js";
import terraViva from "../web/games/terra-viva/package.js";

export { N };
export const C = N.compile(terraViva);

// A test player: applies commands keeping the log, like the interface does.
export function game(cfg = {}) {
  const S = N.newState(C, cfg), cmds = [];
  return {
    S, cmds, cfg,
    cmd(c) { c.t = S.tick; c.autor = "test"; const r = N.apply(S, C, c); if (r.ok) cmds.push(c); return r; },
    seconds(s) { N.step(S, C, Math.ceil(s * N.TPS)); },
    // Gives the player an order and runs the world until the queue is empty.
    // Returns { ok, warnings } with the warnings the player raised on the way.
    order(o, maxSeconds = 120) {
      S.events = [];
      const r = this.cmd({ tipo: "ordem", ...o });
      if (!r.ok) return { ok: false, msg: r.msg };
      for (let k = 0; k < maxSeconds * N.TPS && S.jog.fila.length; k++) N.step(S, C);
      const warnings = S.events.filter(e => e.type === "warning").map(e => e.msg);
      return { ok: !S.jog.fila.length && !warnings.length, warnings };
    },
  };
}

export function findCell(S, test) {
  for (let y = 0; y < S.A; y++) for (let x = 0; x < S.L; x++) if (test(N.idx(S, x, y), x, y)) return { x, y };
  return null;
}

// A deposit of a kind with a free cell to its right (to put a machine next to it).
export function depositWithFreeNeighbor(S, kind) {
  return findCell(S, (i, x) => S.dep[i] === kind && x + 1 < S.L && !S.dep[i + 1] && S.sub[i + 1] === "solo"
    && (!S.cov[i + 1] || C.coberturas[S.cov[i + 1]].substituivel));
}
