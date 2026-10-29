// Exports the world packages (web/games/) to JSON in tests/data/, so the
// Rust tests use the same data as the game. A JavaScript test checks that
// the JSON didn't fall behind: if it fails, run `npm run packages`.
import { writeFileSync } from "node:fs";
import { PACKAGES } from "../web/games/catalog.js";

for (const p of PACKAGES) {
  const target = new URL(`../tests/data/${p.id}.json`, import.meta.url);
  writeFileSync(target, JSON.stringify(p, null, 1) + "\n");
  console.log(`tests/data/${p.id}.json`);
}
