// Builds the Rust core to WebAssembly and puts the result in
// web/platform/duckgr.wasm, where core.js looks for it.
// Needs Rust with the wasm target: rustup target add wasm32-unknown-unknown
import { execFileSync } from "node:child_process";
import { copyFileSync, statSync } from "node:fs";

const root = new URL("..", import.meta.url);
execFileSync("cargo", ["build", "-p", "duckgr-wasm", "--release", "--target", "wasm32-unknown-unknown"], { cwd: root, stdio: "inherit" });
const target = new URL("web/platform/duckgr.wasm", root);
copyFileSync(new URL("target/wasm32-unknown-unknown/release/duckgr_wasm.wasm", root), target);
console.log(`web/platform/duckgr.wasm (${Math.round(statSync(target).size / 1024)} KB)`);
