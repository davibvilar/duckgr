import { test } from "node:test";
import assert from "node:assert/strict";
import { N, C, game, findCell } from "./helpers.js";
import { path, passageCost, possibleActions } from "../web/platform/core.js";

const distance = (a, b) => Math.max(Math.abs(a.x - b.x), Math.abs(a.y - b.y));

test("the player starts on a cell that can be stepped on", () => {
  const { S } = game();
  assert.equal(passageCost(S, C, S.jog.x, S.jog.y), 1);
});

test("the player walks at the package's speed", () => {
  const g = game(), P = () => g.S.jog;
  // a straight destination, over grass, 4 cells away
  const target = { x: P().x, y: P().y };
  const r = path(g.S, C, P(), { x: target.x + 4, y: target.y, acao: "ir" });
  assert.ok(r, "there is a path");
  g.cmd({ tipo: "ordem", x: target.x + 4, y: target.y, acao: "ir" });
  const cost = r.reduce((a, i) => a + passageCost(g.S, C, i % g.S.L, Math.floor(i / g.S.L)), 0);
  N.step(g.S, C, cost * C.jogador.ticksPorCelula - 1);
  assert.notEqual(P().x, target.x + 4, "not there yet one tick before");
  N.step(g.S, C);
  assert.deepEqual([P().x, P().y], [target.x + 4, target.y]);
});

test("the player only acts within reach: walks to the tree before cutting it", () => {
  const g = game(), S = g.S;
  const a = findCell(S, i => S.cov[i] === "adulta" && distance({ x: i % S.L, y: Math.floor(i / S.L) }, S.jog) > 3);
  const wood = S.inv.madeira;
  g.cmd({ tipo: "ordem", ...a, acao: "colher" });
  g.seconds(0.5);
  assert.equal(S.inv.madeira, wood, "still walking");
  assert.ok(g.order({ ...a, acao: "colher" }).ok);
  assert.ok(distance(a, S.jog) <= C.jogador.alcance);
  assert.equal(S.cov[N.idx(S, a.x, a.y)], "toco");
});

test("deep water blocks; the duck crosses the river at the ford", () => {
  const g = game(), S = g.S;
  assert.ok(S.sub.includes("vau"), "the river has a ford");
  const otherSide = findCell(S, (i, x) => x < 3 && passageCost(S, C, x, Math.floor(i / S.L)) === 1);
  assert.ok(g.order({ ...otherSide, acao: "ir" }).ok);
  assert.deepEqual([S.jog.x, S.jog.y], [otherSide.x, otherSide.y]);
  const water = findCell(S, i => S.sub[i] === "agua");
  const r = g.cmd({ tipo: "ordem", ...water, acao: "ir" });
  assert.equal(r.ok, false);
  assert.deepEqual(r.msg, { key: "error.cannotStep" });
});

test("the queue runs the orders in sequence; without `adicionar`, the new order replaces it", () => {
  const g = game(), S = g.S;
  const trees = [];
  for (let i = 0; i < S.cov.length && trees.length < 3; i++) if (S.cov[i] === "adulta") trees.push({ x: i % S.L, y: Math.floor(i / S.L) });
  for (const a of trees) g.cmd({ tipo: "ordem", ...a, acao: "colher", adicionar: true });
  assert.equal(S.jog.fila.length, 3);
  g.seconds(60);
  for (const a of trees) assert.equal(S.cov[N.idx(S, a.x, a.y)], "toco");
  g.cmd({ tipo: "ordem", x: 0, y: 0, acao: "ir", adicionar: true });
  g.cmd({ tipo: "ordem", x: S.jog.x, y: S.jog.y, acao: "ir" });
  assert.equal(S.jog.fila.length, 1);
  assert.ok(g.cmd({ tipo: "cancelar" }).ok);
  assert.equal(S.jog.fila.length, 0);
});

test("mining repeats the action the order's number of times", () => {
  const g = game(), S = g.S;
  const d = findCell(S, i => S.dep[i] === "pedra");
  const before = S.inv.pedra;
  assert.ok(g.order({ ...d, acao: "minerar", vezes: 5 }).ok);
  assert.equal(S.inv.pedra, before + 5);
});

test("mining takes a free number of times, well beyond the old limit of 99", () => {
  const g = game(), { S } = g;
  const i = S.dep.findIndex(Boolean);
  N.edit(S, C, snap => { snap.depQtd[i] = 500; }); // a big deposit, by hand
  const d = { x: i % S.L, y: Math.floor(i / S.L) }, item = C.depositos[S.dep[i]].item;
  const before = S.inv[item] || 0;
  const left = S.depQtd[i];
  const r = g.order({ ...d, acao: "minerar", vezes: 120 }, 600);
  assert.ok(r.ok, JSON.stringify(r));
  assert.equal((S.inv[item] || 0) - before, 120);
  assert.equal(S.depQtd[i], left - 120);
});

test("a repeated order ends quietly when the deposit runs out, and the next order goes", () => {
  const g = game(), S = g.S;
  const i = S.depQtd.findIndex((n, i) => S.dep[i] && n > 3 && n < 60);
  assert.ok(i >= 0, "there is a small deposit");
  const d = { x: i % S.L, y: Math.floor(i / S.L) }, left = S.depQtd[i], item = C.depositos[S.dep[i]].item;
  const before = S.inv[item] || 0;
  // asks for more than the deposit has, then something else
  g.cmd({ tipo: "ordem", ...d, acao: "minerar", vezes: left + 10, adicionar: true });
  g.cmd({ tipo: "ordem", x: S.jog.x, y: S.jog.y, acao: "ir", adicionar: true });
  S.events = [];
  g.seconds(120);
  assert.deepEqual(S.events.filter(e => e.type === "warning"), [], "no warning: the order just ends");
  assert.equal(S.jog.fila.length, 0, "the queue went to the end");
  assert.equal((S.inv[item] || 0) - before, left);
  assert.equal(S.dep[i], null);
});

test("a queued order can be taken out by its position", () => {
  const g = game(), S = g.S;
  const cells = [1, 2, 3].map(dx => ({ x: S.jog.x + dx, y: S.jog.y }));
  for (const c of cells) assert.ok(g.cmd({ tipo: "ordem", ...c, acao: "ir", adicionar: true }).ok);
  assert.ok(g.cmd({ tipo: "remover", indice: 1 }).ok);
  assert.deepEqual(S.jog.fila.map(o => o.x), [cells[0].x, cells[2].x]);
  assert.ok(g.cmd({ tipo: "remover", indice: 0 }).ok);
  assert.deepEqual(S.jog.fila.map(o => o.x), [cells[2].x]);
  assert.equal(g.cmd({ tipo: "remover", indice: 5 }).ok, false);
  assert.equal(g.cmd({ tipo: "remover", indice: -1 }).ok, false);
});

test("the queue holds up to the limit, and says when it is full", () => {
  const g = game(), S = g.S;
  for (let k = 0; k < N.QUEUE_MAX; k++) assert.ok(g.cmd({ tipo: "ordem", x: S.jog.x, y: S.jog.y, acao: "ir", adicionar: true }).ok);
  const r = g.cmd({ tipo: "ordem", x: S.jog.x, y: S.jog.y, acao: "ir", adicionar: true });
  assert.equal(r.ok, false);
  assert.deepEqual(r.msg, { key: "error.queueFull" });
});

test("the cell menu lists what can be done, without looking at the distance", () => {
  const { S } = game();
  const a = findCell(S, i => S.cov[i] === "adulta");
  const menu = possibleActions(S, C, a.x, a.y);
  const actions = menu.actions.map(x => x.acao);
  assert.ok(actions.includes("colher"));
  assert.ok(!actions.includes("construir"), "nothing is built on top of the tree");
  // the labels are messages, with the package's texts by reference
  const cut = menu.actions.find(x => x.acao === "colher");
  assert.equal(cut.label.key, "action.harvest");
  assert.deepEqual(cut.label.args.cover, { ref: ["coberturas", "adulta", "nome"] });
  assert.deepEqual(menu.title, { ref: ["coberturas", "adulta", "nome"] });
  const iron = findCell(S, i => S.dep[i] === "ferro");
  const f = possibleActions(S, C, iron.x, iron.y).actions.map(x => x.acao + (x.elemento ? ":" + x.elemento : ""));
  assert.ok(f.includes("minerar") && f.includes("construir:mineradora"));
});
