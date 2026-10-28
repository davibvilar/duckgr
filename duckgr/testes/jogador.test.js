import { test } from "node:test";
import assert from "node:assert/strict";
import { N, C, partida, acharCelula } from "./ajuda.js";
import { caminho, custoPassagem, acoesPossiveis } from "../web/plataforma/jogador.js";

const longe = (S, a, b) => Math.max(Math.abs(a.x - b.x), Math.abs(a.y - b.y));

test("o jogador começa numa célula onde dá para pisar", () => {
  const { S } = partida();
  assert.equal(custoPassagem(S, C, S.jog.x, S.jog.y), 1);
});

test("o jogador anda a pé na velocidade do pacote", () => {
  const p = partida(), J = p.S.jog;
  // um destino reto, de grama, a 4 células
  const alvo = { x: J.x, y: J.y };
  const r = caminho(p.S, C, J, { x: J.x + 4, y: J.y, acao: "ir" });
  assert.ok(r, "há caminho");
  p.cmd({ tipo: "ordem", x: J.x + 4, y: J.y, acao: "ir" });
  const custo = r.reduce((a, i) => a + custoPassagem(p.S, C, i % p.S.L, Math.floor(i / p.S.L)), 0);
  for (let k = 0; k < custo * C.jogador.ticksPorCelula - 1; k++) N.passo(p.S, C);
  assert.notEqual(J.x, alvo.x + 4, "ainda não chegou um tick antes");
  N.passo(p.S, C);
  assert.deepEqual([J.x, J.y], [alvo.x + 4, alvo.y]);
});

test("o jogador só age ao alcance: anda até a árvore antes de cortar", () => {
  const p = partida(), S = p.S;
  const a = acharCelula(S, i => S.cov[i] === "adulta" && longe(S, { x: i % S.L, y: Math.floor(i / S.L) }, S.jog) > 3);
  const madeira = S.inv.madeira;
  p.cmd({ tipo: "ordem", ...a, acao: "colher" });
  p.segundos(0.5);
  assert.equal(S.inv.madeira, madeira, "ainda está andando");
  assert.ok(p.fazer({ ...a, acao: "colher" }).ok);
  assert.ok(longe(S, a, S.jog) <= C.jogador.alcance);
  assert.equal(S.cov[N.idx(S, a.x, a.y)], "toco");
});

test("água funda bloqueia; o pato atravessa o rio pelo vau", () => {
  const p = partida(), S = p.S;
  assert.ok(S.sub.includes("vau"), "o rio tem vau");
  const outroLado = acharCelula(S, (i, x) => x < 3 && custoPassagem(S, C, x, Math.floor(i / S.L)) === 1);
  assert.ok(p.fazer({ ...outroLado, acao: "ir" }).ok);
  assert.deepEqual([S.jog.x, S.jog.y], [outroLado.x, outroLado.y]);
  const agua = acharCelula(S, i => S.sub[i] === "agua");
  assert.equal(p.cmd({ tipo: "ordem", ...agua, acao: "ir" }).ok, false);
});

test("a fila executa as ordens em sequência; sem Shift, a nova ordem substitui", () => {
  const p = partida(), S = p.S;
  const arvores = [];
  for (let i = 0; i < S.cov.length && arvores.length < 3; i++) if (S.cov[i] === "adulta") arvores.push({ x: i % S.L, y: Math.floor(i / S.L) });
  for (const a of arvores) p.cmd({ tipo: "ordem", ...a, acao: "colher", adicionar: true });
  assert.equal(S.jog.fila.length, 3);
  p.segundos(60);
  for (const a of arvores) assert.equal(S.cov[N.idx(S, a.x, a.y)], "toco");
  p.cmd({ tipo: "ordem", x: 0, y: 0, acao: "ir", adicionar: true });
  p.cmd({ tipo: "ordem", x: S.jog.x, y: S.jog.y, acao: "ir" });
  assert.equal(S.jog.fila.length, 1);
  assert.ok(p.cmd({ tipo: "cancelar" }).ok);
  assert.equal(S.jog.fila.length, 0);
});

test("minerar repete a ação o número de vezes da ordem", () => {
  const p = partida(), S = p.S;
  const d = acharCelula(S, i => S.dep[i] === "pedra");
  const antes = S.inv.pedra;
  assert.ok(p.fazer({ ...d, acao: "minerar", vezes: 5 }).ok);
  assert.equal(S.inv.pedra, antes + 5);
});

test("o menu da célula lista o que dá para fazer, sem olhar a distância", () => {
  const { S } = partida();
  const a = acharCelula(S, i => S.cov[i] === "adulta");
  const acoes = acoesPossiveis(S, C, a.x, a.y).acoes.map(x => x.acao);
  assert.ok(acoes.includes("colher"));
  assert.ok(!acoes.includes("construir"), "não se constrói em cima da árvore");
  const ferro = acharCelula(S, i => S.dep[i] === "ferro");
  const f = acoesPossiveis(S, C, ferro.x, ferro.y).acoes.map(x => x.acao + (x.elemento ? ":" + x.elemento : ""));
  assert.ok(f.includes("minerar") && f.includes("construir:mineradora"));
});
