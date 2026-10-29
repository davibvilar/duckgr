import { test } from "node:test";
import assert from "node:assert/strict";
import { N, C, game, findCell, depositWithFreeNeighbor } from "./helpers.js";

test("cutting an adult tree gives wood and a sapling, and leaves a stump", () => {
  const g = game();
  const a = findCell(g.S, i => g.S.cov[i] === "adulta");
  const wood = g.S.inv.madeira;
  assert.ok(g.order({ ...a, acao: "colher" }).ok);
  assert.equal(g.S.inv.madeira, wood + 4);
  assert.equal(g.S.inv.muda, 5);
  assert.equal(g.S.cov[N.idx(g.S, a.x, a.y)], "toco");
});

test("a planted sapling becomes a young tree and then an adult one", () => {
  const g = game();
  const c = findCell(g.S, (i, x, y) => g.S.cov[i] === "grama" && !g.S.dep[i]
    && ![[1, 0], [-1, 0], [0, 1], [0, -1]].some(([dx, dy]) => g.S.sub[N.idx(g.S, x + dx, y + dy)] === "agua"));
  assert.ok(g.order({ ...c, acao: "plantar", item: "muda" }).ok);
  const i = N.idx(g.S, c.x, c.y);
  g.seconds(40);
  assert.equal(g.S.cov[i], "jovem");
  g.seconds(60);
  assert.equal(g.S.cov[i], "adulta");
});

test("nothing is built on top of a tree", () => {
  const g = game();
  const a = findCell(g.S, i => g.S.cov[i] === "adulta");
  const r = g.cmd({ tipo: "ordem", ...a, acao: "construir", elemento: "fornalha" });
  assert.equal(r.ok, false);
  assert.equal(r.msg.key, "error.clearFirst");
  assert.deepEqual(r.msg.args.cover, { ref: ["coberturas", "adulta", "nome"] });
});

test("a mining drill next to a furnace makes iron plates by itself", () => {
  const g = game();
  const d = depositWithFreeNeighbor(g.S, "ferro");
  assert.ok(g.order({ ...d, acao: "construir", elemento: "mineradora" }).ok);
  assert.ok(g.order({ x: d.x + 1, y: d.y, acao: "construir", elemento: "fornalha" }).ok);
  assert.ok(g.order({ ...d, acao: "abastecer", item: "carvao" }).ok);
  assert.ok(g.order({ x: d.x + 1, y: d.y, acao: "abastecer", item: "madeira" }).ok);
  g.seconds(30);
  assert.ok((g.S.prod["placa-ferro"] || 0) >= 5, `made ${g.S.prod["placa-ferro"]}`);
  const antes = g.S.inv["placa-ferro"];
  assert.ok(g.order({ x: d.x + 1, y: d.y, acao: "recolher" }).ok);
  assert.ok(g.S.inv["placa-ferro"] > antes);
});

test("a machine without fuel stops", () => {
  const g = game();
  const d = depositWithFreeNeighbor(g.S, "ferro");
  g.order({ ...d, acao: "construir", elemento: "mineradora" });
  g.seconds(2);
  const e = N.entityAt(g.S, d.x, d.y);
  assert.equal(e.parado, "Sem combustível"); // the frozen text, which enters the hash
  assert.deepEqual(e.stoppedMsg, { key: "stopped.noFuel" });
});

test("machines pollute, pollution spreads and the forest gets sick", () => {
  const g = game({ recursosIniciais: 20 });
  const S = g.S;
  // furnaces working together in the same area of the map
  const k0 = N.chunkOf(S, C, 20, 12);
  let n = 0;
  N.edit(S, C, e => { e.inv["minerio-ferro"] = 400; });
  for (let y = 8; y < 16 && n < 10; y++) for (let x = 16; x < 24 && n < 10; x++) {
    if (!g.order({ x, y, acao: "construir", elemento: "fornalha" }).ok) continue;
    g.order({ x, y, acao: "fornecer", item: "minerio-ferro" });
    g.order({ x, y, acao: "abastecer", item: "carvao" });
    n++;
  }
  assert.ok(n >= 6, `built ${n} furnaces`);
  const forest0 = N.countMark(S, C, "floresta");
  const neighbors = [k0 - 1, k0 + 1, k0 - S.CL, k0 + S.CL].filter(k => k >= 0 && k < S.campos.poluicao.length);
  let peak = 0, neighborPeak = 0;
  for (let s = 0; s < 40; s++) {
    g.seconds(1);
    peak = Math.max(peak, S.campos.poluicao[k0]);
    neighborPeak = Math.max(neighborPeak, ...neighbors.map(k => S.campos.poluicao[k]));
  }
  assert.ok(peak > 0, "the area got polluted");
  assert.ok(neighborPeak > 0, "pollution reached the neighbors");
  assert.ok(N.countMark(S, C, "floresta") < forest0, "trees got sick");
});

test("mission: accomplished by producing the goal with the forest standing", () => {
  const g = game({ metas: [{ tipo: "producao", nome: "Placas", item: "placa-ferro", quantidade: 3 }, ...C.missao.metas.slice(1)] });
  const d = depositWithFreeNeighbor(g.S, "ferro");
  g.order({ ...d, acao: "construir", elemento: "mineradora" });
  g.order({ x: d.x + 1, y: d.y, acao: "construir", elemento: "fornalha" });
  g.order({ ...d, acao: "abastecer", item: "carvao" });
  g.order({ x: d.x + 1, y: d.y, acao: "abastecer", item: "madeira" });
  g.seconds(30);
  assert.equal(g.S.missao.estado, "cumprida");
});

test("mission: lost if the forest falls below the minimum", () => {
  const g = game({ metas: [C.missao.metas[0], { ...C.missao.metas[1], minimoPct: 99 }] });
  let cuts = 0;
  for (let i = 0; i < g.S.cov.length && cuts < 20; i++) if (g.S.cov[i] === "adulta") {
    g.order({ x: i % g.S.L, y: Math.floor(i / g.S.L), acao: "colher" }); cuts++;
  }
  g.seconds(1);
  assert.equal(g.S.missao.estado, "falhou");
});

test("research: science unlocks the nursery and electricity opens era 2", () => {
  const g = game();
  N.edit(g.S, C, e => { e.inv.ciencia = 25; });
  assert.equal(g.cmd({ tipo: "pesquisar", tecnologia: "eletricidade" }).ok, false); // requires the nursery
  assert.ok(g.cmd({ tipo: "pesquisar", tecnologia: "viveiro" }).ok);
  assert.ok(g.S.liberados.includes("viveiro"));
  assert.ok(g.cmd({ tipo: "pesquisar", tecnologia: "eletricidade" }).ok);
  assert.equal(g.S.era, 2);
});

test("the nursery only makes saplings next to water", () => {
  const g = game({ recursosIniciais: 5 });
  N.edit(g.S, C, e => { e.techs.push("viveiro"); e.liberados.push("viveiro"); });
  const nextToWater = (x, y) => [[1, 0], [-1, 0], [0, 1], [0, -1]].some(([dx, dy]) => g.S.sub[N.idx(g.S, x + dx, y + dy)] === "agua");
  const free = (i, x, y) => x > 0 && x < g.S.L - 1 && y > 0 && y < g.S.A - 1 && g.S.sub[i] === "solo" && !g.S.dep[i] && (!g.S.cov[i] || C.coberturas[g.S.cov[i]].substituivel);
  const near = findCell(g.S, (i, x, y) => free(i, x, y) && nextToWater(x, y));
  const far = findCell(g.S, (i, x, y) => free(i, x, y) && !nextToWater(x, y));
  for (const c of [near, far]) {
    assert.ok(g.order({ ...c, acao: "construir", elemento: "viveiro" }).ok);
    assert.ok(g.order({ ...c, acao: "fornecer", item: "madeira" }).ok);
  }
  g.seconds(10);
  assert.ok(total(N.entityAt(g.S, near.x, near.y).sai) >= 2);
  const stopped = N.entityAt(g.S, far.x, far.y);
  assert.equal(stopped.parado, "Precisa ficar ao lado da água");
  assert.deepEqual(stopped.stoppedMsg, { ref: ["receitas", "mudas", "exige", "aviso"], text: "Precisa ficar ao lado da água" });
});
const total = o => Object.values(o).reduce((a, b) => a + b, 0);
