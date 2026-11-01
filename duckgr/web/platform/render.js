// Draws the world on a 2D canvas. Generic: the look of each kind comes
// from the package (color, shape, symbol). The core doesn't know this file exists.

import { idx, total, passageCost } from "./core.js";

export function noise(x, y) {
  const s = Math.sin(x * 12.9898 + y * 78.233) * 43758.5453;
  return s - Math.floor(s);
}
export function shade(hex, k) {
  const n = parseInt(hex.slice(1), 16);
  const f = v => Math.max(0, Math.min(255, Math.round(k < 0 ? v * (1 + k) : v + (255 - v) * k)));
  return `rgb(${f(n >> 16)},${f((n >> 8) & 255)},${f(n & 255)})`;
}

// op: { cs, t (time in ms, only for animation), hover, ghost, showField, vis, cursorColor }
export function drawWorld(ctx, S, C, op) {
  const cs = op.cs;
  // only draws what the camera sees
  const v = op.vis || { x0: 0, y0: 0, x1: S.L, y1: S.A };
  const X0 = Math.max(0, v.x0), Y0 = Math.max(0, v.y0), X1 = Math.min(S.L, v.x1), Y1 = Math.min(S.A, v.y1);
  for (let y = Y0; y < Y1; y++) for (let x = X0; x < X1; x++) {
    const i = idx(S, x, y), px = x * cs, py = y * cs;
    const sb = C.substratos[S.sub[i]].aparencia;
    const n = noise(x, y);
    ctx.fillStyle = shade(sb.cor, (n - 0.5) * 0.12);
    ctx.fillRect(px, py, cs, cs);
    if (sb.forma === "agua") {
      ctx.fillStyle = "rgba(255,255,255,.18)";
      const o = ((op.t / 900 + n * 4) % 1) * cs;
      ctx.fillRect(px + o * 0.6, py + cs * 0.35, cs * 0.3, 1.5);
      ctx.fillRect(px + cs * 0.5 - o * 0.3, py + cs * 0.7, cs * 0.25, 1.5);
    }
    const c = S.cov[i];
    if (c) cover(ctx, C.coberturas[c].aparencia, px, py, cs, n, x, y, S.cres[i], C.coberturas[c]);
    const d = S.dep[i];
    if (d) pebbles(ctx, C.depositos[d].aparencia.cor, px, py, cs, x, y, S.depQtd[i]);
  }
  for (const e of S.ents) element(ctx, C, e, e.x * cs, e.y * cs, cs, op.t);

  // fields: haze per chunk
  const ch = C.mapa.chunk;
  for (const [fid, F] of Object.entries(C.campos)) {
    const v = S.campos[fid];
    for (let k = 0; k < v.length; k++) {
      const cx = (k % S.CL) * ch * cs, cy = Math.floor(k / S.CL) * ch * cs;
      const a = Math.min(op.showField ? 0.6 : 0.4, v[k] / (F.dano?.limiar * 2.5 || 10000)) + (op.showField ? 0.05 : 0);
      if (a > 0.02) { ctx.fillStyle = `rgba(60,48,38,${a})`; ctx.fillRect(cx, cy, ch * cs, ch * cs); }
      if (op.showField) {
        ctx.strokeStyle = "rgba(255,255,255,.25)"; ctx.lineWidth = 1; ctx.setLineDash([3, 3]);
        ctx.strokeRect(cx + 0.5, cy + 0.5, ch * cs - 1, ch * cs - 1); ctx.setLineDash([]);
        ctx.font = `800 ${Math.max(11, cs * 0.6)}px Nunito, system-ui, sans-serif`;
        ctx.fillStyle = v[k] > F.dano.limiar ? "#ff9d8f" : v[k] > F.inibeCrescimento ? "#f2c14e" : "#f4efe3";
        ctx.fillText(String(Math.floor(v[k] / 100)), cx + 4, cy + Math.max(13, cs * 0.75));
      }
    }
  }

  drawPlayer(ctx, S, C, op);

  if (op.hover) {
    const { x, y } = op.hover;
    ctx.strokeStyle = op.cursorColor || "#ffffff"; ctx.lineWidth = 2;
    ctx.strokeRect(x * cs + 1, y * cs + 1, cs - 2, cs - 2);
    if (op.ghost) {
      ctx.globalAlpha = 0.45;
      element(ctx, C, { p: op.ghost, sai: {}, ent: {}, comb: {}, rec: null, parado: "" }, x * cs, y * cs, cs, 0);
      ctx.globalAlpha = 1;
    }
  }
}

function cover(ctx, ap, px, py, cs, n, x, y, growth, def) {
  const green = "#5a9a4a";
  if (ap.forma !== "agua") {
    ctx.fillStyle = n < 0.5 ? green : n < 0.85 ? "#539145" : "#62a352";
    ctx.fillRect(px, py, cs, cs);
  }
  if (ap.forma === "grama") {
    if (n > 0.9) { ctx.fillStyle = "#7dbb5f"; ctx.fillRect(px + cs * 0.3, py + cs * 0.4, cs * 0.1, cs * 0.25); ctx.fillRect(px + cs * 0.55, py + cs * 0.3, cs * 0.1, cs * 0.35); }
  } else if (ap.forma === "broto") {
    const g = def.cresce ? Math.min(1, growth / def.cresce.segundos) : 0;
    ctx.fillStyle = "#6b4a2b"; ctx.fillRect(px + cs * 0.3, py + cs * 0.68, cs * 0.4, cs * 0.12);
    ctx.fillStyle = ap.cor;
    const h = cs * (0.22 + 0.15 * g);
    ctx.fillRect(px + cs * 0.47, py + cs * 0.68 - h, cs * 0.07, h);
    ctx.beginPath(); ctx.ellipse(px + cs * 0.38, py + cs * 0.68 - h, cs * 0.12, cs * 0.07, -0.5, 0, 7); ctx.fill();
    ctx.beginPath(); ctx.ellipse(px + cs * 0.62, py + cs * 0.68 - h * 0.8, cs * 0.12, cs * 0.07, 0.5, 0, 7); ctx.fill();
  } else if (ap.forma === "arvore") {
    const r = cs * ap.tamanho, ox = (noise(y, x) - 0.5) * cs * 0.12;
    ctx.fillStyle = "#5b3d22"; ctx.fillRect(px + cs * 0.46 + ox, py + cs * 0.55, cs * 0.08, cs * 0.35);
    ctx.fillStyle = "rgba(0,0,0,.2)"; ctx.beginPath(); ctx.arc(px + cs / 2 + ox + 1.5, py + cs * 0.48 + 1.5, r, 0, 7); ctx.fill();
    ctx.fillStyle = ap.cor; ctx.beginPath(); ctx.arc(px + cs / 2 + ox, py + cs * 0.48, r, 0, 7); ctx.fill();
    ctx.fillStyle = "rgba(255,255,255,.14)"; ctx.beginPath(); ctx.arc(px + cs / 2 + ox - r * 0.3, py + cs * 0.48 - r * 0.3, r * 0.45, 0, 7); ctx.fill();
  } else if (ap.forma === "toco") {
    ctx.fillStyle = ap.cor; ctx.beginPath(); ctx.ellipse(px + cs / 2, py + cs * 0.6, cs * 0.2, cs * 0.13, 0, 0, 7); ctx.fill();
    ctx.fillStyle = "#c79a64"; ctx.beginPath(); ctx.ellipse(px + cs / 2, py + cs * 0.56, cs * 0.14, cs * 0.08, 0, 0, 7); ctx.fill();
  }
}

function pebbles(ctx, color, px, py, cs, x, y, amount) {
  const k = Math.min(6, 2 + Math.floor(amount / 25));
  for (let j = 0; j < k; j++) {
    const a = noise(x * 7 + j, y * 3 + j), b = noise(y * 5 + j, x * 11 + j);
    const ex = px + 3 + a * (cs - 9), ey = py + 3 + b * (cs - 9);
    ctx.fillStyle = "rgba(0,0,0,.35)"; ctx.fillRect(ex + 1, ey + 1, cs * 0.24, cs * 0.18);
    ctx.fillStyle = color; ctx.fillRect(ex, ey, cs * 0.24, cs * 0.18);
  }
}

function element(ctx, C, e, px, py, cs, t) {
  const pr = C.elementos[e.p], ap = pr.aparencia;
  const working = !e.parado && (e.rec || (pr.extrai && e.queima > 0));
  ctx.fillStyle = shade(ap.cor, -0.45); ctx.fillRect(px + 1, py + 1, cs - 2, cs - 2);
  ctx.fillStyle = ap.cor; ctx.fillRect(px + 2, py + 2, cs - 4, cs - 5);
  ctx.fillStyle = "rgba(255,255,255,.15)"; ctx.fillRect(px + 2, py + 2, cs - 4, 2);
  if (working && pr.emite) {
    const f = (t / 1200) % 1;
    ctx.fillStyle = `rgba(90,90,90,${0.5 * (1 - f)})`;
    ctx.beginPath(); ctx.arc(px + cs * 0.7, py + cs * 0.1 - f * cs * 0.5, cs * (0.1 + 0.12 * f), 0, 7); ctx.fill();
  }
  if (working && pr.processo && pr.energia) { ctx.fillStyle = "rgba(255,150,40,.35)"; ctx.fillRect(px + 3, py + cs * 0.55, cs - 6, cs * 0.3); }
  ctx.font = `${Math.floor(cs * 0.62)}px system-ui, sans-serif`;
  ctx.textAlign = "center"; ctx.textBaseline = "middle"; ctx.fillStyle = "#f4efe3";
  ctx.fillText(ap.simbolo, px + cs / 2, py + cs / 2 + 1);
  ctx.textAlign = "start"; ctx.textBaseline = "alphabetic";
  if (e.parado && e.stoppedMsg?.key !== "stopped.noInput" && !(pr.inventario?.guarda)) {
    ctx.fillStyle = "#e5533d"; ctx.beginPath(); ctx.arc(px + cs - 4, py + 4, Math.max(3, cs * 0.14), 0, 7); ctx.fill();
  }
  const s = total(e.sai);
  if (s) { ctx.fillStyle = "#f2c14e"; ctx.fillRect(px + 2, py + cs - 4, Math.min(cs - 4, (cs - 4) * s / 20), 2); }
}

// A move longer than one cell is a flight (it takes the time of one step).
const isFlight = (x, y, nx, ny) => Math.max(Math.abs(nx - x), Math.abs(ny - y)) > 1;

// The player's position on screen, with the fraction of the step under way
// (for animation). During a flight, `lift` (0 to 1) says how high the duck is.
export function playerPosition(S, C) {
  const P = S.jog, next = P.cam[0];
  if (next === undefined || !P.passo) return { x: P.x, y: P.y, dx: 0, lift: 0 };
  const nx = next % S.L, ny = Math.floor(next / S.L), flying = isFlight(P.x, P.y, nx, ny);
  const cost = flying ? 1 : Math.max(1, passageCost(S, C, nx, ny));
  const f = Math.min(1, P.passo / (C.jogador.ticksPorCelula * cost));
  return { x: P.x + (nx - P.x) * f, y: P.y + (ny - P.y) * f, dx: nx - P.x, lift: flying ? Math.sin(Math.PI * f) : 0 };
}

let facingLeft = false;
const FLIGHT_HEIGHT = 0.7; // how high the duck flies, in cells
function drawPlayer(ctx, S, C, op) {
  const cs = op.cs, P = S.jog, R = C.jogador.alcance;
  // reach
  ctx.strokeStyle = "rgba(242,193,78,.35)"; ctx.lineWidth = 1.5; ctx.setLineDash([4, 4]);
  ctx.strokeRect((P.x - R) * cs + 0.5, (P.y - R) * cs + 0.5, (2 * R + 1) * cs - 1, (2 * R + 1) * cs - 1);
  ctx.setLineDash([]);
  // path
  const p = playerPosition(S, C);
  if (P.cam.length) {
    ctx.strokeStyle = "rgba(255,255,255,.7)"; ctx.lineWidth = 2; ctx.setLineDash([2, 5]);
    ctx.beginPath(); ctx.moveTo((p.x + 0.5) * cs, (p.y + 0.5) * cs);
    // steps are straight lines; flights are arcs, from cell to cell
    let fx = P.x, fy = P.y, first = true;
    for (const i of P.cam) {
      const nx = i % S.L, ny = Math.floor(i / S.L), tx = (nx + 0.5) * cs, ty = (ny + 0.5) * cs;
      if (isFlight(fx, fy, nx, ny)) {
        const ox = first ? p.x : fx, oy = first ? p.y : fy;
        ctx.quadraticCurveTo(((ox + nx) / 2 + 0.5) * cs, ((oy + ny) / 2 + 0.5) * cs - FLIGHT_HEIGHT * cs * 2, tx, ty);
      } else ctx.lineTo(tx, ty);
      fx = nx; fy = ny; first = false;
    }
    ctx.stroke(); ctx.setLineDash([]);
  }
  // queued tasks: a numbered badge on each target
  P.fila.forEach((o, k) => {
    ctx.strokeStyle = k === 0 ? "#f2c14e" : "rgba(242,193,78,.6)"; ctx.lineWidth = 2;
    ctx.strokeRect(o.x * cs + 2, o.y * cs + 2, cs - 4, cs - 4);
    ctx.fillStyle = "#f2c14e"; ctx.beginPath(); ctx.arc(o.x * cs + cs - 3, o.y * cs + 3, Math.max(6, cs * 0.24), 0, 7); ctx.fill();
    ctx.fillStyle = "#13202e"; ctx.font = `800 ${Math.max(9, cs * 0.34)}px Nunito, system-ui, sans-serif`;
    ctx.textAlign = "center"; ctx.textBaseline = "middle";
    ctx.fillText(String(k + 1), o.x * cs + cs - 3, o.y * cs + 3.5);
    ctx.textAlign = "start"; ctx.textBaseline = "alphabetic";
  });
  // progress bar of the action under way
  const o = P.fila[0];
  if (o && P.tarefa) {
    const f = P.tarefa / (C.jogador.duracoes[o.acao] || 1);
    ctx.fillStyle = "rgba(0,0,0,.5)"; ctx.fillRect(o.x * cs + 3, o.y * cs + cs - 6, cs - 6, 4);
    ctx.fillStyle = "#f2c14e"; ctx.fillRect(o.x * cs + 3, o.y * cs + cs - 6, (cs - 6) * f, 4);
  }
  // the duck
  if (p.dx) facingLeft = p.dx < 0;
  const px = (p.x + 0.5) * cs, gy = (p.y + 0.5) * cs, s = cs * 0.42, d = facingLeft ? -1 : 1;
  // in flight, the duck goes up and the shadow stays on the ground, smaller
  const py = gy - p.lift * FLIGHT_HEIGHT * cs;
  const bob = P.cam.length && !p.lift ? Math.sin(op.t / 90) * cs * 0.04 : 0;
  const shadow = 1 - p.lift * 0.45;
  ctx.fillStyle = `rgba(0,0,0,${0.28 * shadow})`; ctx.beginPath(); ctx.ellipse(px, gy + s * 0.85, s * 0.8 * shadow, s * 0.25 * shadow, 0, 0, 7); ctx.fill();
  // in flight, the wings open and flap: one behind the body, one in front
  const flap = Math.sin(op.t / 40);
  const wing = color => {
    ctx.save(); ctx.translate(px - d * s * 0.15, py + s * 0.1); ctx.rotate(-d * (0.9 + flap * 0.6));
    ctx.fillStyle = color; ctx.strokeStyle = "#7a5a10"; ctx.lineWidth = 1.5;
    ctx.beginPath(); ctx.ellipse(-d * s * 0.6, 0, s * 0.7, s * 0.24, 0, 0, 7); ctx.fill(); ctx.stroke();
    ctx.restore();
  };
  if (p.lift) wing("#d9ad22");
  ctx.fillStyle = "#f6d34a"; ctx.strokeStyle = "#7a5a10"; ctx.lineWidth = 1.5;
  ctx.beginPath(); ctx.ellipse(px - d * s * 0.1, py + s * 0.25 + bob, s * 0.75, s * 0.52, 0, 0, 7); ctx.fill(); ctx.stroke();
  ctx.beginPath(); ctx.arc(px + d * s * 0.38, py - s * 0.35 + bob, s * 0.42, 0, 7); ctx.fill(); ctx.stroke();
  ctx.fillStyle = "#f08a24"; ctx.beginPath();
  ctx.moveTo(px + d * s * 0.7, py - s * 0.38 + bob); ctx.lineTo(px + d * s * 1.15, py - s * 0.24 + bob); ctx.lineTo(px + d * s * 0.7, py - s * 0.14 + bob); ctx.fill();
  ctx.fillStyle = "#13202e"; ctx.beginPath(); ctx.arc(px + d * s * 0.48, py - s * 0.45 + bob, Math.max(1.2, s * 0.08), 0, 7); ctx.fill();
  if (p.lift) wing("#f2c94a");
  else { ctx.fillStyle = "#e9bf2f"; ctx.beginPath(); ctx.ellipse(px - d * s * 0.25, py + s * 0.2 + bob, s * 0.35, s * 0.2, d * -0.4, 0, 7); ctx.fill(); }
}
