// Builds the site that GitHub Pages publishes into _site/: the web/ folder,
// the prototypes, and the service worker's list of files to keep offline.
// The cache version is a hash of those files, so any change to the game makes
// installed copies download the new version.
// Run `npm run wasm` first: the site needs web/platform/duckgr.wasm.
import { cpSync, existsSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { createHash } from "node:crypto";
import { join, relative, sep } from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("..", import.meta.url));
const site = join(root, "_site");
if (!existsSync(join(root, "web/platform/duckgr.wasm"))) throw new Error("web/platform/duckgr.wasm is missing. Run `npm run wasm` first.");

rmSync(site, { recursive: true, force: true });
cpSync(join(root, "web"), site, { recursive: true });

const walk = dir => readdirSync(dir).flatMap(n => {
  const p = join(dir, n);
  return statSync(p).isDirectory() ? walk(p) : [p];
});
// sw.js updates itself through the browser; the READMEs are not part of the game
const files = walk(site)
  .map(p => relative(site, p).split(sep).join("/"))
  .filter(p => p !== "sw.js" && !p.endsWith(".md"))
  .sort();
const hash = createHash("sha256");
for (const f of files) hash.update(f).update(readFileSync(join(site, f)));
const version = hash.digest("hex").slice(0, 12);

const swPath = join(site, "sw.js");
const sw = readFileSync(swPath, "utf8");
const filled = sw.replace("const PRECACHE = null;", `const PRECACHE = ${JSON.stringify({ version, files: ["./", ...files] })};`);
if (filled === sw) throw new Error("sw.js no longer has the PRECACHE line that this tool fills in.");
writeFileSync(swPath, filled);

mkdirSync(join(site, "prototipos"), { recursive: true });
for (const f of readdirSync(join(root, "docs/prototipos")).filter(f => f.endsWith(".html")))
  cpSync(join(root, "docs/prototipos", f), join(site, "prototipos", f));

console.log(`_site/ built: ${files.length} files kept offline, version ${version}`);
