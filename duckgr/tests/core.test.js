import { test } from "node:test";
import assert from "node:assert/strict";
import { N, C, game, findCell } from "./helpers.js";

test("the Terra Viva package compiles without errors", () => {
  assert.equal(C.id, "terra-viva");
  assert.equal(C.receitas["placa-ferro"].ticks, 96); // 3.2 s × 30 ticks
});

test("a package with a broken reference is refused when compiling", () => {
  const bad = structuredClone(C);
  bad.receitas["placa-ferro"].sai = { "placa-de-ouro": 1 };
  assert.throws(() => N.compile(bad), /placa-de-ouro/);
});

test("the same seed makes the same map; another seed makes another", () => {
  const a = N.stateHash(N.newState(C, { semente: 7 }));
  const b = N.stateHash(N.newState(C, { semente: 7 }));
  const c = N.stateHash(N.newState(C, { semente: 8 }));
  assert.equal(a, b);
  assert.notEqual(a, c);
});

test("the default map has water, forest and the four deposits", () => {
  const S = N.newState(C);
  assert.ok(S.sub.includes("agua"));
  assert.ok(N.countMark(S, C, "floresta") > 100);
  for (const d of ["ferro", "cobre", "carvao", "pedra"]) assert.ok(S.dep.includes(d), `missing ${d}`);
});

test("determinism: redoing the game from the commands gives the same hash", () => {
  const g = game();
  const tree = findCell(g.S, i => g.S.cov[i] === "adulta");
  g.cmd({ tipo: "ordem", ...tree, acao: "colher" });
  g.seconds(20);
  const free = findCell(g.S, i => g.S.cov[i] === "grama" && !g.S.dep[i]);
  g.cmd({ tipo: "ordem", ...free, acao: "plantar", item: "muda", adicionar: true });
  g.cmd({ tipo: "ordem", x: 0, y: 0, acao: "ir", adicionar: true });
  g.seconds(40);
  assert.ok(g.S.jog.x !== g.S.jog.y || g.S.jog.x === 0 || g.S.jog.fila.length, "the duck walked");
  const redone = N.replay(C, g.cfg, g.cmds, g.S.tick);
  assert.equal(N.stateHash(redone), N.stateHash(g.S));
});

test("a failed command doesn't change the world", () => {
  const g = game();
  const before = N.stateHash(g.S);
  const water = findCell(g.S, i => g.S.sub[i] === "agua");
  const r = g.cmd({ tipo: "ordem", ...water, acao: "construir", elemento: "fornalha" });
  assert.equal(r.ok, false);
  assert.equal(N.stateHash(g.S), before);
});

test("an element not researched yet can't be built", () => {
  const g = game({ recursosIniciais: 5 });
  const free = findCell(g.S, i => g.S.cov[i] === "grama" && !g.S.dep[i]);
  const r = g.cmd({ tipo: "ordem", ...free, acao: "construir", elemento: "viveiro" });
  assert.equal(r.ok, false);
  assert.deepEqual(r.msg, { key: "error.notResearched" });
});
