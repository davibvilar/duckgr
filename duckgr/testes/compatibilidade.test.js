import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { N, C } from "./ajuda.js";
import * as Save from "../web/plataforma/persistencia.js";
import { PACOTES } from "../web/jogos/catalogo.js";

const dados = nome => JSON.parse(readFileSync(new URL(`./dados/${nome}`, import.meta.url), "utf8"));

// Este save foi gravado pelo núcleo em JavaScript (v0.4), antes da migração
// para Rust: 13 minutos de partida com máquinas, poluição que adoece árvores,
// pesquisa e missão cumprida.
test("save gravado pelo núcleo em JavaScript abre no núcleo em Rust com o mesmo hash", () => {
  const { save } = dados("save-v0.4.json");
  const { S, ok } = Save.carregarSave(C, save);
  assert.ok(ok, "o hash do mundo refeito confere com o do save");
  assert.equal(S.tick, save.tick);
  assert.equal(S.missao.estado, "cumprida");
});

test("o histórico do save antigo refaz a partida do zero, com o mesmo hash em cada marco", () => {
  const { save, marcos } = dados("save-v0.4.json");
  const S = N.novoEstado(C, save.cfg);
  for (const m of marcos) {
    N.avancar(S, C, save.historico.filter(c => c.t < m.tick), m.tick);
    assert.equal(N.hashEstado(S), m.hash, `tick ${m.tick}`);
  }
});

test("os pacotes exportados para os testes em Rust estão em dia (senão: npm run pacotes)", () => {
  for (const p of PACOTES) assert.deepEqual(dados(`${p.id}.json`), JSON.parse(JSON.stringify(p)));
});
