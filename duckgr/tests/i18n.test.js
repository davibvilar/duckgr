import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { N, C, game, findCell } from "./helpers.js";
import { LANGUAGES, setLanguage, detectLanguage, t, msgText, localizePackage, untranslated } from "../web/platform/i18n.js";
import { PACKAGES, PACKAGE_TRANSLATIONS } from "../web/games/catalog.js";
import en from "../web/platform/locales/en.js";

const keysOf = strings => Object.keys(strings).sort();

test("every language has the same keys", () => {
  for (const l of LANGUAGES) assert.deepEqual(keysOf(l.strings), keysOf(en), l.id);
});

test("every message key the core can send has a text", () => {
  const rust = ["lib.rs", "player.rs"].map(f => readFileSync(new URL(`../crates/duckgr-sim/src/${f}`, import.meta.url), "utf8")).join("\n");
  const keys = [...rust.matchAll(/(?:key|msg)\("([a-z]+\.[A-Za-z.]+)"/g)].map(m => m[1]);
  assert.ok(keys.length > 30, `found ${keys.length} keys`);
  for (const l of LANGUAGES) for (const k of keys) assert.ok(k in l.strings, `${l.id} misses ${k}`);
});

test("the browser's language picks the interface language", () => {
  assert.equal(detectLanguage(["pt-BR", "en"]), "pt-BR");
  assert.equal(detectLanguage(["pt-PT"]), "pt-BR");
  assert.equal(detectLanguage(["en-GB"]), "en");
  assert.equal(detectLanguage(["de-DE", "fr"]), "en");
});

test("the core's messages render in each language, with the package's names", () => {
  const Cl = structuredClone(C); // a copy, so the shared package stays untranslated
  const g = game();
  const water = findCell(g.S, i => g.S.sub[i] === "agua");
  const r = g.cmd({ tipo: "ordem", ...water, acao: "construir", elemento: "fornalha" });
  setLanguage("pt-BR");
  assert.equal(msgText(r.msg, Cl), "Não dá para construir em água.");
  setLanguage("en");
  localizePackage(Cl, PACKAGE_TRANSLATIONS["terra-viva"]);
  assert.equal(msgText(r.msg, Cl), "Can't build on water.");
  assert.equal(t("queue.tasks", { count: 1 }), "1 task in the queue");
  assert.equal(t("queue.tasks", { count: 3 }), "3 tasks in the queue");
  assert.equal(msgText({ key: "error.missing", args: { items: { items: { madeira: 2, "placa-ferro": 1 } } } }, Cl), "Missing 2 wood, 1 iron plate.");
});

test("the world doesn't depend on the language: the package texts the world uses stay in Portuguese", () => {
  const Cx = N.compile(PACKAGES[0]);
  setLanguage("en");
  localizePackage(Cx, PACKAGE_TRANSLATIONS["terra-viva"]);
  assert.equal(Cx.missao.metas[1].nome, "Forest");
  assert.equal(untranslated(Cx).missao.metas[1].nome, "Floresta");
  const cfg = { metas: untranslated(Cx).missao.metas };
  assert.equal(N.stateHash(N.newState(Cx, cfg)), N.stateHash(N.newState(C, {})));
  setLanguage("pt-BR");
  localizePackage(Cx, PACKAGE_TRANSLATIONS["terra-viva"]);
  assert.equal(Cx.missao.metas[1].nome, "Floresta");
});
