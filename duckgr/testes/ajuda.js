// Utilidades para os testes: montar mundos pequenos e procurar células.
import * as N from "../web/plataforma/nucleo.js";
import terraViva from "../web/jogos/terra-viva/pacote.js";

export { N };
export const C = N.compilar(terraViva);

// Um jogador de teste: aplica comandos guardando o registro, como a interface faz.
export function partida(cfg = {}) {
  const S = N.novoEstado(C, cfg), cmds = [];
  return {
    S, cmds, cfg,
    cmd(c) { c.t = S.tick; c.autor = "teste"; const r = N.aplicar(S, C, c); if (r.ok) cmds.push(c); return r; },
    segundos(s) { for (let k = 0; k < s * N.TPS; k++) N.passo(S, C); },
    // Dá uma ordem ao jogador e roda o mundo até a fila esvaziar.
    // Devolve { ok, avisos } com os avisos que o jogador deu no caminho.
    fazer(o, maxSeg = 120) {
      S.eventos = [];
      const r = this.cmd({ tipo: "ordem", ...o });
      if (!r.ok) return { ok: false, msg: r.msg };
      for (let k = 0; k < maxSeg * N.TPS && S.jog.fila.length; k++) N.passo(S, C);
      const avisos = S.eventos.filter(e => e.tipo === "aviso").map(e => e.msg);
      return { ok: !S.jog.fila.length && !avisos.length, avisos };
    },
  };
}

export function acharCelula(S, teste) {
  for (let y = 0; y < S.A; y++) for (let x = 0; x < S.L; x++) if (teste(N.idx(S, x, y), x, y)) return { x, y };
  return null;
}

// Depósito de um tipo com uma célula livre à direita (para encostar uma máquina).
export function depositoComVizinhoLivre(S, tipo) {
  return acharCelula(S, (i, x) => S.dep[i] === tipo && x + 1 < S.L && !S.dep[i + 1] && S.sub[i + 1] === "solo"
    && (!S.cov[i + 1] || C.coberturas[S.cov[i + 1]].substituivel));
}
