// Desenho do mundo num canvas 2D. Genérico: a aparência de cada tipo vem
// do pacote (cor, forma, símbolo). O núcleo não sabe que este arquivo existe.

import { idx, total } from "./nucleo.js";

export function ruido(x, y) {
  const s = Math.sin(x * 12.9898 + y * 78.233) * 43758.5453;
  return s - Math.floor(s);
}
export function tom(hex, k) {
  const n = parseInt(hex.slice(1), 16);
  const f = v => Math.max(0, Math.min(255, Math.round(k < 0 ? v * (1 + k) : v + (255 - v) * k)));
  return `rgb(${f(n >> 16)},${f((n >> 8) & 255)},${f(n & 255)})`;
}

// op: { cs, t (tempo em ms, só para animação), hover, fantasma, campo, verCampo }
export function desenharMundo(ctx, S, C, op) {
  const cs = op.cs;
  for (let y = 0; y < S.A; y++) for (let x = 0; x < S.L; x++) {
    const i = idx(S, x, y), px = x * cs, py = y * cs;
    const sb = C.substratos[S.sub[i]].aparencia;
    const n = ruido(x, y);
    ctx.fillStyle = tom(sb.cor, (n - 0.5) * 0.12);
    ctx.fillRect(px, py, cs, cs);
    if (sb.forma === "agua") {
      ctx.fillStyle = "rgba(255,255,255,.18)";
      const o = ((op.t / 900 + n * 4) % 1) * cs;
      ctx.fillRect(px + o * 0.6, py + cs * 0.35, cs * 0.3, 1.5);
      ctx.fillRect(px + cs * 0.5 - o * 0.3, py + cs * 0.7, cs * 0.25, 1.5);
    }
    const c = S.cov[i];
    if (c) cobertura(ctx, C.coberturas[c].aparencia, px, py, cs, n, x, y, S.cres[i], C.coberturas[c]);
    const d = S.dep[i];
    if (d) pedrinhas(ctx, C.depositos[d].aparencia.cor, px, py, cs, x, y, S.depQtd[i]);
  }
  for (const e of S.ents) elemento(ctx, C, e, e.x * cs, e.y * cs, cs, op.t);

  // campos: névoa por chunk
  const ch = C.mapa.chunk;
  for (const [fid, F] of Object.entries(C.campos)) {
    const v = S.campos[fid];
    for (let k = 0; k < v.length; k++) {
      const cx = (k % S.CL) * ch * cs, cy = Math.floor(k / S.CL) * ch * cs;
      const a = Math.min(op.verCampo ? 0.6 : 0.4, v[k] / (F.dano?.limiar * 2.5 || 10000)) + (op.verCampo ? 0.05 : 0);
      if (a > 0.02) { ctx.fillStyle = `rgba(60,48,38,${a})`; ctx.fillRect(cx, cy, ch * cs, ch * cs); }
      if (op.verCampo) {
        ctx.strokeStyle = "rgba(255,255,255,.25)"; ctx.lineWidth = 1; ctx.setLineDash([3, 3]);
        ctx.strokeRect(cx + 0.5, cy + 0.5, ch * cs - 1, ch * cs - 1); ctx.setLineDash([]);
        ctx.font = `800 ${Math.max(11, cs * 0.6)}px Nunito, system-ui, sans-serif`;
        ctx.fillStyle = v[k] > F.dano.limiar ? "#ff9d8f" : v[k] > F.inibeCrescimento ? "#f2c14e" : "#f4efe3";
        ctx.fillText(String(Math.floor(v[k] / 100)), cx + 4, cy + Math.max(13, cs * 0.75));
      }
    }
  }

  if (op.hover) {
    const { x, y } = op.hover;
    ctx.strokeStyle = op.corCursor || "#ffffff"; ctx.lineWidth = 2;
    ctx.strokeRect(x * cs + 1, y * cs + 1, cs - 2, cs - 2);
    if (op.fantasma) {
      ctx.globalAlpha = 0.45;
      elemento(ctx, C, { p: op.fantasma, sai: {}, ent: {}, comb: {}, rec: null, parado: "" }, x * cs, y * cs, cs, 0);
      ctx.globalAlpha = 1;
    }
  }
}

function cobertura(ctx, ap, px, py, cs, n, x, y, cres, def) {
  const verde = "#5a9a4a";
  if (ap.forma !== "agua") {
    ctx.fillStyle = n < 0.5 ? verde : n < 0.85 ? "#539145" : "#62a352";
    ctx.fillRect(px, py, cs, cs);
  }
  if (ap.forma === "grama") {
    if (n > 0.9) { ctx.fillStyle = "#7dbb5f"; ctx.fillRect(px + cs * 0.3, py + cs * 0.4, cs * 0.1, cs * 0.25); ctx.fillRect(px + cs * 0.55, py + cs * 0.3, cs * 0.1, cs * 0.35); }
  } else if (ap.forma === "broto") {
    const g = def.cresce ? Math.min(1, cres / def.cresce.segundos) : 0;
    ctx.fillStyle = "#6b4a2b"; ctx.fillRect(px + cs * 0.3, py + cs * 0.68, cs * 0.4, cs * 0.12);
    ctx.fillStyle = ap.cor;
    const h = cs * (0.22 + 0.15 * g);
    ctx.fillRect(px + cs * 0.47, py + cs * 0.68 - h, cs * 0.07, h);
    ctx.beginPath(); ctx.ellipse(px + cs * 0.38, py + cs * 0.68 - h, cs * 0.12, cs * 0.07, -0.5, 0, 7); ctx.fill();
    ctx.beginPath(); ctx.ellipse(px + cs * 0.62, py + cs * 0.68 - h * 0.8, cs * 0.12, cs * 0.07, 0.5, 0, 7); ctx.fill();
  } else if (ap.forma === "arvore") {
    const r = cs * ap.tamanho, ox = (ruido(y, x) - 0.5) * cs * 0.12;
    ctx.fillStyle = "#5b3d22"; ctx.fillRect(px + cs * 0.46 + ox, py + cs * 0.55, cs * 0.08, cs * 0.35);
    ctx.fillStyle = "rgba(0,0,0,.2)"; ctx.beginPath(); ctx.arc(px + cs / 2 + ox + 1.5, py + cs * 0.48 + 1.5, r, 0, 7); ctx.fill();
    ctx.fillStyle = ap.cor; ctx.beginPath(); ctx.arc(px + cs / 2 + ox, py + cs * 0.48, r, 0, 7); ctx.fill();
    ctx.fillStyle = "rgba(255,255,255,.14)"; ctx.beginPath(); ctx.arc(px + cs / 2 + ox - r * 0.3, py + cs * 0.48 - r * 0.3, r * 0.45, 0, 7); ctx.fill();
  } else if (ap.forma === "toco") {
    ctx.fillStyle = ap.cor; ctx.beginPath(); ctx.ellipse(px + cs / 2, py + cs * 0.6, cs * 0.2, cs * 0.13, 0, 0, 7); ctx.fill();
    ctx.fillStyle = "#c79a64"; ctx.beginPath(); ctx.ellipse(px + cs / 2, py + cs * 0.56, cs * 0.14, cs * 0.08, 0, 0, 7); ctx.fill();
  }
}

function pedrinhas(ctx, cor, px, py, cs, x, y, qtd) {
  const k = Math.min(6, 2 + Math.floor(qtd / 25));
  for (let j = 0; j < k; j++) {
    const a = ruido(x * 7 + j, y * 3 + j), b = ruido(y * 5 + j, x * 11 + j);
    const ex = px + 3 + a * (cs - 9), ey = py + 3 + b * (cs - 9);
    ctx.fillStyle = "rgba(0,0,0,.35)"; ctx.fillRect(ex + 1, ey + 1, cs * 0.24, cs * 0.18);
    ctx.fillStyle = cor; ctx.fillRect(ex, ey, cs * 0.24, cs * 0.18);
  }
}

function elemento(ctx, C, e, px, py, cs, t) {
  const pr = C.elementos[e.p], ap = pr.aparencia;
  const ativo = !e.parado && (e.rec || (pr.extrai && e.queima > 0));
  ctx.fillStyle = tom(ap.cor, -0.45); ctx.fillRect(px + 1, py + 1, cs - 2, cs - 2);
  ctx.fillStyle = ap.cor; ctx.fillRect(px + 2, py + 2, cs - 4, cs - 5);
  ctx.fillStyle = "rgba(255,255,255,.15)"; ctx.fillRect(px + 2, py + 2, cs - 4, 2);
  if (ativo && pr.emite) {
    const f = (t / 1200) % 1;
    ctx.fillStyle = `rgba(90,90,90,${0.5 * (1 - f)})`;
    ctx.beginPath(); ctx.arc(px + cs * 0.7, py + cs * 0.1 - f * cs * 0.5, cs * (0.1 + 0.12 * f), 0, 7); ctx.fill();
  }
  if (ativo && pr.processo && pr.energia) { ctx.fillStyle = "rgba(255,150,40,.35)"; ctx.fillRect(px + 3, py + cs * 0.55, cs - 6, cs * 0.3); }
  ctx.font = `${Math.floor(cs * 0.62)}px system-ui, sans-serif`;
  ctx.textAlign = "center"; ctx.textBaseline = "middle"; ctx.fillStyle = "#f4efe3";
  ctx.fillText(ap.simbolo, px + cs / 2, py + cs / 2 + 1);
  ctx.textAlign = "start"; ctx.textBaseline = "alphabetic";
  if (e.parado && e.parado !== "Sem entrada" && !(pr.inventario?.guarda)) {
    ctx.fillStyle = "#e5533d"; ctx.beginPath(); ctx.arc(px + cs - 4, py + 4, Math.max(3, cs * 0.14), 0, 7); ctx.fill();
  }
  const s = total(e.sai);
  if (s) { ctx.fillStyle = "#f2c14e"; ctx.fillRect(px + 2, py + cs - 4, Math.min(cs - 4, (cs - 4) * s / 20), 2); }
}
