// The published site: the service worker must keep every file the game needs
// to open offline, and nothing may come from outside the site.
import { test } from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";

const root = new URL("..", import.meta.url);

test("the built site keeps the whole game for offline play", () => {
  execFileSync("node", ["tools/build-site.mjs"], { cwd: root });
  const sw = readFileSync(new URL("_site/sw.js", root), "utf8");
  const m = sw.match(/const PRECACHE = (\{.*\});/);
  assert.ok(m, "sw.js got its file list");
  const { version, files } = JSON.parse(m[1]);
  assert.match(version, /^[0-9a-f]{12}$/);
  for (const f of ["./", "index.html", "platform/app.js", "platform/duckgr.wasm", "games/catalog.js", "manifest.webmanifest", "icons/icon-192.png"])
    assert.ok(files.includes(f), `${f} is kept offline`);
  assert.ok(!files.some(f => f.startsWith("prototipos/")), "the prototypes are not part of the game");
});

test("the interface loads nothing from other sites", () => {
  for (const f of ["web/index.html", "web/style.css", "web/manifest.webmanifest"])
    assert.doesNotMatch(readFileSync(new URL(f, root), "utf8"), /(src|href)=["']?https?:|url\(["']?https?:/, f);
});
