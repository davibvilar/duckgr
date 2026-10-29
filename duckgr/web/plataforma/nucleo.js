// Ponte para o núcleo da simulação do DuckGR, escrito em Rust
// (crates/duckgr-sim) e compilado para WebAssembly (web/plataforma/duckgr.wasm).
//
// O núcleo não sabe que existe floresta, fornalha ou poluição: todo o
// conteúdo vem do pacote do mundo (ver web/jogos/). Ele é determinístico:
// mesmo pacote + mesma configuração + mesmos comandos = mesmo hash.
//
// Este arquivo mantém a interface que a plataforma já usava (compilar,
// novoEstado, passo, aplicar, hashEstado, rodar...). O estado mora dentro do
// módulo wasm; o objeto S que a interface recebe é uma janela para ele: lê
// os campos de sempre (S.tick, S.cov, S.jog, S.inv...), mas só muda por
// comandos e passos, nunca por atribuição.

export const TPS = 30; // ticks por segundo

const W = await carregar();

async function carregar() {
  const url = new URL("./duckgr.wasm", import.meta.url);
  let bytes;
  if (typeof process !== "undefined" && process.versions?.node) {
    const { readFile } = await import("node:fs/promises");
    bytes = await readFile(url).catch(() => {
      throw new Error("Falta web/plataforma/duckgr.wasm. Rode `npm run wasm` para compilar o núcleo.");
    });
  } else {
    const r = await fetch(url);
    if (!r.ok) throw new Error(`Não deu para baixar o núcleo do jogo (${r.status}).`);
    bytes = await r.arrayBuffer();
  }
  const { instance } = await WebAssembly.instantiate(bytes, {});
  return instance.exports;
}

// ---------- conversa com o módulo: texto JSON entra, texto JSON sai
const codificar = new TextEncoder(), decodificar = new TextDecoder();
function comTexto(txt, f) {
  const b = codificar.encode(txt), p = W.alocar(b.length);
  new Uint8Array(W.memory.buffer, p, b.length).set(b);
  try { return f(p, b.length); } finally { W.desalocar(p, b.length); }
}
const saida = () => decodificar.decode(new Uint8Array(W.memory.buffer, W.saida_ptr(), W.saida_len()));
function criar(f, dado) {
  const h = comTexto(JSON.stringify(dado), f);
  if (h < 0) throw new Error(saida());
  return h;
}

// ---------- pacote
const numeros = new WeakMap(); // pacote compilado → número dele dentro do módulo

// Valida as referências do pacote e converte segundos em ticks. Devolve os
// dados convertidos (a interface usa nomes, cores e descrições daqui).
export function compilar(pacote) {
  const h = criar(W.pacote_compilar, pacote);
  const C = JSON.parse(saida());
  numeros.set(C, h);
  return C;
}
function numeroDe(C) {
  const h = numeros.get(C);
  if (h === undefined) throw new Error("Use o pacote devolvido por compilar().");
  return h;
}

// ---------- estado
const soltar = new FinalizationRegistry(h => W.mundo_liberar(h));
const CAMPOS = ["rng", "L", "A", "CL", "CA", "sub", "dep", "depQtd", "cov", "cres", "occ", "ents", "prox",
  "inv", "prod", "campos", "techs", "era", "liberados", "log", "ultimoDano", "missao", "jog"];

class Estado {
  constructor(h) {
    this._h = h;
    this._vista = null;
    this.eventos = []; // avisos para a interface, fora do hash
    soltar.register(this, h, this);
  }
  get tick() { return W.mundo_tick(this._h); }
  // o estado em JSON, guardado até o próximo passo ou comando
  vista() { return (this._vista ||= this._op({ op: "estado" })); }
  _op(op) {
    const r = comTexto(JSON.stringify(op), (p, n) => W.mundo_op(this._h, p, n));
    const txt = saida();
    if (r < 0) throw new Error(txt);
    return JSON.parse(txt);
  }
  _mudou(eventos = 1) {
    this._vista = null;
    if (eventos) this.eventos.push(...this._op({ op: "eventos" }));
  }
  _trocar(h) {
    soltar.unregister(this);
    W.mundo_liberar(this._h);
    this._h = h;
    soltar.register(this, h, this);
    this._mudou(0);
  }
}
for (const k of CAMPOS) Object.defineProperty(Estado.prototype, k, { get() { return this.vista()[k]; }, enumerable: true });

export function novoEstado(C, cfg = {}) {
  return new Estado(criar((p, n) => W.mundo_novo(numeroDe(C), p, n), cfg));
}

// O estado do mundo, pronto para virar JSON (o checkpoint do save).
export function instantaneo(S) { return S._op({ op: "estado" }); }
export function restaurar(C, inst) {
  return new Estado(criar((p, n) => W.mundo_restaurar(numeroDe(C), p, n), inst));
}

// Muda o estado à mão, por fora dos comandos. Só para testes e ferramentas:
// num jogo, todo efeito no mundo nasce de um comando.
export function editar(S, C, f) {
  const inst = instantaneo(S);
  f(inst);
  S._trocar(criar((p, n) => W.mundo_restaurar(numeroDe(C), p, n), inst));
  return S;
}

// ---------- passo da simulação
export function passo(S, C, n = 1) {
  const eventos = W.mundo_passo(S._h, n);
  S._mudou(eventos);
}

// ---------- comandos: a única forma de mudar o mundo
export function aplicar(S, C, cmd) {
  const r = S._op({ op: "aplicar", cmd });
  S._mudou();
  return r;
}

export function hashEstado(S) { return S._op({ op: "hash" }); }
export function progressoMissao(S) { return S._op({ op: "progresso" }); }
export function contarMarca(S, C, marca) { return S._op({ op: "contarMarca", marca }); }

// ---------- replay: recria o mundo a partir da configuração e do registro de comandos
export function rodar(C, cfg, cmds, ateTick) {
  return new Estado(criar((p, n) => W.mundo_rodar(numeroDe(C), p, n), { cfg, cmds, ate: ateTick }));
}
// Refaz os comandos a partir de um estado, até o tick pedido.
export function avancar(S, C, cmds, ateTick) {
  S._op({ op: "avancar", cmds, ate: ateTick });
  S._mudou(0);
  S.eventos = [];
  return S;
}

// ---------- o jogador
// O que dá para fazer numa célula, já com rótulos (para o menu do tabuleiro).
export function acoesPossiveis(S, C, x, y) { return S._op({ op: "acoes", x, y }); }
// Quanto custa (em múltiplos do passo) entrar numa célula; -1 bloqueia.
export function custoPassagem(S, C, x, y) { return W.mundo_custo_passagem(S._h, x, y); }
// Caminho até de onde a ordem pode ser feita: índices das células, [] se já está lá, null se não há.
export function caminho(S, C, J, o) { return S._op({ op: "caminho", x: J.x, y: J.y, ordem: o }); }
export const velocidadeCelulasPorSegundo = C => TPS / C.jogador.ticksPorCelula;

// ---------- utilidades de leitura (para a interface)
export const idx = (S, x, y) => x + y * S.L;
export const chunkDe = (S, C, x, y) => Math.floor(x / C.mapa.chunk) + Math.floor(y / C.mapa.chunk) * S.CL;
export const total = o => Object.values(o).reduce((a, b) => a + b, 0);
export const nomeItem = (C, it) => C.itens[it]?.nome ?? it;
export function entidadeEm(S, x, y) {
  if (x < 0 || y < 0 || x >= S.L || y >= S.A) return null;
  const id = S.occ[idx(S, x, y)];
  return id ? S.ents.find(e => e.id === id) : null;
}
