// Compila o núcleo em Rust para WebAssembly e põe o resultado em
// web/plataforma/duckgr.wasm, onde o nucleo.js o procura.
// Precisa do Rust com o alvo wasm: rustup target add wasm32-unknown-unknown
import { execFileSync } from "node:child_process";
import { copyFileSync, statSync } from "node:fs";

const raiz = new URL("..", import.meta.url);
execFileSync("cargo", ["build", "-p", "duckgr-wasm", "--release", "--target", "wasm32-unknown-unknown"], { cwd: raiz, stdio: "inherit" });
const destino = new URL("web/plataforma/duckgr.wasm", raiz);
copyFileSync(new URL("target/wasm32-unknown-unknown/release/duckgr_wasm.wasm", raiz), destino);
console.log(`web/plataforma/duckgr.wasm (${Math.round(statSync(destino).size / 1024)} KB)`);
