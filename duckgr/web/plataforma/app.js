// Interface da plataforma: telas (jogo, menu, configurações, montagem),
// laço de ticks, entrada do jogador e salvamento. Genérica: tudo o que é
// específico do mundo vem do pacote compilado (C).

import * as N from "./nucleo.js";
import { desenharMundo } from "./desenho.js";
import { acoesPossiveis, velocidadeCelulasPorSegundo } from "./jogador.js";
import { PACOTES, PACOTE_PADRAO, EM_BREVE } from "../jogos/catalogo.js";

const $ = s => document.querySelector(s);
const esc = s => String(s).replace(/[&<>"]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);
const ler = (k, d) => { try { const v = localStorage.getItem(k); return v ? JSON.parse(v) : d; } catch { return d; } };
const gravar = (k, v) => { try { localStorage.setItem(k, JSON.stringify(v)); } catch { /* sem armazenamento: segue sem salvar */ } };
const apagar = k => { try { localStorage.removeItem(k); } catch { /* idem */ } };
const CHAVE_PARTIDA = "duckgr:partida", CHAVE_PREFS = "duckgr:prefs";

const PREFS_PADRAO = { velocidade: 1, zoom: "auto", dicas: true };
let prefs = { ...PREFS_PADRAO, ...ler(CHAVE_PREFS, {}) };

const compilados = {};
const pacote = id => (compilados[id] ||= N.compilar(PACOTES.find(p => p.id === id) || PACOTES[0]));

// estado da partida e da interface
let P = null, C = null, S = null;
let construindo = null, verCampo = false, invAberto = false, menuFixo = null;
const flutuantes = []; // textos "+1 madeira" que sobem acima do pato
let hover = null, celSel = null, pausado = false, tela = "jogo", hashVisto = "";

// ---------- partida
function iniciar(partida, ateTick = 0) {
  P = { pacote: partida.pacote, cfg: partida.cfg || {}, cmds: partida.cmds || [] };
  C = pacote(P.pacote);
  S = ateTick ? N.rodar(C, P.cfg, P.cmds, ateTick) : N.novoEstado(C, P.cfg);
  construindo = null; celSel = null; menuFixo = null; pausado = false;
  $("#nomeMundo").textContent = C.nome;
  document.title = `DuckGR · ${C.nome}`;
  dimensionar();
  renderTudo();
  salvar();
}
function novoPadrao() { iniciar({ pacote: PACOTE_PADRAO, cfg: {}, cmds: [] }); }
function salvar() { gravar(CHAVE_PARTIDA, { ...P, versao: C.versao, tick: S.tick }); }

function comando(cmd, silencioso = false) {
  cmd.t = S.tick; cmd.autor = "jogador";
  const r = N.aplicar(S, C, cmd);
  if (r.ok) { P.cmds.push(cmd); salvar(); }
  if (r.msg && (!silencioso || !r.ok)) aviso(r.msg);
  renderPaineis();
  return r;
}

// ---------- laço
let ultimo = performance.now(), acc = 0, ultimoPainel = 0, ultimoSave = 0;
function quadro(agora) {
  const dt = Math.min(250, agora - ultimo); ultimo = agora;
  if (tela === "jogo" && S) {
    if (!pausado && $("#alerta").hidden) {
      acc += dt * prefs.velocidade;
      const ms = 1000 / N.TPS;
      let n = 0;
      while (acc >= ms && n < 60) { N.passo(S, C); acc -= ms; n++; }
      if (n >= 60) acc = 0;
      if (S.eventos.length) tratarEventos();
    }
    desenhar(agora);
    if (agora - ultimoPainel > 250) { ultimoPainel = agora; renderPaineis(); }
    if (agora - ultimoSave > 5000) { ultimoSave = agora; salvar(); }
  }
  requestAnimationFrame(quadro);
}

function tratarEventos() {
  for (const ev of S.eventos.splice(0)) {
    if (ev.tipo === "aviso") aviso(ev.msg);
    else if (ev.tipo === "ganho") {
      const partes = [...Object.entries(ev.ganho).map(([it, n]) => [`+${n} ${nItem(it).toLowerCase()}`, "#f2c14e"]),
        ...Object.entries(ev.perda).map(([it, n]) => [`−${n} ${nItem(it).toLowerCase()}`, "#a9b6c6"])];
      partes.forEach(([txt, cor], k) => flutuantes.push({ x: ev.x, y: ev.y, txt, cor, t0: performance.now() + k * 180 }));
    } else if (ev.tipo === "era") {
      const era = C.eras[ev.era - 1];
      alerta(`Era ${ev.era}: ${era.nome}`, `<p>Estamos entrando na era <b>${esc(era.nome)}</b>.</p><p>${esc(era.resumo)}</p><p>Nesta primeira versão do jogo, a era ${ev.era} ainda está sendo construída. Você pode continuar jogando e cuidando da floresta.</p>`, "Seguir", "ouro");
    } else if (ev.tipo === "missao" && ev.estado === "cumprida") {
      alerta("Missão cumprida!", `<p>Você completou <b>${esc(C.missao.nome)}</b> sem derrubar a floresta. Produção e equilíbrio, os dois ao mesmo tempo.</p><p>Continue jogando: pesquise as tecnologias para chegar à próxima era.</p>`, "Continuar", "ouro");
    } else if (ev.tipo === "missao" && ev.estado === "falhou") {
      alerta("A floresta caiu demais", `<p>${esc(ev.meta.nome)} ficou abaixo de ${ev.meta.minimoPct}% do que havia no começo. A missão foi perdida.</p><p>Dá para continuar jogando e replantar, ou começar de novo pelo menu.</p>`, "Entendi", "ruim");
    }
  }
}

// ---------- desenho
const cv = $("#cv"), ctx = cv.getContext("2d");
let cs = 20;
function dimensionar() {
  if (!S) return;
  const w = $("#tabuleiro").clientWidth || 600;
  cs = prefs.zoom === "auto" ? Math.max(14, Math.floor(w / S.L)) : Number(prefs.zoom);
  const dpr = window.devicePixelRatio || 1;
  cv.width = S.L * cs * dpr; cv.height = S.A * cs * dpr;
  cv.style.width = S.L * cs + "px"; cv.style.height = S.A * cs + "px";
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
}
window.addEventListener("pagehide", () => { if (S) salvar(); });
document.addEventListener("visibilitychange", () => { if (S && document.hidden) salvar(); });
window.addEventListener("resize", () => { dimensionar(); if (tela === "montar") previa(); });

function desenhar(t) {
  desenharMundo(ctx, S, C, {
    cs, t, verCampo, hover,
    fantasma: construindo && hover && !S.occ[N.idx(S, hover.x, hover.y)] ? construindo : null,
    corCursor: construindo ? "#8fd16a" : "#ffffff",
  });
  // textos flutuantes de ganho
  ctx.font = `800 ${Math.max(12, cs * 0.55)}px Nunito, system-ui, sans-serif`; ctx.textAlign = "center";
  for (let k = flutuantes.length - 1; k >= 0; k--) {
    const f = flutuantes[k], dt = t - f.t0;
    if (dt > 1400) { flutuantes.splice(k, 1); continue; }
    if (dt < 0) continue;
    const a = 1 - dt / 1400;
    ctx.fillStyle = `rgba(0,0,0,${0.6 * a})`; ctx.fillText(f.txt, (f.x + 0.5) * cs + 1, (f.y - 0.1) * cs - dt / 40 + 1);
    ctx.globalAlpha = a; ctx.fillStyle = f.cor; ctx.fillText(f.txt, (f.x + 0.5) * cs, (f.y - 0.1) * cs - dt / 40); ctx.globalAlpha = 1;
  }
  ctx.textAlign = "start";
}

// ---------- entrada no tabuleiro
// Mouse: passar por cima mostra as ações da célula; clicar faz a ação padrão;
// teclas fazem a ação do atalho. Shift (ou o botão "+ fila") põe no fim da fila.
// Toque: tocar abre o menu da célula, e tocar numa ação a executa.
const TECLAS = { ir: "I", colher: "C", minerar: "M", plantar: "P", recolher: "R", abastecer: "A", fornecer: "F", desmontar: "D" };
const PADRAO = ["recolher", "abastecer", "fornecer", "colher", "minerar", "ir"];
const teclaDe = a => a.acao === "construir" ? String(construiveis().indexOf(a.elemento) + 1) : TECLAS[a.acao];
const construiveis = () => Object.keys(C.elementos).filter(el => S.liberados.includes(el));
function opcoes(c) {
  const r = acoesPossiveis(S, C, c.x, c.y);
  r.acoes.forEach(a => (a.tecla = teclaDe(a)));
  r.padrao = PADRAO.map(ac => r.acoes.find(a => a.acao === ac)).find(Boolean) || null;
  return r;
}
function ordenar(a, adicionar) {
  if (!a) return;
  const { x, y, acao, item, elemento, vezes } = a;
  comando({ tipo: "ordem", x, y, acao, item, elemento, vezes, adicionar: !!adicionar });
}

function celulaDe(e) {
  const r = cv.getBoundingClientRect();
  const x = Math.floor(((e.clientX - r.left) / r.width) * S.L), y = Math.floor(((e.clientY - r.top) / r.height) * S.A);
  return x < 0 || y < 0 || x >= S.L || y >= S.A ? null : { x, y };
}
cv.addEventListener("pointerdown", e => {
  const c = celulaDe(e); if (!c) return;
  celSel = c;
  if (construindo) { ordenar({ ...c, acao: "construir", elemento: construindo }, e.shiftKey); return; }
  if (e.pointerType === "mouse") { menuFixo = null; ordenar(opcoes(c).padrao, e.shiftKey); }
  else { hover = null; menuFixo = c; renderMenuCel(); }
});
cv.addEventListener("pointermove", e => {
  if (e.pointerType !== "mouse") return;
  const c = celulaDe(e);
  if (c && hover && c.x === hover.x && c.y === hover.y) return;
  hover = c; menuFixo = null; renderMenuCel(); renderCelula();
});
cv.addEventListener("pointerleave", e => {
  if (e.relatedTarget && $("#menuCel").contains(e.relatedTarget)) return;
  hover = null; renderMenuCel();
});
$("#menuCel").addEventListener("pointerleave", e => {
  if (e.relatedTarget === cv) return;
  hover = null; renderMenuCel();
});

document.addEventListener("keydown", e => {
  if (tela !== "jogo" || e.target.closest("input,select,textarea") || e.ctrlKey || e.metaKey || e.altKey) return;
  // dígitos pelo código da tecla, para Shift+2 continuar sendo "2" (e não "@")
  const k = /^Digit\d$/.test(e.code) ? e.code.slice(5) : e.key.length === 1 ? e.key.toUpperCase() : e.key;
  if (!$("#alerta").hidden) { if (k === "Escape") $("#alerta").hidden = true; return; }
  if (k === "E") { e.preventDefault(); return alternarInv(); }
  if (k === "Escape") {
    if (invAberto) return alternarInv(false);
    if (construindo) { construindo = null; return renderConstrucoes(); }
    if (menuFixo) { menuFixo = null; return renderMenuCel(); }
    return comando({ tipo: "cancelar" });
  }
  if (invAberto) return;
  if (k === " ") { e.preventDefault(); pausado = !pausado; return renderTopo(); }
  if (k === "V") return $("#verCampo").click();
  const c = hover || menuFixo;
  if (!c) return;
  const a = opcoes(c).acoes.find(a => a.tecla === k);
  if (a) { e.preventDefault(); ordenar(a, e.shiftKey); }
});

function renderMenuCel() {
  const el = $("#menuCel"), c = hover || menuFixo;
  if (!c || !S || invAberto) { el.hidden = true; return; }
  const o = opcoes(c);
  const linhas = o.acoes.map((a, k) => `<button class="ac ${a === o.padrao && !menuFixo ? "padrao" : ""}" data-ac="${k}"><span class="tecla">${esc(a.tecla)}</span><span>${esc(a.rotulo)}${a.detalhe ? `<small>${esc(a.detalhe)}</small>` : ""}</span></button>`).join("");
  const notas = o.notas.map(n => `<div class="nota ${n.tipo}">${esc(n.txt)}</div>`).join("");
  const dica = menuFixo ? "Toque numa ação. Ela entra no lugar das tarefas atuais." : "Clique: ação destacada · Tecla: atalho · Shift: põe na fila";
  html(el, `<div class="tit"><span>${esc(o.titulo || "")} <span class="small muted">(${c.x}, ${c.y})</span></span>${menuFixo ? `<button data-fecharmenu="1" aria-label="Fechar">×</button>` : ""}</div>${notas}${linhas || `<div class="nota">Nada a fazer aqui.</div>`}<div class="rodape">${dica}</div>`);
  el.__acoes = o.acoes;
  el.hidden = false;
  // no mouse o menu só informa (não pode tapar a célula vizinha); no toque ele é clicável
  el.classList.toggle("fixo", !!menuFixo);
  // posiciona ao lado da célula, sem sair da área visível do tabuleiro
  const b = $("#tabuleiro"), w = el.offsetWidth, h = el.offsetHeight;
  let x = (c.x + 1) * cs + 12, y = c.y * cs + cs;
  if (x + w > b.scrollLeft + b.clientWidth) x = c.x * cs - w - 8;
  x = Math.max(b.scrollLeft + 4, x);
  y = Math.max(b.scrollTop + 4, Math.min(y, b.scrollTop + b.clientHeight - h - 4));
  el.style.left = x + "px"; el.style.top = y + "px";
}

// ---------- painéis
function html(el, h) { if (el.__h !== h) { el.__h = h; el.innerHTML = h; } }
const nItem = it => N.nomeItem(C, it);
const lista = o => Object.entries(o).map(([it, n]) => `${n} ${esc(nItem(it).toLowerCase())}`).join(", ") || "vazio";
const relogio = tick => { const s = Math.floor(tick / N.TPS); return `${String(Math.floor(s / 60)).padStart(2, "0")}:${String(s % 60).padStart(2, "0")}`; };

function renderTudo() { renderTopo(); renderConstrucoes(); renderAtalhos(); renderPaineis(); }
function renderPaineis() {
  if (!S) return;
  renderMissao(); renderInventario(); renderPesquisa(); renderCelula();
  renderEras(); renderRegistro(); renderRodape(); renderConstrucoes(); renderTopo(); renderMenuCel();
  if (invAberto) { renderSlots(); renderFabricar(); }
}

function renderTopo() {
  html($("#topo"), tela === "jogo"
    ? `<span class="era-badge">Era ${S.era} · ${esc(C.eras[S.era - 1].nome)}</span><button class="btn" id="pausar">${pausado ? "▶ Continuar" : "⏸ Pausar"}</button><button class="btn" data-ir="menu">Menu</button>`
    : `<button class="btn prim" data-ir="jogo">Voltar ao jogo</button>`);
}

function renderMissao() {
  const metas = N.progressoMissao(S, C);
  const est = { andamento: "Em andamento", cumprida: "Cumprida!", falhou: "Perdida" }[S.missao.estado];
  let h = `<div class="nome">${esc(C.missao.nome)}<span>${est}</span></div>`;
  for (const m of metas) {
    if (m.tipo === "producao") {
      const pct = Math.min(100, (m.feito * 100) / m.quantidade);
      h += `<div class="meta"><div class="rot"><span>${esc(m.nome)}</span><span>${Math.min(m.feito, m.quantidade)} / ${m.quantidade}</span></div><div class="bar"><i style="width:${pct}%"></i></div></div>`;
    } else {
      const escala = 150, cls = m.pct < m.minimoPct ? "ruim" : m.pct < m.minimoPct + 15 ? "aviso" : "";
      h += `<div class="meta"><div class="rot"><span>${esc(m.nome)} (mínimo ${m.minimoPct}%)</span><span>${m.pct}%</span></div><div class="bar ${cls}"><i style="width:${Math.min(100, (m.pct * 100) / escala)}%"></i><b style="left:${(m.minimoPct * 100) / escala}%"></b></div></div>`;
    }
  }
  html($("#missao"), h);
}

function renderConstrucoes() {
  const lib = construiveis();
  html($("#construcoes"), Object.entries(C.elementos).map(([id, e]) => {
    const k = lib.indexOf(id);
    const custo = Object.entries(e.custo).map(([it, n]) => `${n} ${nItem(it).toLowerCase()}`).join(" + ");
    return `<button data-construir="${id}" class="${construindo === id ? "on" : ""}" ${k >= 0 ? "" : "disabled"} title="${esc(e.dica || "")} Custo: ${esc(custo)}">${k >= 0 ? `<span class="tecla">${k + 1}</span>` : ""}${esc(e.aparencia.simbolo)} ${esc(e.nome)}${k >= 0 ? "" : " 🔒"}</button>`;
  }).join(""));
  const n = S.jog.fila.length;
  html($("#fila"), n ? `<span>${n === 1 ? "1 tarefa" : `${n} tarefas`} na fila</span><button class="btn" id="cancelarFila">Parar <span class="tecla">Esc</span></button>` : "");
  let dica = "Passe o mouse no mapa para ver o que dá para fazer em cada célula. O pato anda até lá e faz.";
  if (construindo) { const e = C.elementos[construindo]; dica = `${esc(e.nome)}: ${esc(e.dica || "")} Clique onde construir; Shift para várias. Esc cancela.`; }
  else if (!matchMedia("(hover:hover)").matches) dica = "Toque numa célula para ver as ações. O pato anda até lá e faz.";
  html($("#dica"), prefs.dicas ? dica : "");
}

function renderInventario() {
  const itens = Object.keys(C.itens).filter(it => S.inv[it]);
  html($("#inventario"), itens.map(it => `<span class="chip"><span class="d" style="background:${C.itens[it].cor}"></span>${esc(C.itens[it].nome)} <span class="n">${S.inv[it]}</span></span>`).join("") || `<span class="muted small">Vazio. Corte árvores e minere.</span>`);
  const v = velocidadeCelulasPorSegundo(C);
  html($("#infoJogador"), `${esc(C.jogador.nome)} a pé: ${String(v).replace(".", ",")} células por segundo, alcance de ${C.jogador.alcance} célula em volta.`);
}

function renderSlots() {
  const itens = Object.keys(C.itens).filter(it => S.inv[it]);
  const vazios = Math.max(0, 20 - itens.length);
  html($("#slots"), itens.map(it => `<div class="slot" title="${esc(C.itens[it].nome)}"><span class="n">${S.inv[it]}</span><span class="d" style="background:${C.itens[it].cor}"></span><span class="nm">${esc(C.itens[it].nome)}</span></div>`).join("")
    + `<div class="slot vazio"></div>`.repeat(vazios));
  html($("#invJogador"), $("#infoJogador").innerHTML);
}

function alternarInv(abrir = !invAberto) {
  invAberto = abrir;
  $("#telaInv").hidden = !abrir;
  if (abrir) { renderSlots(); renderFabricar(); $("#fecharInv").focus(); }
  renderMenuCel();
}

function renderAtalhos() {
  const ac = Object.entries(TECLAS).map(([a, t]) => `<span class="tecla">${t}</span><span>${esc(NOMES[a])}</span>`).join("");
  html($("#atalhos"), ac + `<span class="tecla">1–9</span><span>Construir (na célula sob o mouse) ou escolher na barra</span><span class="tecla">Shift</span><span>Põe a ação no fim da fila</span><span class="tecla">E</span><span>Inventário e fabricação</span><span class="tecla">Esc</span><span>Parar tarefas / cancelar construção</span><span class="tecla">Espaço</span><span>Pausar</span><span class="tecla">V</span><span>Ver poluição</span>`);
}
const NOMES = { ir: "Ir até a célula", colher: "Cortar / colher", minerar: "Minerar", plantar: "Plantar muda", recolher: "Recolher da máquina", abastecer: "Abastecer com combustível", fornecer: "Fornecer entrada (minério, madeira)", desmontar: "Desmontar máquina" };

function renderFabricar() {
  html($("#fabricar"), Object.entries(C.receitas).filter(([, r]) => r.mao).map(([id, r]) => {
    const pode = n => Object.entries(r.entra).every(([it, q]) => (S.inv[it] || 0) >= q * n);
    return `<div class="linha"><div><b>${esc(r.nome)}</b><div class="receita">${lista(r.entra)} → ${lista(r.sai)}</div></div><div class="acoes"><button class="btn" data-fabricar="${id}" data-n="1" ${pode(1) ? "" : "disabled"}>+1</button><button class="btn" data-fabricar="${id}" data-n="5" ${pode(5) ? "" : "disabled"}>+5</button></div></div>`;
  }).join(""));
}

function renderPesquisa() {
  html($("#pesquisa"), Object.entries(C.tecnologias).map(([id, t]) => {
    const feita = S.techs.includes(id);
    const falta = (t.requer || []).filter(r => !S.techs.includes(r));
    const pode = !feita && !falta.length && Object.entries(t.custo).every(([it, n]) => (S.inv[it] || 0) >= n);
    const sub = feita ? "Pesquisada" : falta.length ? `Requer ${falta.map(r => esc(C.tecnologias[r].nome)).join(", ")}` : `${esc(t.descricao || "")} Custo: ${lista(t.custo)}`;
    return `<div class="tech ${feita ? "feita" : ""}"><div><b>${feita ? "✓ " : ""}${esc(t.nome)}</b><span>${sub}</span></div>${feita ? "" : `<button class="btn ${pode ? "prim" : ""}" data-pesquisar="${id}" ${pode ? "" : "disabled"}>Pesquisar</button>`}</div>`;
  }).join(""));
}

function renderCelula() {
  const c = hover || celSel;
  if (!c) { html($("#celula"), `<dt>Dica</dt><dd>Passe o mouse ou toque no mapa para ver as camadas de uma célula.</dd>`); $("#celTit").textContent = "Camadas da célula"; return; }
  $("#celTit").textContent = `Camadas da célula (${c.x}, ${c.y})`;
  const i = N.idx(S, c.x, c.y), cov = S.cov[i], dep = S.dep[i], e = N.entidadeEm(S, c.x, c.y);
  let h = `<dt>Substrato</dt><dd>${esc(C.substratos[S.sub[i]].nome)}</dd>`;
  h += `<dt>Depósito</dt><dd>${dep ? `${esc(C.depositos[dep].nome)} · restam ${S.depQtd[i]}` : "—"}</dd>`;
  let cob = "—";
  if (cov) {
    const d = C.coberturas[cov];
    cob = esc(d.nome) + (d.cresce ? ` · crescendo ${S.cres[i]}/${d.cresce.segundos}s` : "");
  }
  h += `<dt>Cobertura</dt><dd>${cob}</dd>`;
  if (e) {
    const pr = C.elementos[e.p];
    const r = e.rec && C.receitas[e.rec];
    const estado = e.parado ? `<span class="parado">${esc(e.parado)}</span>` : r ? `Fazendo ${esc(r.nome.toLowerCase())} ${Math.floor((e.prog * 100) / r.ticks)}%` : pr.extrai ? `Extraindo ${Math.floor((e.prog * 100) / pr.extrai.ticks)}%` : "Pronto";
    h += `<dt>Ocupante</dt><dd>${esc(pr.nome)} · ${estado}</dd>`;
    if (pr.energia) h += `<dt>Combustível</dt><dd>${lista(e.comb)}</dd>`;
    if (pr.processo) h += `<dt>Entrada</dt><dd>${lista(e.ent)}</dd>`;
    h += `<dt>${pr.inventario?.guarda ? "Guardado" : "Saída"}</dt><dd>${lista(e.sai)}</dd>`;
  } else h += `<dt>Ocupante</dt><dd>—</dd>`;
  const k = N.chunkDe(S, C, c.x, c.y);
  for (const [fid, F] of Object.entries(C.campos)) {
    const v = S.campos[fid][k];
    const nota = F.dano && v > F.dano.limiar ? " · árvores adoecendo" : F.inibeCrescimento && v > F.inibeCrescimento ? " · plantas não crescem" : "";
    h += `<dt>${esc(F.nome)}</dt><dd>${Math.floor(v / 100)} no trecho${nota}</dd>`;
  }
  html($("#celula"), h);
}

function renderEras() {
  html($("#eras"), C.eras.map((e, k) => `<div class="era ${k + 1 === S.era ? "atual" : k + 1 < S.era ? "passada" : ""}" title="${esc(e.resumo)}">${k + 1}. ${esc(e.nome)}${k + 1 > S.era ? " 🔒" : ""}</div>`).join(""));
}

function renderRegistro() {
  html($("#registro"), S.log.map(l => `<li class="${l.cls}"><span class="t">${relogio(l.t)}</span>${esc(l.txt)}</li>`).join(""));
}

function renderRodape() {
  if (S.tick % N.TPS === 0 || !hashVisto) hashVisto = N.hashEstado(S);
  html($("#rodape"), `<span>Tempo ${relogio(S.tick)} · tick ${S.tick}</span><span>${P.cmds.length} comandos</span><span>Hash do mundo <code>${hashVisto}</code></span><button class="linkbtn" id="conferir">Conferir replay</button><span>Pacote ${esc(C.id)} v${esc(C.versao)}</span>`);
}

// ---------- cliques nos painéis (delegação)
document.addEventListener("click", e => {
  const b = e.target.closest("button"); if (!b) return;
  const d = b.dataset;
  if (d.ir) return ir(d.ir);
  if (d.construir) { construindo = construindo === d.construir ? null : d.construir; return renderConstrucoes(); }
  if (d.ac) { const a = $("#menuCel").__acoes?.[Number(d.ac)]; menuFixo = null; ordenar(a, e.shiftKey); return renderMenuCel(); }
  if (d.fecharmenu) { menuFixo = null; return renderMenuCel(); }
  if (b.id === "cancelarFila") return comando({ tipo: "cancelar" });
  if (b.id === "abrirInv" || b.id === "abrirInv2") return alternarInv(true);
  if (b.id === "fecharInv") return alternarInv(false);
  if (d.fabricar) { for (let k = 0; k < Number(d.n); k++) if (!comando({ tipo: "fabricar", receita: d.fabricar }, k > 0).ok) break; return; }
  if (d.pesquisar) return comando({ tipo: "pesquisar", tecnologia: d.pesquisar });
  if (b.id === "pausar") { pausado = !pausado; return renderTopo(); }
  if (b.id === "conferir") return conferirReplay();
});
$("#verCampo").onclick = () => { verCampo = !verCampo; $("#verCampo").classList.toggle("on", verCampo); $("#verCampo").setAttribute("aria-pressed", verCampo); };
$("#novoPadrao").onclick = () => { novoPadrao(); ir("jogo"); aviso("Novo jogo no mundo padrão."); };
$("#comoJogar").onclick = () => { ir("jogo"); comoJogar(); };

function conferirReplay() {
  const t0 = performance.now();
  const S2 = N.rodar(C, P.cfg, P.cmds, S.tick);
  const a = N.hashEstado(S), b = N.hashEstado(S2), ms = Math.round(performance.now() - t0);
  alerta(a === b ? "Replay confere" : "Replay diferente!",
    `<p>O mundo foi refeito do zero a partir da configuração e dos ${P.cmds.length} comandos, até o tick ${S.tick}, em ${ms} ms.</p><p>Hash do jogo: <b>${a}</b><br>Hash do replay: <b>${b}</b></p><p>${a === b ? "Mesmos comandos, mesmo mundo. É isso que vai permitir salvar, rever partidas e jogar em grupo." : "Algo no núcleo não é determinístico. Isso é um bug: avise no repositório."}</p>`,
    "Ok", a === b ? "" : "ruim");
}

// ---------- telas
function ir(t) {
  tela = t;
  for (const [id, nome] of [["#telaJogo", "jogo"], ["#telaMenu", "menu"], ["#telaConfigurar", "configurar"], ["#telaMontar", "montar"]]) $(id).hidden = nome !== t;
  if (t === "configurar") renderPrefs();
  if (t === "montar") renderMontar();
  if (t === "jogo") { dimensionar(); renderPaineis(); }
  renderTopo();
  window.scrollTo(0, 0);
}

// Configurações (preferências deste aparelho; não mudam o mundo)
function renderPrefs() {
  const sel = (id, rot, ajuda, opcoes) => `<div class="campo"><label for="${id}">${rot}</label><select id="${id}">${opcoes.map(([v, t]) => `<option value="${v}" ${String(prefs[id]) === String(v) ? "selected" : ""}>${t}</option>`).join("")}</select><span class="ajuda">${ajuda}</span></div>`;
  $("#formPrefs").innerHTML =
    sel("velocidade", "Velocidade do tempo", "Acelera o relógio da tela. O mundo continua determinístico.", [[0.5, "Devagar (0,5×)"], [1, "Normal (1×)"], [2, "Rápida (2×)"], [4, "Muito rápida (4×)"]]) +
    sel("zoom", "Tamanho das células", "Automático cabe na largura da tela.", [["auto", "Automático"], [16, "Pequeno"], [22, "Médio"], [30, "Grande"]]) +
    sel("dicas", "Dicas das ferramentas", "Mostra a linha de ajuda acima do mapa.", [[true, "Mostrar"], [false, "Esconder"]]);
  for (const id of ["velocidade", "zoom", "dicas"]) $("#" + id).onchange = ev => {
    const v = ev.target.value;
    prefs[id] = id === "velocidade" ? Number(v) : id === "dicas" ? v === "true" : v;
    gravar(CHAVE_PREFS, prefs);
  };
}
$("#apagarSave").onclick = () => { apagar(CHAVE_PARTIDA); novoPadrao(); aviso("Partida apagada. Novo jogo no mundo padrão."); };

// Montar mundo: pacote, mapa, floresta, recursos e metas, com prévia
let montagem = null;
function renderMontar() {
  const Cm = pacote(montagem?.pacote || P.pacote);
  const prod = Cm.missao.metas.find(m => m.tipo === "producao"), cob = Cm.missao.metas.find(m => m.tipo === "cobertura");
  if (!montagem) montagem = { pacote: Cm.id, semente: Cm.padrao.semente, tamanho: "medio", floresta: 1, recursos: 1, meta: prod?.quantidade ?? 50, minimo: cob?.minimoPct ?? 50 };
  const m = montagem;
  const sel = (id, rot, ajuda, opcoes) => `<div class="campo"><label for="m_${id}">${rot}</label><select id="m_${id}">${opcoes.map(([v, t, off]) => `<option value="${v}" ${String(m[id]) === String(v) ? "selected" : ""} ${off ? "disabled" : ""}>${t}</option>`).join("")}</select><span class="ajuda">${ajuda}</span></div>`;
  $("#formMundo").innerHTML =
    sel("pacote", "Mundo", "Cada mundo é um pacote de dados.", [...PACOTES.map(p => [p.id, esc(p.nome)]), ...EM_BREVE.map(p => ["", `${esc(p.nome)} (em breve)`, true])]) +
    `<div class="campo"><label for="m_semente">Semente do mapa</label><div class="junto"><input id="m_semente" type="number" min="1" max="999999" value="${m.semente}"><button class="btn" id="m_sortear" title="Sortear outra semente">🎲</button></div><span class="ajuda">A mesma semente gera sempre o mesmo mapa.</span></div>` +
    sel("tamanho", "Tamanho do mapa", "Mapas maiores pedem mais exploração.", Object.entries(Cm.mapa.tamanhos).map(([k, t]) => [k, `${t.nome} (${t.largura}×${t.altura})`])) +
    sel("floresta", "Floresta inicial", "Mais floresta absorve mais poluição.", [[0.6, "Pouca"], [1, "Normal"], [1.5, "Muita"]]) +
    sel("recursos", "Recursos iniciais", "O que você tem no bolso ao começar.", [[0.5, "Poucos"], [1, "Normais"], [2, "Generosos"]]) +
    (prod ? sel("meta", `Meta: ${esc(prod.nome.toLowerCase())}`, "Quanto produzir para cumprir a missão.", [[20, "20"], [50, "50"], [100, "100"], [200, "200"]]) : "") +
    (cob ? sel("minimo", `Mínimo de ${esc(cob.nome.toLowerCase())}`, "Se cair abaixo disso, a missão é perdida.", [[30, "30%"], [50, "50%"], [70, "70%"], [90, "90%"]]) : "");
  for (const id of ["pacote", "semente", "tamanho", "floresta", "recursos", "meta", "minimo"]) {
    const el = $("#m_" + id); if (!el) continue;
    el.onchange = ev => { m[id] = id === "pacote" || id === "tamanho" ? ev.target.value : Number(ev.target.value) || 1; if (id === "pacote") renderMontar(); else previa(); };
  }
  $("#m_sortear").onclick = () => { m.semente = 1 + Math.floor(Math.random() * 999998); $("#m_semente").value = m.semente; previa(); };
  previa();
}
function cfgMontada() {
  const Cm = pacote(montagem.pacote), t = Cm.mapa.tamanhos[montagem.tamanho];
  const metas = Cm.missao.metas.map(x => x.tipo === "producao" ? { ...x, quantidade: montagem.meta } : x.tipo === "cobertura" ? { ...x, minimoPct: montagem.minimo } : x);
  return { semente: montagem.semente, largura: t.largura, altura: t.altura, floresta: montagem.floresta, recursosIniciais: montagem.recursos, metas };
}
function previa() {
  const Cm = pacote(montagem.pacote), cfg = cfgMontada(), Sp = N.novoEstado(Cm, cfg);
  const cvp = $("#cvPrevia"), w = $("#previa").clientWidth || 600, c = Math.max(8, Math.floor(w / Sp.L)), dpr = window.devicePixelRatio || 1;
  cvp.width = Sp.L * c * dpr; cvp.height = Sp.A * c * dpr; cvp.style.width = Sp.L * c + "px"; cvp.style.height = Sp.A * c + "px";
  const x = cvp.getContext("2d"); x.setTransform(dpr, 0, 0, dpr, 0, 0);
  desenharMundo(x, Sp, Cm, { cs: c, t: 0 });
  $("#previaInfo").textContent = `${Sp.L}×${Sp.A} células · ${N.contarMarca(Sp, Cm, "floresta")} árvores · inventário inicial: ${Object.entries(Sp.inv).map(([it, n]) => `${n} ${N.nomeItem(Cm, it).toLowerCase()}`).join(", ")}`;
}
$("#comecarMontado").onclick = () => { iniciar({ pacote: montagem.pacote, cfg: cfgMontada(), cmds: [] }); ir("jogo"); aviso("Mundo montado. Bom jogo!"); };

// ---------- avisos
let tAviso;
function aviso(m) { const t = $("#toast"); t.textContent = m; t.hidden = false; clearTimeout(tAviso); tAviso = setTimeout(() => (t.hidden = true), 2200); }
function alerta(tit, corpo, botao = "Entendi", tipo = "") {
  $("#alertaTit").textContent = tit; $("#alertaTxt").innerHTML = corpo; $("#alertaBtn").textContent = botao;
  $("#alertaBox").className = "box " + tipo; $("#alerta").hidden = false; $("#alertaBtn").focus();
}
$("#alertaBtn").onclick = () => { $("#alerta").hidden = true; };
function comoJogar() {
  alerta("Como jogar", `<ol>
    <li>Você é o <b>pato</b>. Ele só mexe no que está ao lado dele; para o resto, anda até lá.</li>
    <li>Passe o mouse numa célula para ver o que dá para fazer, com uma tecla para cada ação: <b>C</b> corta árvore, <b>M</b> minera, <b>R</b> recolhe, <b>A</b> abastece. Clicar faz a ação destacada. No celular, toque na célula.</li>
    <li>Com <b>Shift</b> a ação entra na fila: o pato faz uma depois da outra.</li>
    <li>Construa uma <b>mineradora</b> (tecla 1) em cima do ferro e uma <b>fornalha</b> (tecla 2) encostada nela: o minério passa sozinho para a fornalha.</li>
    <li>Máquinas poluem. A <b>floresta</b> absorve a poluição, mas se ela passar do limite as árvores adoecem. Plante mudas (<b>P</b>)!</li>
    <li>Missão: <b>${esc(C.missao.metas.find(m => m.tipo === "producao")?.nome || "produzir")}</b> sem deixar a floresta cair abaixo do mínimo. No inventário (<b>E</b>) você fabrica engrenagens e pacotes de ciência para pesquisar.</li></ol>`, "Começar");
  gravar("duckgr:viuComoJogar2", true);
}

// ---------- partida: entra direto num jogo, sem menu no caminho
const salvo = ler(CHAVE_PARTIDA, null);
try {
  // partidas de versões antigas do pacote não são refeitas: os comandos mudaram
  if (salvo && PACOTES.some(p => p.id === salvo.pacote) && salvo.versao === pacote(salvo.pacote).versao) iniciar(salvo, salvo.tick || 0);
  else novoPadrao();
} catch (err) {
  console.error("Partida salva não pôde ser carregada; começando de novo.", err);
  novoPadrao();
}
ir("jogo");
if (!ler("duckgr:viuComoJogar2", false)) comoJogar();
requestAnimationFrame(quadro);

// para depurar no console do navegador: duckgr.estado(), duckgr.pacote()
window.duckgr = { estado: () => S, pacote: () => C, partida: () => P };
