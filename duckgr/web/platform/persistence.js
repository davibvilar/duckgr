// Persistence: how a game becomes a file and comes back.
//
// "Checkpoint + commands" format (see docs/DESIGN.md, "Como a partida é salva"):
// - checkpoint: the whole world state at one tick (the "current status");
// - historico: every command since tick 0. On load, only the ones after the
//   checkpoint are redone; the rest is there to review and check the game.
// Everything goes into gzip-compressed JSON. Loading costs redoing only a
// few minutes of play, and the file stays at a few KB.
//
// The save's keys (formato, versao, pacote, historico...) are the file
// format, shared with saves already out there, so they stay as they are.

import { newState, restore, advance, snapshot, stateHash, TPS } from "./core.js";

export const FORMAT = "duckgr-save";
export const FORMAT_VERSION = 1;
export const CHECKPOINT_TICKS = 5 * 60 * TPS; // a new checkpoint every 5 minutes of play

// The checkpoint is the world state in JSON, the same the JavaScript core
// wrote: saves made before the Rust core open the same way.
export { snapshot };

// A save that can't be opened. `key` and `args` say why, for the
// interface to show in the player's language.
export class SaveError extends Error {
  constructor(key, args = {}) {
    super(key);
    this.key = key;
    this.args = args;
  }
}

// Builds the save object. game = { pacote, cfg, cmds (history), checkpoint? }
export function buildSave(C, game, S) {
  const cp = game.checkpoint || { tick: 0, estado: null };
  return {
    formato: FORMAT, versao: FORMAT_VERSION,
    pacote: { id: C.id, versao: C.versao },
    cfg: game.cfg,
    tick: S.tick,
    hash: stateHash(S),
    checkpoint: cp,
    historico: game.cmds, // the commands after the checkpoint come from here
  };
}

// Rebuilds the world from a save; checks the hash at the end.
export function loadSave(C, save) {
  if (save?.formato !== FORMAT) throw new SaveError("save.error.notASave");
  if (save.versao > FORMAT_VERSION) throw new SaveError("save.error.newerPlatform");
  if (save.pacote.id !== C.id) throw new SaveError("save.error.otherWorld", { world: save.pacote.id });
  if (save.pacote.versao !== C.versao) throw new SaveError("save.error.otherVersion", { saved: save.pacote.versao, current: C.versao });
  const S = save.checkpoint.estado ? restore(C, save.checkpoint.estado) : newState(C, save.cfg);
  advance(S, C, save.historico.filter(c => c.t >= save.checkpoint.tick), save.tick);
  const ok = stateHash(S) === save.hash;
  return { S, ok };
}

// Takes a new checkpoint when the last one got old. Returns the current checkpoint.
export function maybeCheckpoint(game, S) {
  const cp = game.checkpoint;
  if (S.tick - (cp?.tick || 0) >= CHECKPOINT_TICKS) game.checkpoint = { tick: S.tick, estado: snapshot(S) };
  return game.checkpoint;
}

// ---------- compression (native gzip in browsers and Node) and base64 text
const hasGzip = typeof CompressionStream !== "undefined";
export async function compress(text) {
  if (!hasGzip) return new TextEncoder().encode(text);
  const s = new Blob([text]).stream().pipeThrough(new CompressionStream("gzip"));
  return new Uint8Array(await new Response(s).arrayBuffer());
}
export async function decompress(bytes) {
  const gz = bytes[0] === 0x1f && bytes[1] === 0x8b;
  if (!gz) return new TextDecoder().decode(bytes);
  const s = new Blob([bytes]).stream().pipeThrough(new DecompressionStream("gzip"));
  return await new Response(s).text();
}
export function toBase64(bytes) {
  let s = "";
  for (let i = 0; i < bytes.length; i += 0x8000) s += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
  return btoa(s);
}
export function fromBase64(txt) {
  const bin = atob(txt.replace(/\s+/g, ""));
  const out = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
  return out;
}
