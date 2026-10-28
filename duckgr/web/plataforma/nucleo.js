// Núcleo da simulação do DuckGR (protótipo em JavaScript).
//
// O núcleo não sabe que existe floresta, fornalha ou poluição. Ele conhece:
// células com camadas (substrato, depósito, cobertura, ocupante), elementos
// feitos de componentes (energia, extração, processo, entrega, emissão),
// campos por chunk, eras, tecnologias, missões e comandos.
// Todo o conteúdo vem do pacote do mundo (ver web/jogos/).
//
// Regras de determinismo: tick fixo, só aritmética inteira no estado,
// sorteios pelo gerador do próprio estado, nada de relógio de parede.
// Mesmo pacote + mesma configuração + mesmos comandos = mesmo hash.

import { gerarMapa } from "./gerador.js";
import { sortear } from "./sorteio.js";

export { sortear };

export const TPS = 30; // ticks por segundo
const VIZINHOS = [[1, 0], [0, 1], [-1, 0], [0, -1]];

// ---------- compilação do pacote: valida referências e converte segundos em ticks
export function compilar(pacote) {
  const erros = [];
  const C = structuredClone(pacote);
  const existe = (tabela, id, onde) => {
    if (id != null && !C[tabela][id]) erros.push(`${onde}: "${id}" não existe em ${tabela}`);
  };
  for (const [id, d] of Object.entries(C.depositos)) existe("itens", d.item, `depósito ${id}`);
  for (const [id, c] of Object.entries(C.coberturas)) {
    if (c.cresce) existe("coberturas", c.cresce.para, `cobertura ${id}`);
    if (c.regride !== undefined) existe("coberturas", c.regride, `cobertura ${id}`);
    if (c.colheita) {
      existe("coberturas", c.colheita.vira, `cobertura ${id}`);
      for (const it of Object.keys(c.colheita.da)) existe("itens", it, `colheita de ${id}`);
    }
  }
  for (const [id, it] of Object.entries(C.itens)) if (it.planta) existe("coberturas", it.planta, `item ${id}`);
  for (const [id, r] of Object.entries(C.receitas)) {
    for (const it of [...Object.keys(r.entra), ...Object.keys(r.sai)]) existe("itens", it, `receita ${id}`);
    r.id = id;
    r.ticks = Math.max(1, Math.round((r.segundos || 0) * TPS));
  }
  for (const [id, e] of Object.entries(C.elementos)) {
    for (const it of Object.keys(e.custo)) existe("itens", it, `custo de ${id}`);
    for (const r of e.processo || []) existe("receitas", r, `elemento ${id}`);
    for (const f of Object.keys(e.emite || {})) existe("campos", f, `elemento ${id}`);
    if (e.extrai) e.extrai.ticks = Math.max(1, Math.round(e.extrai.segundos * TPS));
  }
  for (const [id, t] of Object.entries(C.tecnologias)) {
    for (const it of Object.keys(t.custo)) existe("itens", it, `tecnologia ${id}`);
    for (const el of t.libera || []) existe("elementos", el, `tecnologia ${id}`);
    for (const r of t.requer || []) existe("tecnologias", r, `tecnologia ${id}`);
  }
  for (const el of C.inicio.liberados) existe("elementos", el, "início");
  for (const it of Object.keys(C.inicio.inventario)) existe("itens", it, "início");
  if (erros.length) throw new Error(`Pacote ${pacote.id} inválido:\n- ${erros.join("\n- ")}`);
  return C;
}

// ---------- estado inicial
export function novoEstado(C, cfg = {}) {
  const config = { ...C.padrao, ...cfg };
  const mapa = gerarMapa(C, config);
  const { L, A } = mapa;
  const ch = C.mapa.chunk;
  const CL = Math.ceil(L / ch), CA = Math.ceil(A / ch);
  const inv = {};
  const mult = config.recursosIniciais ?? 1;
  for (const [k, v] of Object.entries(C.inicio.inventario)) inv[k] = Math.round(v * mult);
  const S = {
    tick: 0,
    rng: (config.semente >>> 0) || 1,
    L, A, CL, CA,
    sub: mapa.sub, dep: mapa.dep, depQtd: mapa.depQtd, cov: mapa.cov,
    cres: new Array(L * A).fill(0),
    occ: new Array(L * A).fill(0),
    ents: [], prox: 1,
    inv, prod: {},
    campos: Object.fromEntries(Object.keys(C.campos).map(k => [k, new Array(CL * CA).fill(0)])),
    techs: [], era: 1,
    liberados: [...C.inicio.liberados],
    log: [], ultimoDano: -1e9,
    missao: { estado: "andamento", metas: config.metas ?? C.missao.metas, ref: {} },
    eventos: [], // transitório: avisos para a interface, fora do hash
  };
  for (const m of S.missao.metas) if (m.tipo === "cobertura") S.missao.ref[m.marca] = contarMarca(S, C, m.marca);
  registrar(S, C.missao.abertura || "O mundo começou.");
  return S;
}

// ---------- utilidades
export const idx = (S, x, y) => x + y * S.L;
const dentro = (S, x, y) => x >= 0 && y >= 0 && x < S.L && y < S.A;
export const chunkDe = (S, C, x, y) => Math.floor(x / C.mapa.chunk) + Math.floor(y / C.mapa.chunk) * S.CL;
export const total = o => Object.values(o).reduce((a, b) => a + b, 0);
function por(o, k, n) { o[k] = (o[k] || 0) + n; if (o[k] <= 0) delete o[k]; }
export const nomeItem = (C, it) => C.itens[it]?.nome ?? it;
export function entidadeEm(S, x, y) {
  if (!dentro(S, x, y)) return null;
  const id = S.occ[idx(S, x, y)];
  return id ? S.ents.find(e => e.id === id) : null;
}
function vizinhoTem(S, C, x, y, marca) {
  return VIZINHOS.some(([dx, dy]) => dentro(S, x + dx, y + dy) && (C.substratos[S.sub[idx(S, x + dx, y + dy)]].marcas || []).includes(marca));
}
export function contarMarca(S, C, marca) {
  let n = 0;
  for (const c of S.cov) if (c && (C.coberturas[c].marcas || []).includes(marca)) n++;
  return n;
}
function registrar(S, txt, cls = "") {
  S.log.unshift({ t: S.tick, txt, cls });
  if (S.log.length > 80) S.log.pop();
}

export function hashEstado(S) {
  const txt = JSON.stringify(S, (k, v) => (k === "eventos" ? undefined : v));
  let h = 0x811c9dc5;
  for (let i = 0; i < txt.length; i++) { h ^= txt.charCodeAt(i); h = Math.imul(h, 0x01000193); }
  return (h >>> 0).toString(16).padStart(8, "0");
}

// ---------- passo da simulação (um tick)
export function passo(S, C) {
  for (const e of S.ents) passoElemento(S, C, e);
  S.tick++;
  if (S.tick % TPS === 0) porSegundo(S, C);
}

function energia(S, C, e, pr) {
  if (!pr.energia) return true;
  const g = pr.energia.consumo;
  if (e.queima < g) {
    const comb = Object.keys(e.comb)[0];
    if (!comb) { e.parado = "Sem combustível"; return false; }
    por(e.comb, comb, -1);
    e.queima += C.itens[comb].combustivel;
  }
  e.queima -= g;
  return true;
}

// Em que compartimento o elemento aceita receber este item?
export function aceita(C, e, item) {
  const pr = C.elementos[e.p], inv = pr.inventario || {};
  if (pr.processo && pr.processo.some(r => C.receitas[r].entra[item]) && total(e.ent) < inv.entrada) return "ent";
  if (pr.energia && C.itens[item].combustivel && total(e.comb) < inv.combustivel) return "comb";
  if (inv.guarda && total(e.sai) < inv.guarda) return "sai";
  return null;
}

function passoElemento(S, C, e) {
  const pr = C.elementos[e.p];
  e.parado = "";
  // componente "entrega": passa um item por tick para um vizinho que aceite
  if (pr.entrega) {
    const item = Object.keys(e.sai)[0];
    if (item) for (const [dx, dy] of VIZINHOS) {
      const v = entidadeEm(S, e.x + dx, e.y + dy);
      const slot = v && aceita(C, v, item);
      if (slot) { por(e.sai, item, -1); por(v[slot], item, 1); break; }
    }
  }
  // componente "extrai": tira itens do depósito sob o elemento
  if (pr.extrai) {
    const i = idx(S, e.x, e.y), dep = S.dep[i];
    if (!dep) e.parado = "Depósito esgotado";
    else if (total(e.sai) >= pr.inventario.saida) e.parado = "Saída cheia";
    else if (energia(S, C, e, pr)) {
      e.ativo++;
      if (++e.prog >= pr.extrai.ticks) {
        e.prog = 0;
        const item = C.depositos[dep].item;
        por(e.sai, item, 1); por(S.prod, item, 1);
        if (--S.depQtd[i] <= 0) {
          S.dep[i] = null; S.depQtd[i] = 0;
          registrar(S, `Um depósito de ${C.depositos[dep].nome.toLowerCase()} se esgotou em (${e.x},${e.y}).`);
        }
      }
    }
  }
  // componente "processo": transforma entradas em saídas segundo receitas
  if (pr.processo) {
    if (!e.rec) {
      const cand = pr.processo.map(r => C.receitas[r]).filter(r => Object.entries(r.entra).every(([it, n]) => (e.ent[it] || 0) >= n));
      const r = cand[0];
      if (!r) e.parado = "Sem entrada";
      else if (r.exige?.vizinho && !vizinhoTem(S, C, e.x, e.y, r.exige.vizinho)) e.parado = r.exige.aviso || "Condição do ambiente não atendida";
      else if (total(e.sai) + total(r.sai) > pr.inventario.saida) e.parado = "Saída cheia";
      else {
        for (const [it, n] of Object.entries(r.entra)) por(e.ent, it, -n);
        e.rec = r.id; e.prog = 0;
      }
    }
    if (e.rec && energia(S, C, e, pr)) {
      e.ativo++;
      const r = C.receitas[e.rec];
      if (++e.prog >= r.ticks) {
        for (const [it, n] of Object.entries(r.sai)) { por(e.sai, it, n); por(S.prod, it, n); }
        e.rec = null; e.prog = 0;
      }
    }
  }
}

// ---------- o que acontece uma vez por segundo: campos, crescimento, missão
function porSegundo(S, C) {
  const ch = C.mapa.chunk;
  for (const [fid, F] of Object.entries(C.campos)) {
    const v = S.campos[fid];
    // fontes: elementos que trabalharam neste segundo
    for (const e of S.ents) {
      const em = C.elementos[e.p].emite?.[fid];
      if (em) v[chunkDe(S, C, e.x, e.y)] += Math.floor((em * e.ativo) / TPS);
    }
    // sumidouros: coberturas que absorvem
    const abs = new Array(v.length).fill(0);
    for (let y = 0; y < S.A; y++) for (let x = 0; x < S.L; x++) {
      const c = S.cov[idx(S, x, y)];
      const a = c && C.coberturas[c].absorve?.[fid];
      if (a) abs[chunkDe(S, C, x, y)] += a;
    }
    for (let k = 0; k < v.length; k++) v[k] = Math.max(0, v[k] - abs[k]);
    // difusão em duas fases: calcula tudo, depois aplica
    const delta = new Array(v.length).fill(0);
    for (let cy = 0; cy < S.CA; cy++) for (let cx = 0; cx < S.CL; cx++) {
      const k = cx + cy * S.CL;
      if (v[k] <= F.difusao.limiar) continue;
      const parte = Math.floor(v[k] / F.difusao.divisor);
      for (const [dx, dy] of VIZINHOS) {
        const nx = cx + dx, ny = cy + dy;
        if (nx < 0 || ny < 0 || nx >= S.CL || ny >= S.CA) continue;
        delta[k] -= parte; delta[nx + ny * S.CL] += parte;
      }
    }
    for (let k = 0; k < v.length; k++) v[k] += delta[k];
    // dano: acima do limiar, a cobertura regride
    if (F.dano) for (let k = 0; k < v.length; k++) {
      if (v[k] <= F.dano.limiar || sortear(S) % F.dano.chance !== 0) continue;
      const cx = k % S.CL, cy = Math.floor(k / S.CL), alvos = [];
      for (let y = cy * ch; y < Math.min(S.A, cy * ch + ch); y++) for (let x = cx * ch; x < Math.min(S.L, cx * ch + ch); x++) {
        const c = S.cov[idx(S, x, y)];
        if (c && C.coberturas[c].regride !== undefined) alvos.push(idx(S, x, y));
      }
      if (!alvos.length) continue;
      const i = alvos[sortear(S) % alvos.length];
      S.cov[i] = C.coberturas[S.cov[i]].regride; S.cres[i] = 0;
      if (S.tick - S.ultimoDano > 15 * TPS) {
        S.ultimoDano = S.tick;
        registrar(S, F.dano.aviso, "ruim");
      }
    }
  }
  for (let y = 0; y < S.A; y++) for (let x = 0; x < S.L; x++) {
    const i = idx(S, x, y), c = S.cov[i];
    const cr = c && C.coberturas[c].cresce;
    if (!cr) continue;
    const k = chunkDe(S, C, x, y);
    if (Object.entries(C.campos).some(([fid, F]) => F.inibeCrescimento && S.campos[fid][k] > F.inibeCrescimento)) continue;
    S.cres[i] += cr.bonusVizinho && vizinhoTem(S, C, x, y, cr.bonusVizinho) ? 2 : 1;
    if (S.cres[i] >= cr.segundos) { S.cov[i] = cr.para; S.cres[i] = 0; }
  }
  for (const e of S.ents) e.ativo = 0;
  avaliarMissao(S, C);
}

export function progressoMissao(S, C) {
  return S.missao.metas.map(m => {
    if (m.tipo === "producao") {
      const feito = S.prod[m.item] || 0;
      return { ...m, feito, ok: feito >= m.quantidade };
    }
    const atual = contarMarca(S, C, m.marca), ref = S.missao.ref[m.marca] || 1;
    const pct = Math.floor((atual * 100) / ref);
    return { ...m, atual, pct, ok: pct >= m.minimoPct };
  });
}

function avaliarMissao(S, C) {
  if (S.missao.estado !== "andamento") return;
  const p = progressoMissao(S, C);
  const falha = p.find(m => m.tipo === "cobertura" && !m.ok);
  if (falha) {
    S.missao.estado = "falhou";
    registrar(S, `Missão perdida: ${falha.nome.toLowerCase()} caiu abaixo de ${falha.minimoPct}%.`, "ruim");
    S.eventos.push({ tipo: "missao", estado: "falhou", meta: falha });
  } else if (p.every(m => m.ok)) {
    S.missao.estado = "cumprida";
    registrar(S, `Missão cumprida: ${C.missao.nome}!`, "bom");
    S.eventos.push({ tipo: "missao", estado: "cumprida" });
  }
}

// ---------- comandos: a única forma de mudar o mundo
// Um comando falho não muda nada e devolve a razão.
export function aplicar(S, C, cmd) {
  const f = COMANDOS[cmd.tipo];
  if (!f) return { ok: false, msg: `Comando desconhecido: ${cmd.tipo}` };
  return f(S, C, cmd);
}

function temItens(S, custo) { return Object.entries(custo).every(([it, n]) => (S.inv[it] || 0) >= n); }
function faltam(S, C, custo) {
  return Object.entries(custo).filter(([it, n]) => (S.inv[it] || 0) < n)
    .map(([it, n]) => `${n - (S.inv[it] || 0)} ${nomeItem(C, it).toLowerCase()}`).join(", ");
}

const COMANDOS = {
  // mão: recolhe/abastece máquina, colhe cobertura ou extrai do depósito
  usar(S, C, { x, y, segurando }) {
    if (!dentro(S, x, y)) return { ok: false };
    const i = idx(S, x, y), e = entidadeEm(S, x, y);
    if (e) {
      const pr = C.elementos[e.p], feito = [];
      const recolhido = total(e.sai);
      if (recolhido) { for (const [it, n] of Object.entries(e.sai)) por(S.inv, it, n); e.sai = {}; feito.push(`recolheu ${recolhido}`); }
      const colocar = (it, slot, ate) => {
        const n = Math.min(S.inv[it] || 0, ate - total(e[slot]));
        if (n > 0) { por(S.inv, it, -n); por(e[slot], it, n); feito.push(`colocou ${n} ${nomeItem(C, it).toLowerCase()}`); }
      };
      const inv = pr.inventario || {};
      if (segurando && S.inv[segurando]) {
        const slot = aceita(C, e, segurando);
        if (slot) colocar(segurando, slot, slot === "ent" ? inv.entrada : slot === "comb" ? inv.combustivel : inv.guarda);
      } else {
        if (pr.energia && total(e.comb) < 5) {
          const comb = Object.keys(C.itens).filter(it => C.itens[it].combustivel && S.inv[it])
            .sort((a, b) => C.itens[b].combustivel - C.itens[a].combustivel)[0];
          if (comb) colocar(comb, "comb", 5);
        }
        const atual = Object.keys(e.ent)[0];
        if (atual) colocar(atual, "ent", Math.min(10, inv.entrada));
      }
      if (!feito.length) return { ok: false, msg: pr.processo && !total(e.ent) ? "Segure um item do inventário (toque nele) para abastecer esta máquina." : `${pr.nome}: nada para recolher.` };
      return { ok: true, msg: `${pr.nome}: ${feito.join(", ")}.` };
    }
    const c = S.cov[i], col = c && C.coberturas[c].colheita;
    if (col) {
      for (const [it, n] of Object.entries(col.da)) por(S.inv, it, n);
      S.cov[i] = col.vira; S.cres[i] = 0;
      return { ok: true, msg: `+ ${Object.entries(col.da).map(([it, n]) => `${n} ${nomeItem(C, it).toLowerCase()}`).join(", ")}` };
    }
    const d = S.dep[i];
    if (d) {
      const it = C.depositos[d].item;
      por(S.inv, it, 1); por(S.prod, it, 1);
      if (--S.depQtd[i] <= 0) { S.dep[i] = null; S.depQtd[i] = 0; }
      return { ok: true, msg: `+ 1 ${nomeItem(C, it).toLowerCase()}` };
    }
    return { ok: false, msg: "Nada para pegar aqui." };
  },

  plantar(S, C, { x, y, item }) {
    if (!dentro(S, x, y)) return { ok: false };
    const it = C.itens[item], i = idx(S, x, y);
    if (!it?.planta) return { ok: false, msg: "Isso não se planta." };
    if (!S.inv[item]) return { ok: false, msg: `Você não tem ${it.nome.toLowerCase()}.` };
    if (S.occ[i]) return { ok: false, msg: "Tem uma máquina aqui." };
    if (!C.substratos[S.sub[i]].plantar) return { ok: false, msg: `Não dá para plantar em ${C.substratos[S.sub[i]].nome.toLowerCase()}.` };
    const c = S.cov[i];
    if (c && !C.coberturas[c].substituivel) return { ok: false, msg: "Já tem algo crescendo aqui." };
    por(S.inv, item, -1);
    S.cov[i] = it.planta; S.cres[i] = 0;
    return { ok: true };
  },

  construir(S, C, { x, y, elemento }) {
    if (!dentro(S, x, y)) return { ok: false };
    const pr = C.elementos[elemento], i = idx(S, x, y);
    if (!pr || !S.liberados.includes(elemento)) return { ok: false, msg: "Ainda não pesquisado." };
    if (S.occ[i]) return { ok: false, msg: "Já tem uma máquina aqui." };
    if (!C.substratos[S.sub[i]].construir) return { ok: false, msg: `Não dá para construir em ${C.substratos[S.sub[i]].nome.toLowerCase()}.` };
    const c = S.cov[i];
    if (c && !C.coberturas[c].substituivel) return { ok: false, msg: `Primeiro tire ${C.coberturas[c].nome.toLowerCase()} daqui.` };
    if (pr.exige === "deposito" && !S.dep[i]) return { ok: false, msg: `${pr.nome} precisa ficar em cima de um depósito.` };
    if (!temItens(S, pr.custo)) return { ok: false, msg: `Faltam ${faltam(S, C, pr.custo)}.` };
    for (const [it, n] of Object.entries(pr.custo)) por(S.inv, it, -n);
    const e = { id: S.prox++, p: elemento, x, y, ent: {}, sai: {}, comb: {}, queima: 0, prog: 0, rec: null, ativo: 0, parado: "" };
    S.ents.push(e); S.occ[i] = e.id;
    if (c) S.cov[i] = null;
    registrar(S, `Construiu ${pr.nome.toLowerCase()} em (${x},${y}).`);
    return { ok: true };
  },

  desmontar(S, C, { x, y }) {
    const e = entidadeEm(S, x, y);
    if (!e) return { ok: false, msg: "Não há máquina aqui." };
    const pr = C.elementos[e.p];
    for (const o of [pr.custo, e.ent, e.sai, e.comb]) for (const [it, n] of Object.entries(o)) por(S.inv, it, n);
    S.ents = S.ents.filter(o => o !== e);
    S.occ[idx(S, x, y)] = 0;
    registrar(S, `Desmontou ${pr.nome.toLowerCase()} em (${x},${y}).`);
    return { ok: true };
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

// ---------- replay: recria o mundo a partir da configuração e do registro de comandos
export function rodar(C, cfg, comandos, ateTick) {
  const S = novoEstado(C, cfg);
  let k = 0;
  const aplicarDoTick = () => { while (k < comandos.length && comandos[k].t === S.tick) aplicar(S, C, comandos[k++]); };
  while (S.tick < ateTick) { aplicarDoTick(); passo(S, C); }
  aplicarDoTick();
  S.eventos = [];
  return S;
}
