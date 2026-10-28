import { test } from "node:test";
import assert from "node:assert/strict";
import { N, C, partida, acharCelula } from "./ajuda.js";

test("o pacote Terra Viva compila sem erros", () => {
  assert.equal(C.id, "terra-viva");
  assert.equal(C.receitas["placa-ferro"].ticks, 96); // 3,2 s × 30 ticks
});

test("pacote com referência quebrada é recusado na compilação", () => {
  const ruim = structuredClone(C);
  ruim.receitas["placa-ferro"].sai = { "placa-de-ouro": 1 };
  assert.throws(() => N.compilar(ruim), /placa-de-ouro/);
});

test("a mesma semente gera o mesmo mapa; outra semente gera outro", () => {
  const a = N.hashEstado(N.novoEstado(C, { semente: 7 }));
  const b = N.hashEstado(N.novoEstado(C, { semente: 7 }));
  const c = N.hashEstado(N.novoEstado(C, { semente: 8 }));
  assert.equal(a, b);
  assert.notEqual(a, c);
});

test("o mapa padrão tem água, floresta e os quatro depósitos", () => {
  const S = N.novoEstado(C);
  assert.ok(S.sub.includes("agua"));
  assert.ok(N.contarMarca(S, C, "floresta") > 100);
  for (const d of ["ferro", "cobre", "carvao", "pedra"]) assert.ok(S.dep.includes(d), `falta ${d}`);
});

test("determinismo: refazer a partida pelos comandos dá o mesmo hash", () => {
  const p = partida();
  const arvore = acharCelula(p.S, i => p.S.cov[i] === "adulta");
  p.cmd({ tipo: "ordem", ...arvore, acao: "colher" });
  p.segundos(20);
  const livre = acharCelula(p.S, i => p.S.cov[i] === "grama" && !p.S.dep[i]);
  p.cmd({ tipo: "ordem", ...livre, acao: "plantar", item: "muda", adicionar: true });
  p.cmd({ tipo: "ordem", x: 0, y: 0, acao: "ir", adicionar: true });
  p.segundos(40);
  assert.ok(p.S.jog.x !== p.S.jog.y || p.S.jog.x === 0 || p.S.jog.fila.length, "o pato andou");
  const refeito = N.rodar(C, p.cfg, p.cmds, p.S.tick);
  assert.equal(N.hashEstado(refeito), N.hashEstado(p.S));
});

test("comando que falha não muda o mundo", () => {
  const p = partida();
  const antes = N.hashEstado(p.S);
  const agua = acharCelula(p.S, i => p.S.sub[i] === "agua");
  const r = p.cmd({ tipo: "ordem", ...agua, acao: "construir", elemento: "fornalha" });
  assert.equal(r.ok, false);
  assert.equal(N.hashEstado(p.S), antes);
});

test("elemento ainda não pesquisado não pode ser construído", () => {
  const p = partida({ recursosIniciais: 5 });
  const livre = acharCelula(p.S, i => p.S.cov[i] === "grama" && !p.S.dep[i]);
  assert.equal(p.cmd({ tipo: "ordem", ...livre, acao: "construir", elemento: "viveiro" }).ok, false);
});
