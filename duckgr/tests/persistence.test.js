import { test } from "node:test";
import assert from "node:assert/strict";
import { N, C, game, findCell } from "./helpers.js";
import * as Save from "../web/platform/persistence.js";

// A game with a few minutes and checkpoints, like the interface plays it.
function play(minutes) {
  const g = game(), S = g.S;
  const G = { pkg: C.id, cfg: g.cfg, cmds: g.cmds, checkpoint: null };
  let k = 0;
  while (S.tick < minutes * 60 * N.TPS) {
    if (S.tick % (5 * N.TPS) === 0) {
      const a = findCell(S, (i, x) => S.cov[i] === "adulta" && x >= (k * 3) % S.L);
      if (a) g.cmd({ tipo: "ordem", ...a, acao: "colher", adicionar: true });
      k++;
    }
    N.step(S, C);
    Save.maybeCheckpoint(G, S);
  }
  return { S, G };
}

test("saving and opening again gives the same world (same hash)", () => {
  const { S, G } = play(12);
  assert.ok(G.checkpoint && G.checkpoint.tick > 0, "took a checkpoint");
  const save = JSON.parse(JSON.stringify(Save.buildSave(C, G, S)));
  assert.ok(save.historico.filter(c => c.t >= save.checkpoint.tick).length < save.historico.length, "the save redoes only the commands after the checkpoint");
  const { S: S2, ok } = Save.loadSave(C, save);
  assert.ok(ok);
  assert.equal(N.stateHash(S2), N.stateHash(S));
  // the checkpoint keeps the log's messages, so a loaded game shows them in any language
  assert.ok(S2.log.some(l => l.msg), "the log entries kept their messages");
});

test("the full history also redoes the world from zero", () => {
  const { S, G } = play(6);
  assert.equal(N.stateHash(N.replay(C, G.cfg, G.cmds, S.tick)), N.stateHash(S));
});

test("gzip and base64 go and come back without loss, and the file is small", async () => {
  const { S, G } = play(10);
  const json = JSON.stringify(Save.buildSave(C, G, S));
  const gz = await Save.compress(json);
  assert.ok(gz.length < json.length / 5, `gzip ${gz.length} of ${json.length}`);
  assert.ok(gz.length < 50 * 1024);
  assert.equal(await Save.decompress(Save.fromBase64(Save.toBase64(gz))), json);
});

test("a save from another world or another version is refused, saying why", () => {
  const { S, G } = play(1);
  const save = Save.buildSave(C, G, S);
  const refused = (s, key) => assert.throws(() => Save.loadSave(C, s), e => e instanceof Save.SaveError && e.key === key);
  refused({ ...save, pacote: { id: "other", versao: "1" } }, "save.error.otherWorld");
  refused({ ...save, pacote: { ...save.pacote, versao: "0.0.1" } }, "save.error.otherVersion");
  refused({ formato: "x" }, "save.error.notASave");
});
