import { test } from "node:test";
import assert from "node:assert/strict";
import { N, C, partida, acharCelula, depositoComVizinhoLivre } from "./ajuda.js";

test("cortar uma árvore adulta dá madeira e uma muda, e deixa um toco", () => {
  const p = partida();
  const a = acharCelula(p.S, i => p.S.cov[i] === "adulta");
  const madeira = p.S.inv.madeira;
  assert.ok(p.fazer({ ...a, acao: "colher" }).ok);
  assert.equal(p.S.inv.madeira, madeira + 4);
  assert.equal(p.S.inv.muda, 5);
  assert.equal(p.S.cov[N.idx(p.S, a.x, a.y)], "toco");
});

test("uma muda plantada vira árvore jovem e depois adulta", () => {
  const p = partida();
  const c = acharCelula(p.S, (i, x, y) => p.S.cov[i] === "grama" && !p.S.dep[i]
    && ![[1, 0], [-1, 0], [0, 1], [0, -1]].some(([dx, dy]) => p.S.sub[N.idx(p.S, x + dx, y + dy)] === "agua"));
  assert.ok(p.fazer({ ...c, acao: "plantar", item: "muda" }).ok);
  const i = N.idx(p.S, c.x, c.y);
  p.segundos(40);
  assert.equal(p.S.cov[i], "jovem");
  p.segundos(60);
  assert.equal(p.S.cov[i], "adulta");
});

test("não se constrói em cima de árvore", () => {
  const p = partida();
  const a = acharCelula(p.S, i => p.S.cov[i] === "adulta");
  const r = p.cmd({ tipo: "ordem", ...a, acao: "construir", elemento: "fornalha" });
  assert.equal(r.ok, false);
  assert.match(r.msg, /árvore/i);
});

test("mineradora encostada na fornalha produz placas de ferro sozinha", () => {
  const p = partida();
  const d = depositoComVizinhoLivre(p.S, "ferro");
  assert.ok(p.fazer({ ...d, acao: "construir", elemento: "mineradora" }).ok);
  assert.ok(p.fazer({ x: d.x + 1, y: d.y, acao: "construir", elemento: "fornalha" }).ok);
  assert.ok(p.fazer({ ...d, acao: "abastecer", item: "carvao" }).ok);
  assert.ok(p.fazer({ x: d.x + 1, y: d.y, acao: "abastecer", item: "madeira" }).ok);
  p.segundos(30);
  assert.ok((p.S.prod["placa-ferro"] || 0) >= 5, `produziu ${p.S.prod["placa-ferro"]}`);
  const antes = p.S.inv["placa-ferro"];
  assert.ok(p.fazer({ x: d.x + 1, y: d.y, acao: "recolher" }).ok);
  assert.ok(p.S.inv["placa-ferro"] > antes);
});

test("máquina sem combustível para", () => {
  const p = partida();
  const d = depositoComVizinhoLivre(p.S, "ferro");
  p.fazer({ ...d, acao: "construir", elemento: "mineradora" });
  p.segundos(2);
  assert.equal(N.entidadeEm(p.S, d.x, d.y).parado, "Sem combustível");
});

test("máquinas poluem, a poluição se espalha e a floresta adoece", () => {
  const p = partida({ recursosIniciais: 20 });
  const S = p.S;
  // fornalhas trabalhando juntas no mesmo trecho do mapa
  const k0 = N.chunkDe(S, C, 20, 12);
  let n = 0;
  S.inv["minerio-ferro"] = 400;
  for (let y = 8; y < 16 && n < 10; y++) for (let x = 16; x < 24 && n < 10; x++) {
    if (!p.fazer({ x, y, acao: "construir", elemento: "fornalha" }).ok) continue;
    p.fazer({ x, y, acao: "fornecer", item: "minerio-ferro" });
    p.fazer({ x, y, acao: "abastecer", item: "carvao" });
    n++;
  }
  assert.ok(n >= 6, `construiu ${n} fornalhas`);
  const floresta0 = N.contarMarca(S, C, "floresta");
  const vizinhos = [k0 - 1, k0 + 1, k0 - S.CL, k0 + S.CL].filter(k => k >= 0 && k < S.campos.poluicao.length);
  let pico = 0, picoViz = 0;
  for (let s = 0; s < 40; s++) {
    p.segundos(1);
    pico = Math.max(pico, S.campos.poluicao[k0]);
    picoViz = Math.max(picoViz, ...vizinhos.map(k => S.campos.poluicao[k]));
  }
  assert.ok(pico > 0, "o trecho ficou poluído");
  assert.ok(picoViz > 0, "a poluição chegou aos vizinhos");
  assert.ok(N.contarMarca(S, C, "floresta") < floresta0, "árvores adoeceram");
});

test("missão: cumprida ao produzir a meta com a floresta de pé", () => {
  const p = partida({ metas: [{ tipo: "producao", nome: "Placas", item: "placa-ferro", quantidade: 3 }, ...C.missao.metas.slice(1)] });
  const d = depositoComVizinhoLivre(p.S, "ferro");
  p.fazer({ ...d, acao: "construir", elemento: "mineradora" });
  p.fazer({ x: d.x + 1, y: d.y, acao: "construir", elemento: "fornalha" });
  p.fazer({ ...d, acao: "abastecer", item: "carvao" });
  p.fazer({ x: d.x + 1, y: d.y, acao: "abastecer", item: "madeira" });
  p.segundos(30);
  assert.equal(p.S.missao.estado, "cumprida");
});

test("missão: perdida se a floresta cair abaixo do mínimo", () => {
  const p = partida({ metas: [C.missao.metas[0], { ...C.missao.metas[1], minimoPct: 99 }] });
  let cortes = 0;
  for (let i = 0; i < p.S.cov.length && cortes < 20; i++) if (p.S.cov[i] === "adulta") {
    p.fazer({ x: i % p.S.L, y: Math.floor(i / p.S.L), acao: "colher" }); cortes++;
  }
  p.segundos(1);
  assert.equal(p.S.missao.estado, "falhou");
});

test("pesquisa: ciência libera o viveiro e a eletricidade abre a era 2", () => {
  const p = partida();
  p.S.inv.ciencia = 25;
  assert.equal(p.cmd({ tipo: "pesquisar", tecnologia: "eletricidade" }).ok, false); // requer viveiro
  assert.ok(p.cmd({ tipo: "pesquisar", tecnologia: "viveiro" }).ok);
  assert.ok(p.S.liberados.includes("viveiro"));
  assert.ok(p.cmd({ tipo: "pesquisar", tecnologia: "eletricidade" }).ok);
  assert.equal(p.S.era, 2);
});

test("viveiro só produz mudas ao lado da água", () => {
  const p = partida({ recursosIniciais: 5 });
  p.S.techs.push("viveiro"); p.S.liberados.push("viveiro");
  const viz = (x, y) => [[1, 0], [-1, 0], [0, 1], [0, -1]].some(([dx, dy]) => p.S.sub[N.idx(p.S, x + dx, y + dy)] === "agua");
  const livre = (i, x, y) => x > 0 && x < p.S.L - 1 && y > 0 && y < p.S.A - 1 && p.S.sub[i] === "solo" && !p.S.dep[i] && (!p.S.cov[i] || C.coberturas[p.S.cov[i]].substituivel);
  const perto = acharCelula(p.S, (i, x, y) => livre(i, x, y) && viz(x, y));
  const longe = acharCelula(p.S, (i, x, y) => livre(i, x, y) && !viz(x, y));
  for (const c of [perto, longe]) {
    assert.ok(p.fazer({ ...c, acao: "construir", elemento: "viveiro" }).ok);
    assert.ok(p.fazer({ ...c, acao: "fornecer", item: "madeira" }).ok);
  }
  p.segundos(10);
  assert.ok(total(N.entidadeEm(p.S, perto.x, perto.y).sai) >= 2);
  assert.equal(N.entidadeEm(p.S, longe.x, longe.y).parado, "Precisa ficar ao lado da água");
});
const total = o => Object.values(o).reduce((a, b) => a + b, 0);
