import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { N, C } from "./helpers.js";
import * as Save from "../web/platform/persistence.js";
import { PACKAGES } from "../web/games/catalog.js";
import { msgText, setLanguage } from "../web/platform/i18n.js";

const data = name => JSON.parse(readFileSync(new URL(`./data/${name}`, import.meta.url), "utf8"));

// This save was written by the JavaScript core (v0.4), before the move to
// Rust: 13 minutes of play with machines, pollution that makes trees sick,
// research and an accomplished mission.
test("a save written by the JavaScript core opens in the Rust core with the same hash", () => {
  const { save } = data("save-v0.4.json");
  const { S, ok } = Save.loadSave(C, save);
  assert.ok(ok, "the rebuilt world's hash matches the save's");
  assert.equal(S.tick, save.tick);
  assert.equal(S.missao.estado, "cumprida");
});

test("the old save's log, written only in Portuguese, gets messages that render back to the same text", () => {
  const { save } = data("save-v0.4.json");
  const S = N.restore(C, save.checkpoint.estado);
  setLanguage("pt-BR");
  assert.ok(S.log.length > 10);
  for (const entry of S.log) assert.equal(msgText(entry.msg, C), entry.txt);
});

test("the old save's history redoes the game from zero, with the same hash at each milestone", () => {
  const { save, milestones } = data("save-v0.4.json");
  const S = N.newState(C, save.cfg);
  for (const m of milestones) {
    N.advance(S, C, save.historico.filter(c => c.t < m.tick), m.tick);
    assert.equal(N.stateHash(S), m.hash, `tick ${m.tick}`);
  }
});

test("the packages exported for the Rust tests are up to date (otherwise: npm run packages)", () => {
  for (const p of PACKAGES) assert.deepEqual(data(`${p.id}.json`), JSON.parse(JSON.stringify(p)));
});
