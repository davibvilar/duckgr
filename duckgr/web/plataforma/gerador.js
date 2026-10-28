// Gerador de mapa da plataforma. Recebe as regras de geração do pacote
// (rios, manchas de cobertura, depósitos) e devolve as camadas iniciais.
// Determinístico: só usa o sorteio semeado.

import { sortear } from "./sorteio.js";

export function gerarMapa(C, cfg) {
  const L = cfg.largura, A = cfg.altura, G = C.mapa.geracao;
  const r = { rng: Math.imul(cfg.semente | 0, 0x9e3779b1) >>> 0 || 7 };
  const entre = (a, b) => a + (sortear(r) % (b - a + 1));
  const n = L * A;
  const sub = new Array(n).fill(G.substrato);
  const cov = new Array(n).fill(G.cobertura);
  const dep = new Array(n).fill(null);
  const depQtd = new Array(n).fill(0);
  const i = (x, y) => x + y * L;
  const dentro = (x, y) => x >= 0 && y >= 0 && x < L && y < A;
  const circulo = (cx, cy, raio, f) => {
    for (let y = cy - raio; y <= cy + raio; y++) for (let x = cx - raio; x <= cx + raio; x++)
      if (dentro(x, y) && (x - cx) ** 2 + (y - cy) ** 2 <= raio * raio + raio) f(x, y);
  };

  // rios: uma faixa que serpenteia de cima a baixo
  for (const rio of G.rios || []) {
    let x = Math.floor(L * rio.posicao);
    for (let y = 0; y < A; y++) {
      x = Math.max(1, Math.min(L - rio.largura - 1, x + entre(-1, 1)));
      for (let k = 0; k < rio.largura; k++) { sub[i(x + k, y)] = rio.substrato; cov[i(x + k, y)] = null; }
    }
  }
  const agua = (x, y) => C.substratos[sub[i(x, y)]].plantar === false && !C.substratos[sub[i(x, y)]].construir;

  for (const m of G.manchas) {
    if (m.camada === "cobertura") {
      const tipos = Object.entries(m.tipos), soma = tipos.reduce((a, [, p]) => a + p, 0);
      const qtd = Math.round(n * m.porArea * (cfg[m.escalaPor] ?? 1));
      for (let k = 0; k < qtd; k++) {
        const cx = entre(0, L - 1), cy = entre(0, A - 1);
        circulo(cx, cy, entre(m.raio[0], m.raio[1]), (x, y) => {
          if (agua(x, y) || sortear(r) % 100 >= m.preenche) return;
          let s = sortear(r) % soma;
          for (const [t, p] of tipos) { if (s < p) { cov[i(x, y)] = t; break; } s -= p; }
        });
      }
    } else if (m.camada === "deposito") {
      const cx = Math.floor(L * m.em[0]) + entre(-2, 2), cy = Math.floor(A * m.em[1]) + entre(-1, 1);
      circulo(cx, cy, m.raio, (x, y) => {
        if (agua(x, y)) return;
        dep[i(x, y)] = m.tipo; depQtd[i(x, y)] = entre(m.qtd[0], m.qtd[1]);
        cov[i(x, y)] = G.cobertura;
      });
    }
  }
  return { L, A, sub, cov, dep, depQtd };
}
