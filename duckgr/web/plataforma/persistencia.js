// Persistência: como uma partida vira um arquivo e volta.
//
// Formato "checkpoint + comandos" (ver docs/DESIGN.md, "Como a partida é salva"):
// - checkpoint: o estado inteiro do mundo num tick (o "status atual");
// - historico: todos os comandos desde o tick 0. Ao carregar, só os que vieram
//   depois do checkpoint são refeitos; o resto serve para rever e conferir a partida.
// Tudo vai em JSON comprimido com gzip. Carregar custa refazer só alguns
// minutos de jogo, e o arquivo fica na casa de poucos KB.

import { novoEstado, aplicar, passo, hashEstado, TPS } from "./nucleo.js";

export const FORMATO = "duckgr-save";
export const VERSAO_FORMATO = 1;
export const CHECKPOINT_TICKS = 5 * 60 * TPS; // um checkpoint novo a cada 5 minutos de jogo

// O estado do mundo, pronto para virar JSON (sem o que é só da interface).
export function instantaneo(S) {
  return JSON.parse(JSON.stringify(S, (k, v) => (k === "eventos" ? undefined : v)));
}
export function restaurar(inst) {
  const S = structuredClone(inst);
  S.eventos = [];
  return S;
}

// Refaz os comandos a partir de um estado, até o tick pedido.
export function avancar(S, C, comandos, ateTick) {
  let k = 0;
  while (k < comandos.length && comandos[k].t < S.tick) k++; // já incluídos no estado
  const doTick = () => { while (k < comandos.length && comandos[k].t === S.tick) aplicar(S, C, comandos[k++]); };
  while (S.tick < ateTick) { doTick(); passo(S, C); }
  doTick();
  S.eventos = [];
  return S;
}

// Monta o objeto do save. partida = { pacote, cfg, cmds (histórico), checkpoint? }
export function montarSave(C, partida, S) {
  const cp = partida.checkpoint || { tick: 0, estado: null };
  return {
    formato: FORMATO, versao: VERSAO_FORMATO,
    pacote: { id: C.id, versao: C.versao },
    cfg: partida.cfg,
    tick: S.tick,
    hash: hashEstado(S),
    checkpoint: cp,
    historico: partida.cmds, // os comandos depois do checkpoint saem daqui
  };
}

// Recria o mundo a partir de um save; confere o hash no fim.
export function carregarSave(C, save) {
  if (save?.formato !== FORMATO) throw new Error("Este arquivo não é um save do DuckGR.");
  if (save.versao > VERSAO_FORMATO) throw new Error("Save de uma versão mais nova da plataforma.");
  if (save.pacote.id !== C.id) throw new Error(`Save de outro mundo (${save.pacote.id}).`);
  if (save.pacote.versao !== C.versao) throw new Error(`Save da versão ${save.pacote.versao} do mundo; esta é a ${C.versao}.`);
  const S = save.checkpoint.estado ? restaurar(save.checkpoint.estado) : novoEstado(C, save.cfg);
  avancar(S, C, save.historico.filter(c => c.t >= save.checkpoint.tick), save.tick);
  const ok = hashEstado(S) === save.hash;
  return { S, ok };
}

// Tira um checkpoint novo quando o último ficou velho. Devolve o checkpoint atual.
export function talvezCheckpoint(partida, S) {
  const cp = partida.checkpoint;
  if (S.tick - (cp?.tick || 0) >= CHECKPOINT_TICKS) partida.checkpoint = { tick: S.tick, estado: instantaneo(S) };
  return partida.checkpoint;
}

// ---------- compressão (gzip nativo do navegador e do Node) e texto base64
const temGzip = typeof CompressionStream !== "undefined";
export async function comprimir(texto) {
  if (!temGzip) return new TextEncoder().encode(texto);
  const s = new Blob([texto]).stream().pipeThrough(new CompressionStream("gzip"));
  return new Uint8Array(await new Response(s).arrayBuffer());
}
export async function descomprimir(bytes) {
  const gz = bytes[0] === 0x1f && bytes[1] === 0x8b;
  if (!gz) return new TextDecoder().decode(bytes);
  const s = new Blob([bytes]).stream().pipeThrough(new DecompressionStream("gzip"));
  return await new Response(s).text();
}
export function paraBase64(bytes) {
  let s = "";
  for (let i = 0; i < bytes.length; i += 0x8000) s += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
  return btoa(s);
}
export function deBase64(txt) {
  const bin = atob(txt.replace(/\s+/g, ""));
  const out = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
  return out;
}
