// O jogador: um agente com posição, velocidade e alcance. Ele só age no que
// está ao alcance; para o resto, anda até lá. O jogador recebe ordens
// (comando "ordem") que entram numa fila; cada ordem é "vá até perto desta
// célula e faça esta ação". Andar, esperar e agir acontecem tick a tick,
// então tudo continua determinístico e entra no replay.
//
// O núcleo não sabe o que é árvore ou minério: as ações são genéricas
// (colher cobertura, minerar depósito, plantar item, construir elemento,
// recolher, abastecer, fornecer, desmontar) e os nomes vêm do pacote.

import {
  TPS, idx, total, por, nomeItem, entidadeEm, aceita, registrar, temItens, faltam,
} from "./nucleo.js";

const VIZ = [[1, 0], [0, 1], [-1, 0], [0, -1]];
const dentro = (S, x, y) => x >= 0 && y >= 0 && x < S.L && y < S.A;
const minusc = s => s.charAt(0).toLowerCase() + s.slice(1);
const listaItens = (C, o) => Object.entries(o).map(([it, n]) => `${n} ${minusc(nomeItem(C, it))}`).join(", ");

// ---------- passagem: quanto custa (em múltiplos do passo) entrar numa célula; -1 bloqueia
export function custoPassagem(S, C, x, y) {
  if (!dentro(S, x, y)) return -1;
  const i = idx(S, x, y), cls = C.jogador.passagem;
  if (S.occ[i]) return -1;
  let custo = 1;
  for (const def of [C.substratos[S.sub[i]], S.cov[i] && C.coberturas[S.cov[i]]]) {
    const p = def?.passagem?.[cls];
    if (p === "bloqueia") return -1;
    if (typeof p === "number") custo *= p;
  }
  return custo;
}

// Posição inicial: a célula transitável mais perto do centro do mapa.
export function posicaoInicial(S, C) {
  const cx = Math.floor(S.L / 2), cy = Math.floor(S.A / 2);
  for (let r = 0; r < Math.max(S.L, S.A); r++)
    for (let dy = -r; dy <= r; dy++) for (let dx = -r; dx <= r; dx++) {
      if (Math.max(Math.abs(dx), Math.abs(dy)) !== r) continue;
      const x = cx + dx, y = cy + dy;
      if (custoPassagem(S, C, x, y) === 1) return { x, y };
    }
  return { x: 0, y: 0 };
}

// A ordem pode ser feita de onde o jogador está?
export function alcanca(S, C, J, o) {
  if (o.acao === "ir") return J.x === o.x && J.y === o.y;
  if (o.acao === "construir" && J.x === o.x && J.y === o.y) return false; // não constrói onde está pisando
  return Math.max(Math.abs(J.x - o.x), Math.abs(J.y - o.y)) <= C.jogador.alcance;
}

// Caminho mais barato (Dijkstra com baldes; vizinhos em ordem fixa = determinístico)
// até qualquer célula de onde a ordem seja alcançável. Devolve a lista de
// índices a percorrer (sem a célula atual), [] se já está lá, null se não há caminho.
export function caminho(S, C, J, o) {
  const ok = (x, y) => alcanca(S, C, { x, y }, o)
    || (o.acao === "ir" && custoPassagem(S, C, o.x, o.y) < 0 && Math.max(Math.abs(x - o.x), Math.abs(y - o.y)) <= 1);
  if (ok(J.x, J.y)) return [];
  const n = S.L * S.A, dist = new Array(n).fill(Infinity), de = new Array(n).fill(-1);
  const ini = idx(S, J.x, J.y);
  dist[ini] = 0;
  const baldes = [[ini]];
  for (let d = 0; d < baldes.length; d++) {
    const b = baldes[d];
    if (!b) continue;
    for (let k = 0; k < b.length; k++) {
      const i = b[k];
      if (dist[i] !== d) continue;
      const x = i % S.L, y = Math.floor(i / S.L);
      if (i !== ini && ok(x, y)) {
        const rota = [];
        for (let c = i; c !== ini; c = de[c]) rota.push(c);
        return rota.reverse();
      }
      for (const [dx, dy] of VIZ) {
        const c = custoPassagem(S, C, x + dx, y + dy);
        if (c < 0) continue;
        const j = idx(S, x + dx, y + dy), nd = d + c;
        if (nd < dist[j]) { dist[j] = nd; de[j] = i; (baldes[nd] ||= []).push(j); }
      }
    }
  }
  return null;
}

// ---------- ações: validar (sem olhar o alcance) e executar
const ACOES = {
  ir: { validar: () => null, executar: () => ({ ok: true }) },

  colher: {
    validar(S, C, o) { const c = S.cov[idx(S, o.x, o.y)]; return c && C.coberturas[c].colheita ? null : "Não há o que colher aqui."; },
    executar(S, C, o) {
      const i = idx(S, o.x, o.y), col = C.coberturas[S.cov[i]].colheita;
      for (const [it, n] of Object.entries(col.da)) por(S.inv, it, n);
      S.cov[i] = col.vira; S.cres[i] = 0;
      return { ok: true, ganho: col.da };
    },
  },

  minerar: {
    validar(S, C, o) { const i = idx(S, o.x, o.y); return S.dep[i] && !S.occ[i] ? null : S.occ[i] ? "Tem uma máquina em cima." : "Não há depósito aqui."; },
    executar(S, C, o) {
      const i = idx(S, o.x, o.y), d = S.dep[i], it = C.depositos[d].item;
      por(S.inv, it, 1); por(S.prod, it, 1);
      if (--S.depQtd[i] <= 0) { S.dep[i] = null; S.depQtd[i] = 0; }
      return { ok: true, ganho: { [it]: 1 } };
    },
  },

  plantar: {
    validar(S, C, o) {
      const it = C.itens[o.item], i = idx(S, o.x, o.y);
      if (!it?.planta) return "Isso não se planta.";
      if (!S.inv[o.item]) return `Você não tem ${minusc(it.nome)}.`;
      if (S.occ[i]) return "Tem uma máquina aqui.";
      if (!C.substratos[S.sub[i]].plantar) return `Não dá para plantar em ${minusc(C.substratos[S.sub[i]].nome)}.`;
      const c = S.cov[i];
      if (c && !C.coberturas[c].substituivel) return "Já tem algo crescendo aqui.";
      return null;
    },
    executar(S, C, o) {
      const i = idx(S, o.x, o.y);
      por(S.inv, o.item, -1);
      S.cov[i] = C.itens[o.item].planta; S.cres[i] = 0;
      return { ok: true };
    },
  },

  construir: {
    validar(S, C, o) {
      const pr = C.elementos[o.elemento], i = idx(S, o.x, o.y);
      if (!pr || !S.liberados.includes(o.elemento)) return "Ainda não pesquisado.";
      if (S.occ[i]) return "Já tem uma máquina aqui.";
      if (!C.substratos[S.sub[i]].construir) return `Não dá para construir em ${minusc(C.substratos[S.sub[i]].nome)}.`;
      const c = S.cov[i];
      if (c && !C.coberturas[c].substituivel) return `Primeiro tire ${minusc(C.coberturas[c].nome)} daqui.`;
      if (pr.exige === "deposito" && !S.dep[i]) return `${pr.nome} precisa ficar em cima de um depósito.`;
      if (!temItens(S, pr.custo)) return `Faltam ${faltam(S, C, pr.custo)}.`;
      return null;
    },
    executar(S, C, o) {
      const pr = C.elementos[o.elemento], i = idx(S, o.x, o.y);
      for (const [it, n] of Object.entries(pr.custo)) por(S.inv, it, -n);
      const e = { id: S.prox++, p: o.elemento, x: o.x, y: o.y, ent: {}, sai: {}, comb: {}, queima: 0, prog: 0, rec: null, ativo: 0, parado: "" };
      S.ents.push(e); S.occ[i] = e.id;
      if (S.cov[i]) S.cov[i] = null;
      registrar(S, `Construiu ${minusc(pr.nome)} em (${o.x},${o.y}).`);
      return { ok: true };
    },
  },

  desmontar: {
    validar(S, C, o) { return entidadeEm(S, o.x, o.y) ? null : "Não há máquina aqui."; },
    executar(S, C, o) {
      const e = entidadeEm(S, o.x, o.y), pr = C.elementos[e.p], ganho = {};
      for (const src of [pr.custo, e.ent, e.sai, e.comb]) for (const [it, n] of Object.entries(src)) { por(S.inv, it, n); por(ganho, it, n); }
      S.ents = S.ents.filter(x => x !== e);
      S.occ[idx(S, o.x, o.y)] = 0;
      registrar(S, `Desmontou ${minusc(pr.nome)} em (${o.x},${o.y}).`);
      return { ok: true, ganho };
    },
  },

  recolher: {
    validar(S, C, o) { const e = entidadeEm(S, o.x, o.y); return !e ? "Não há máquina aqui." : total(e.sai) ? null : "Nada para recolher."; },
    executar(S, C, o) {
      const e = entidadeEm(S, o.x, o.y), ganho = { ...e.sai };
      for (const [it, n] of Object.entries(e.sai)) por(S.inv, it, n);
      e.sai = {};
      return { ok: true, ganho };
    },
  },

  // abastecer: combustível; fornecer: entrada de receita
  abastecer: colocarEm("comb"),
  fornecer: colocarEm("ent"),
};

function colocarEm(slot) {
  const capDe = pr => (slot === "comb" ? pr.inventario.combustivel : pr.inventario.entrada);
  return {
    validar(S, C, o) {
      const e = entidadeEm(S, o.x, o.y);
      if (!e) return "Não há máquina aqui.";
      if (!S.inv[o.item]) return `Você não tem ${minusc(nomeItem(C, o.item))}.`;
      if (aceita(C, e, o.item) !== slot) return `${C.elementos[e.p].nome} não aceita ${minusc(nomeItem(C, o.item))} agora.`;
      return null;
    },
    executar(S, C, o) {
      const e = entidadeEm(S, o.x, o.y), pr = C.elementos[e.p];
      const n = Math.min(S.inv[o.item], capDe(pr) - total(e[slot]));
      por(S.inv, o.item, -n); por(e[slot], o.item, n);
      return { ok: true, perda: { [o.item]: n } };
    },
  };
}

export const NOMES_ACOES = {
  ir: "Ir até aqui", colher: "Colher", minerar: "Minerar", plantar: "Plantar", construir: "Construir",
  desmontar: "Desmontar", recolher: "Recolher", abastecer: "Abastecer", fornecer: "Fornecer",
};

// ---------- o que dá para fazer numa célula, já com rótulos (para o menu do tabuleiro)
export function acoesPossiveis(S, C, x, y) {
  if (!dentro(S, x, y)) return { acoes: [], notas: [] };
  const i = idx(S, x, y), lista = [], notas = [];
  const tenta = (o, rotulo, detalhe = "") => { if (!ACOES[o.acao].validar(S, C, { x, y, ...o })) lista.push({ x, y, ...o, rotulo, detalhe }); };
  const e = entidadeEm(S, x, y);
  if (e) {
    const pr = C.elementos[e.p];
    if (total(e.sai)) tenta({ acao: "recolher" }, `Recolher ${listaItens(C, e.sai)}`);
    if (pr.energia) {
      const livre = pr.inventario.combustivel - total(e.comb);
      const comb = Object.keys(C.itens).filter(it => C.itens[it].combustivel && S.inv[it])
        .sort((a, b) => C.itens[b].combustivel - C.itens[a].combustivel)[0];
      if (!total(e.comb) && !e.queima) notas.push({ tipo: "ruim", txt: "Sem combustível" });
      else if (livre > 0) notas.push({ tipo: "", txt: `Cabe mais ${livre} de combustível` });
      if (comb && livre > 0) tenta({ acao: "abastecer", item: comb }, `Abastecer com ${Math.min(livre, S.inv[comb])} ${minusc(nomeItem(C, comb))}`);
    }
    if (pr.processo) {
      const livre = pr.inventario.entrada - total(e.ent);
      const atual = Object.keys(e.ent)[0];
      const ent = [atual, ...Object.keys(C.itens).filter(it => S.inv[it] && pr.processo.some(r => C.receitas[r].entra[it])).sort((a, b) => S.inv[b] - S.inv[a])]
        .find(it => it && S.inv[it]);
      if (!total(e.ent) && !e.rec) notas.push({ tipo: "ruim", txt: "Sem entrada" });
      if (ent && livre > 0) tenta({ acao: "fornecer", item: ent }, `Fornecer ${Math.min(livre, S.inv[ent])} ${minusc(nomeItem(C, ent))}`);
    }
    if (e.parado && e.parado !== "Sem combustível" && e.parado !== "Sem entrada") notas.push({ tipo: "ruim", txt: e.parado });
    tenta({ acao: "desmontar" }, `Desmontar ${minusc(pr.nome)}`, "devolve o custo e o conteúdo");
    return { acoes: lista, notas, titulo: pr.nome };
  }
  const c = S.cov[i], d = S.dep[i];
  if (c && C.coberturas[c].colheita) {
    const col = C.coberturas[c].colheita;
    tenta({ acao: "colher" }, `${col.verbo || "Colher"} ${minusc(C.coberturas[c].nome)}`, `+ ${listaItens(C, col.da)}`);
  }
  if (d) {
    const vezes = Math.min(C.jogador.minerarVezes || 1, S.depQtd[i]);
    tenta({ acao: "minerar", vezes }, `Minerar ${minusc(C.depositos[d].nome)} ×${vezes}`, `restam ${S.depQtd[i]}`);
  }
  const plantavel = Object.keys(C.itens).find(it => C.itens[it].planta && S.inv[it]);
  if (plantavel) tenta({ acao: "plantar", item: plantavel }, `Plantar ${minusc(nomeItem(C, plantavel))}`, `você tem ${S.inv[plantavel]}`);
  for (const el of Object.keys(C.elementos)) if (S.liberados.includes(el))
    tenta({ acao: "construir", elemento: el }, `Construir ${minusc(C.elementos[el].nome)}`, listaItens(C, C.elementos[el].custo));
  if (custoPassagem(S, C, x, y) > 0) tenta({ acao: "ir" }, "Ir até aqui");
  const titulo = c ? C.coberturas[c].nome : d ? C.depositos[d].nome : C.substratos[S.sub[i]].nome;
  return { acoes: lista, notas, titulo };
}

// ---------- um tick do jogador
export function passoJogador(S, C) {
  const J = S.jog, o = J.fila[0];
  if (!o) return;
  if (!alcanca(S, C, J, o)) {
    J.tarefa = 0;
    let prox = J.cam[0];
    let custo = prox === undefined ? -1 : custoPassagem(S, C, prox % S.L, Math.floor(prox / S.L));
    if (custo < 0) { // sem caminho ainda, ou o caminho foi bloqueado: calcula de novo
      const rota = caminho(S, C, J, o);
      if (!rota || !rota.length) return falhar(S, "Não há caminho até lá.");
      J.cam = rota; J.passo = 0;
      prox = rota[0]; custo = custoPassagem(S, C, prox % S.L, Math.floor(prox / S.L));
    }
    if (++J.passo >= C.jogador.ticksPorCelula * custo) {
      J.passo = 0; J.cam.shift();
      J.x = prox % S.L; J.y = Math.floor(prox / S.L);
    }
    return;
  }
  J.cam = []; J.passo = 0;
  const erro = ACOES[o.acao].validar(S, C, o);
  if (erro) return falhar(S, erro);
  if (++J.tarefa < (C.jogador.duracoes[o.acao] || 1)) return;
  J.tarefa = 0;
  const r = ACOES[o.acao].executar(S, C, o);
  if (r.ganho || r.perda) S.eventos.push({ tipo: "ganho", x: J.x, y: J.y, ganho: r.ganho || {}, perda: r.perda || {} });
  o.feitas = (o.feitas || 0) + 1;
  if (o.feitas >= (o.vezes || 1)) J.fila.shift();
}

function falhar(S, msg) {
  S.jog.fila.shift(); S.jog.cam = []; S.jog.passo = 0; S.jog.tarefa = 0;
  S.eventos.push({ tipo: "aviso", msg });
}

// ---------- comandos: a única forma de mudar o mundo. Um comando falho não muda nada.
export const COMANDOS = {
  // Dá uma ordem ao jogador. Com adicionar, entra no fim da fila; sem, substitui a fila.
  ordem(S, C, { x, y, acao, item, elemento, vezes, adicionar }) {
    if (!ACOES[acao] || !dentro(S, x, y)) return { ok: false };
    const o = { x, y, acao };
    if (item) o.item = item;
    if (elemento) o.elemento = elemento;
    if (vezes > 1) o.vezes = Math.min(99, vezes | 0);
    const erro = ACOES[acao].validar(S, C, o);
    if (erro) return { ok: false, msg: erro };
    if (acao === "ir" && custoPassagem(S, C, x, y) < 0) return { ok: false, msg: "Não dá para pisar aí." };
    if (adicionar) { if (S.jog.fila.length >= 20) return { ok: false, msg: "A fila de tarefas está cheia." }; S.jog.fila.push(o); }
    else { S.jog.fila = [o]; S.jog.cam = []; S.jog.passo = 0; S.jog.tarefa = 0; }
    return { ok: true };
  },

  cancelar(S) {
    if (!S.jog.fila.length) return { ok: false };
    S.jog.fila = []; S.jog.cam = []; S.jog.passo = 0; S.jog.tarefa = 0;
    return { ok: true, msg: "Tarefas canceladas." };
  },

  fabricar(S, C, { receita }) {
    const r = C.receitas[receita];
    if (!r?.mao) return { ok: false, msg: "Isso não se faz à mão." };
    if (!temItens(S, r.entra)) return { ok: false, msg: `Faltam ${faltam(S, C, r.entra)}.` };
    for (const [it, n] of Object.entries(r.entra)) por(S.inv, it, -n);
    for (const [it, n] of Object.entries(r.sai)) { por(S.inv, it, n); por(S.prod, it, n); }
    return { ok: true };
  },

  pesquisar(S, C, { tecnologia }) {
    const t = C.tecnologias[tecnologia];
    if (!t || S.techs.includes(tecnologia)) return { ok: false };
    const falta = (t.requer || []).filter(r => !S.techs.includes(r));
    if (falta.length) return { ok: false, msg: `Antes pesquise: ${falta.map(r => C.tecnologias[r].nome).join(", ")}.` };
    if (!temItens(S, t.custo)) return { ok: false, msg: `Faltam ${faltam(S, C, t.custo)}.` };
    for (const [it, n] of Object.entries(t.custo)) por(S.inv, it, -n);
    S.techs.push(tecnologia);
    for (const el of t.libera || []) if (!S.liberados.includes(el)) S.liberados.push(el);
    registrar(S, `Pesquisa concluída: ${t.nome}.`, "bom");
    if (t.abreEra && t.abreEra > S.era) {
      S.era = t.abreEra;
      registrar(S, `Estamos entrando na era ${C.eras[t.abreEra - 1].nome}!`, "bom");
      S.eventos.push({ tipo: "era", era: t.abreEra });
    }
    return { ok: true };
  },
};

export const velocidadeCelulasPorSegundo = C => TPS / C.jogador.ticksPorCelula;
