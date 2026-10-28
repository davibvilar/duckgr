import { test } from "node:test";
import assert from "node:assert/strict";
import { N, C, partida, acharCelula } from "./ajuda.js";
import * as Save from "../web/plataforma/persistencia.js";

// Uma partida com alguns minutos e checkpoints, como a interface faz.
function jogar(minutos) {
  const p = partida(), S = p.S;
  const P = { pacote: C.id, cfg: p.cfg, cmds: p.cmds, checkpoint: null };
  let k = 0;
  while (S.tick < minutos * 60 * N.TPS) {
    if (S.tick % (5 * N.TPS) === 0) {
      const a = acharCelula(S, (i, x) => S.cov[i] === "adulta" && x >= (k * 3) % S.L);
      if (a) p.cmd({ tipo: "ordem", ...a, acao: "colher", adicionar: true });
      k++;
    }
    N.passo(S, C);
    Save.talvezCheckpoint(P, S);
  }
  return { S, P };
}

test("salvar e abrir de novo dá o mesmo mundo (mesmo hash)", () => {
  const { S, P } = jogar(12);
  assert.ok(P.checkpoint && P.checkpoint.tick > 0, "tirou checkpoint");
  const save = JSON.parse(JSON.stringify(Save.montarSave(C, P, S)));
  assert.ok(save.historico.filter(c => c.t >= save.checkpoint.tick).length < save.historico.length, "o save refaz só os comandos depois do checkpoint");
  const { S: S2, ok } = Save.carregarSave(C, save);
  assert.ok(ok);
  assert.equal(N.hashEstado(S2), N.hashEstado(S));
});

test("o histórico completo também refaz o mundo do zero", () => {
  const { S, P } = jogar(6);
  assert.equal(N.hashEstado(N.rodar(C, P.cfg, P.cmds, S.tick)), N.hashEstado(S));
});

test("gzip e base64 vão e voltam sem perda, e o arquivo é pequeno", async () => {
  const { S, P } = jogar(10);
  const json = JSON.stringify(Save.montarSave(C, P, S));
  const gz = await Save.comprimir(json);
  assert.ok(gz.length < json.length / 5, `gzip ${gz.length} de ${json.length}`);
  assert.ok(gz.length < 50 * 1024);
  assert.equal(await Save.descomprimir(Save.deBase64(Save.paraBase64(gz))), json);
});

test("save de outro mundo ou de outra versão é recusado", () => {
  const { S, P } = jogar(1);
  const save = Save.montarSave(C, P, S);
  assert.throws(() => Save.carregarSave(C, { ...save, pacote: { id: "outro", versao: "1" } }), /outro mundo/);
  assert.throws(() => Save.carregarSave(C, { ...save, pacote: { ...save.pacote, versao: "0.0.1" } }), /versão/);
  assert.throws(() => Save.carregarSave(C, { formato: "x" }), /não é um save/);
});
