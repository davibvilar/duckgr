// Platform interface: screens (game, menu, settings, world setup), tick
// loop, player input and saving. Generic: everything specific to the world
// comes from the compiled package (C). No interface text lives here: texts
// come from the language files through t() (see i18n.js).

import * as N from "./core.js";
import { drawWorld, playerPosition } from "./render.js";
import * as Save from "./persistence.js";
import { t, tHtml, msgText, itemList, formatNumber, translatePage, localizePackage, untranslated, setLanguage, detectLanguage, language, LANGUAGES } from "./i18n.js";
import { PACKAGES, DEFAULT_PACKAGE, PACKAGE_TRANSLATIONS, COMING_SOON } from "../games/catalog.js";

const $ = s => document.querySelector(s);
const esc = s => String(s).replace(/[&<>"]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);
const read = (k, d) => { try { const v = localStorage.getItem(k); return v ? JSON.parse(v) : d; } catch { return d; } };
const write = (k, v) => { try { localStorage.setItem(k, JSON.stringify(v)); } catch { /* no storage: carry on without saving */ } };
const remove = k => { try { localStorage.removeItem(k); } catch { /* same */ } };
// localStorage keys: they hold games already saved, so they keep their names
const KEY_OLD_GAME = "duckgr:partida", KEY_SAVE = "duckgr:save", KEY_PREFS = "duckgr:prefs", KEY_SAW_HOW_TO = "duckgr:viuComoJogar2";

// Preferences of this device. Older versions stored { velocidade, dicas }.
const DEFAULT_PREFS = { speed: 1, hints: true, language: "auto" };
const storedPrefs = read(KEY_PREFS, {});
let prefs = { ...DEFAULT_PREFS, ...storedPrefs };
if ("velocidade" in storedPrefs && !("speed" in storedPrefs)) prefs.speed = storedPrefs.velocidade;
if ("dicas" in storedPrefs && !("hints" in storedPrefs)) prefs.hints = storedPrefs.dicas;
delete prefs.velocidade; delete prefs.dicas;

function applyLanguage() {
  setLanguage(prefs.language === "auto" ? detectLanguage(navigator.languages || [navigator.language]) : prefs.language);
  for (const C of Object.values(compiled)) localizePackage(C, PACKAGE_TRANSLATIONS[C.id]);
  translatePage();
}

const compiled = {};
function pkg(id) {
  if (!compiled[id]) {
    const C = N.compile(PACKAGES.find(p => p.id === id) || PACKAGES[0]);
    compiled[id] = localizePackage(C, PACKAGE_TRANSLATIONS[C.id]);
  }
  return compiled[id];
}

// game and interface state
let G = null, C = null, S = null;
let building = null, showField = false, invOpen = false, pinnedMenu = null;
const floating = []; // "+1 wood" texts rising above the duck
let hover = null, selCell = null, paused = false, screen = "game", seenHash = "", hashSecond = -1;

const itemName = id => C.itens[id]?.nome ?? id;
const items = o => esc(itemList(o, C));
const msg = m => msgText(m, C);

// ---------- game
// game = { pkg, cfg, cmds, checkpoint? }; a ready state (from a save) is optional
function start(game, untilTick = 0, state = null) {
  G = { pkg: game.pkg, cfg: game.cfg || {}, cmds: game.cmds || [], checkpoint: game.checkpoint || null };
  C = pkg(G.pkg);
  S = state || (untilTick ? N.replay(C, G.cfg, G.cmds, untilTick) : N.newState(C, G.cfg));
  building = null; selCell = null; pinnedMenu = null; paused = false; cam.ready = false;
  renderWorldName();
  resize();
  renderAll();
  save();
}
function renderWorldName() {
  $("#worldName").textContent = C.nome;
  document.title = t("page.title", { world: C.nome });
}
function newDefault() { start({ pkg: DEFAULT_PACKAGE, cfg: {}, cmds: [] }); }
// Saves in the browser: checkpoint + commands, as compressed JSON (base64).
let saving = false, lastSize = null;
async function save() {
  if (!S || saving) return;
  saving = true;
  try {
    const json = JSON.stringify(Save.buildSave(C, G, S));
    const gz = await Save.compress(json);
    lastSize = { json: json.length, gz: gz.length, tick: S.tick };
    write(KEY_SAVE, Save.toBase64(gz));
    remove(KEY_OLD_GAME); // old format (only the command list)
  } catch (err) { console.error("Could not save.", err); }
  finally { saving = false; }
}

function command(cmd, quiet = false) {
  cmd.t = S.tick; cmd.autor = "jogador";
  const r = N.apply(S, C, cmd);
  if (r.ok) { G.cmds.push(cmd); save(); }
  if (r.msg && (!quiet || !r.ok)) toast(msg(r.msg));
  renderPanels();
  return r;
}

// ---------- loop
let last = performance.now(), acc = 0, lastPanels = 0, lastSave = 0;
function frame(now) {
  const dt = Math.min(250, now - last); last = now;
  if (screen === "game" && S) {
    if (!paused && $("#alert").hidden) {
      acc += dt * prefs.speed;
      const ms = 1000 / N.TPS;
      let n = 0;
      while (acc >= ms && n < 60) { acc -= ms; n++; }
      if (n) { N.step(S, C, n); Save.maybeCheckpoint(G, S); } // checkpoint right after the steps, before new commands
      if (n >= 60) acc = 0;
      if (S.events.length) handleEvents();
    }
    draw(now);
    if (now - lastPanels > 250) { lastPanels = now; renderPanels(); }
    if (now - lastSave > 5000) { lastSave = now; save(); }
  }
  requestAnimationFrame(frame);
}

const goalName = k => C.missao.metas[k]?.nome ?? S.missao.metas[k]?.nome ?? "";

function handleEvents() {
  for (const ev of S.events.splice(0)) {
    if (ev.type === "warning") toast(msg(ev.msg));
    else if (ev.type === "gain") {
      const parts = [...Object.entries(ev.gain).map(([it, n]) => [t("float.gain", { n, item: itemName(it) }), "#f2c14e"]),
        ...Object.entries(ev.loss).map(([it, n]) => [t("float.loss", { n, item: itemName(it) }), "#a9b6c6"])];
      parts.forEach(([txt, color], k) => floating.push({ x: ev.x, y: ev.y, txt, color, t0: performance.now() + k * 180 }));
    } else if (ev.type === "era") {
      const era = C.eras[ev.era - 1];
      alertBox(t("era.title", { n: ev.era, era: era.nome }), tHtml("era.body", { n: ev.era, era: era.nome, summary: era.resumo }), t("era.button"), "gold");
    } else if (ev.type === "mission" && ev.status === "cumprida") {
      alertBox(t("mission.doneTitle"), tHtml("mission.doneBody", { mission: C.missao.nome }), t("mission.doneButton"), "gold");
    } else if (ev.type === "mission" && ev.status === "falhou") {
      alertBox(t("mission.failedTitle"), tHtml("mission.failedBody", { goal: goalName(ev.goal), pct: ev.pct }), t("common.gotIt"), "bad");
    }
  }
}

// ---------- drawing and camera
// The canvas has the board's size on screen; the camera says which piece
// of the world shows in it (offset in pixels and cell size = zoom).
const cv = $("#cv"), ctx = cv.getContext("2d");
let cs = 24, width = 600, height = 400;
const cam = { x: 0, y: 0, follow: true, ready: false };
const ZOOM_MAX = 72;
const zoomMin = () => Math.max(6, Math.min(width / S.L, height / S.A));
function resize() {
  if (!S) return;
  const b = $("#board");
  width = b.clientWidth || 600; height = b.clientHeight || 400;
  const dpr = window.devicePixelRatio || 1;
  cv.width = Math.round(width * dpr); cv.height = Math.round(height * dpr);
  if (!cam.ready) {
    // phone: start close to the duck, with big cells for the finger; computer: the map across the width
    cs = matchMedia("(pointer:coarse)").matches ? 34 : Math.max(zoomMin(), Math.min(36, width / S.L));
    cam.ready = true; cam.follow = true;
    center(S.jog.x + 0.5, S.jog.y + 0.5);
  }
  cs = Math.max(zoomMin(), Math.min(ZOOM_MAX, cs));
  clampCam();
}
function clampCam() {
  const W = S.L * cs, H = S.A * cs;
  cam.x = W <= width ? (W - width) / 2 : Math.max(0, Math.min(W - width, cam.x));
  cam.y = H <= height ? (H - height) / 2 : Math.max(0, Math.min(H - height, cam.y));
}
function center(wx, wy) { cam.x = wx * cs - width / 2; cam.y = wy * cs - height / 2; clampCam(); }
// zoom keeping the screen point (sx, sy) fixed
function zoomAt(factor, sx = width / 2, sy = height / 2) {
  const wx = (sx + cam.x) / cs, wy = (sy + cam.y) / cs;
  cs = Math.max(zoomMin(), Math.min(ZOOM_MAX, cs * factor));
  cam.x = wx * cs - sx; cam.y = wy * cs - sy;
  clampCam(); renderCellMenu();
}
function setFollow(v) { cam.follow = v; $("#follow").setAttribute("aria-pressed", v); }
window.addEventListener("pagehide", () => { if (S) save(); });
document.addEventListener("visibilitychange", () => { if (S && document.hidden) save(); });
window.addEventListener("resize", () => { resize(); renderCellMenu(); if (screen === "setup") preview(); });

function draw(time) {
  if (cam.follow) { // the camera follows the duck, smoothly
    const p = playerPosition(S, C), targetX = (p.x + 0.5) * cs - width / 2, targetY = (p.y + 0.5) * cs - height / 2;
    cam.x += (targetX - cam.x) * 0.15; cam.y += (targetY - cam.y) * 0.15; clampCam();
  }
  const dpr = window.devicePixelRatio || 1;
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.fillStyle = "#1d2a3a"; ctx.fillRect(0, 0, width, height);
  ctx.translate(-Math.round(cam.x), -Math.round(cam.y));
  const vis = { x0: Math.floor(cam.x / cs), y0: Math.floor(cam.y / cs), x1: Math.ceil((cam.x + width) / cs), y1: Math.ceil((cam.y + height) / cs) };
  drawWorld(ctx, S, C, {
    cs, t: time, showField, hover, vis,
    ghost: building && hover && !S.occ[N.idx(S, hover.x, hover.y)] ? building : null,
    cursorColor: building ? "#8fd16a" : "#ffffff",
  });
  // floating gain texts
  ctx.font = `800 ${Math.max(12, Math.min(20, cs * 0.55))}px Nunito, system-ui, sans-serif`; ctx.textAlign = "center";
  for (let k = floating.length - 1; k >= 0; k--) {
    const f = floating[k], dt = time - f.t0;
    if (dt > 1400) { floating.splice(k, 1); continue; }
    if (dt < 0) continue;
    const a = 1 - dt / 1400;
    ctx.fillStyle = `rgba(0,0,0,${0.6 * a})`; ctx.fillText(f.txt, (f.x + 0.5) * cs + 1, (f.y - 0.1) * cs - dt / 40 + 1);
    ctx.globalAlpha = a; ctx.fillStyle = f.color; ctx.fillText(f.txt, (f.x + 0.5) * cs, (f.y - 0.1) * cs - dt / 40); ctx.globalAlpha = 1;
  }
  ctx.textAlign = "start";
}

// ---------- board input
// Mouse: hovering shows the cell's actions; clicking does the default
// action; keys do the shortcut's action. Shift (or the "+ queue" button)
// puts it at the end of the queue.
// Touch: tapping opens the cell menu, and tapping an action does it.
// The actions keep the names of the commands ("ir", "colher"...).
const ACTIONS = ["ir", "colher", "minerar", "plantar", "recolher", "abastecer", "fornecer", "desmontar"];
const DEFAULT_ORDER = ["recolher", "abastecer", "fornecer", "colher", "minerar", "ir"];
const keyOf = a => a.acao === "construir" ? String(buildable().indexOf(a.elemento) + 1) : t(`key.${a.acao}`);
const buildable = () => Object.keys(C.elementos).filter(el => S.liberados.includes(el));
function options(c) {
  const r = N.possibleActions(S, C, c.x, c.y);
  r.actions.forEach(a => (a.key = keyOf(a)));
  r.default = DEFAULT_ORDER.map(ac => r.actions.find(a => a.acao === ac)).find(Boolean) || null;
  return r;
}
function order(a, append) {
  if (!a) return;
  const { x, y, acao, item, elemento, vezes } = a;
  command({ tipo: "ordem", x, y, acao, item, elemento, vezes, adicionar: !!append });
}

function cellAt(e) {
  const r = cv.getBoundingClientRect();
  const x = Math.floor((e.clientX - r.left + cam.x) / cs), y = Math.floor((e.clientY - r.top + cam.y) / cs);
  return x < 0 || y < 0 || x >= S.L || y >= S.A ? null : { x, y };
}

// Touch and mouse in the same scheme: dragging moves the camera; two
// fingers (pinch) zoom; a short tap/click, without dragging, is an action
// on the cell.
const fingers = new Map();
let gesture = null; // { x, y, camX, camY, moved, dist, mid }
function tap(e) {
  const c = cellAt(e); if (!c) return;
  selCell = c;
  if (building) { order({ ...c, acao: "construir", elemento: building }, e.shiftKey); return; }
  if (e.pointerType === "mouse") { pinnedMenu = null; order(options(c).default, e.shiftKey); }
  else { hover = null; pinnedMenu = c; renderCellMenu(); renderCell(); }
}
const localPoint = e => { const r = cv.getBoundingClientRect(); return { x: e.clientX - r.left, y: e.clientY - r.top }; };
cv.addEventListener("pointerdown", e => {
  try { cv.setPointerCapture(e.pointerId); } catch { /* the pointer is already gone */ }
  fingers.set(e.pointerId, localPoint(e));
  if (fingers.size === 1) gesture = { ...localPoint(e), camX: cam.x, camY: cam.y, moved: false, button: e.button };
  else if (fingers.size === 2) {
    const [a, b] = [...fingers.values()];
    gesture = { moved: true, dist: Math.hypot(a.x - b.x, a.y - b.y), mid: { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 } };
  }
});
cv.addEventListener("pointermove", e => {
  if (fingers.has(e.pointerId)) {
    fingers.set(e.pointerId, localPoint(e));
    if (fingers.size === 2 && gesture?.dist) {
      const [a, b] = [...fingers.values()];
      const dist = Math.hypot(a.x - b.x, a.y - b.y), mid = { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 };
      setFollow(false);
      cam.x -= mid.x - gesture.mid.x; cam.y -= mid.y - gesture.mid.y;
      zoomAt(dist / gesture.dist, mid.x, mid.y);
      gesture.dist = dist; gesture.mid = mid;
      return;
    }
    if (fingers.size === 1 && gesture) {
      const p = localPoint(e), dx = p.x - gesture.x, dy = p.y - gesture.y;
      if (!gesture.moved && Math.hypot(dx, dy) > 8) { gesture.moved = true; setFollow(false); $("#board").classList.add("dragging"); pinnedMenu = null; }
      if (gesture.moved) { cam.x = gesture.camX - dx; cam.y = gesture.camY - dy; clampCam(); renderCellMenu(); }
      return;
    }
  }
  if (e.pointerType !== "mouse") return;
  const c = cellAt(e);
  if (c && hover && c.x === hover.x && c.y === hover.y) return;
  hover = c; pinnedMenu = null; renderCellMenu(); renderCell();
});
function release(e, cancelled) {
  if (!fingers.has(e.pointerId)) return;
  fingers.delete(e.pointerId);
  $("#board").classList.remove("dragging");
  if (fingers.size === 0) {
    if (!cancelled && gesture && !gesture.moved && !gesture.dist && gesture.button === 0) tap(e);
    gesture = null;
  } else if (fingers.size === 1) { // one finger left the pinch: keep dragging with the other
    const [p] = [...fingers.values()];
    gesture = { ...p, camX: cam.x, camY: cam.y, moved: true };
  }
}
cv.addEventListener("pointerup", e => release(e, false));
cv.addEventListener("pointercancel", e => release(e, true));
cv.addEventListener("pointerleave", e => {
  if (e.pointerType !== "mouse" || fingers.size) return;
  hover = null; renderCellMenu();
});
cv.addEventListener("wheel", e => {
  e.preventDefault();
  const p = localPoint(e);
  setFollow(false);
  zoomAt(Math.exp(-e.deltaY * (e.deltaMode ? 0.05 : 0.0015)), p.x, p.y);
}, { passive: false });
cv.addEventListener("contextmenu", e => e.preventDefault());

$("#zoomIn").onclick = () => zoomAt(1.25);
$("#zoomOut").onclick = () => zoomAt(0.8);
$("#follow").onclick = () => setFollow(!cam.follow);
$("#pauseCam").onclick = () => { paused = !paused; renderTop(); };
$("#fullscreen").onclick = () => toggleFullscreen();

// Fullscreen: uses the browser API when there is one; on the iPhone, which
// doesn't allow it, the immersive mode (the board filling the window) does.
function toggleFullscreen(on = !document.body.classList.contains("immersive")) {
  document.body.classList.toggle("immersive", on);
  $("#fullscreen").textContent = on ? "✕" : "⛶";
  $("#fullscreen").setAttribute("aria-label", t(on ? "board.exitFullscreen" : "board.fullscreen"));
  try {
    if (on && !document.fullscreenElement) document.documentElement.requestFullscreen?.()?.catch(() => {});
    if (!on && document.fullscreenElement) document.exitFullscreen?.()?.catch(() => {});
  } catch { /* no real fullscreen: the immersive mode stays */ }
  requestAnimationFrame(() => { resize(); renderCellMenu(); });
}
document.addEventListener("fullscreenchange", () => {
  if (!document.fullscreenElement && document.body.classList.contains("immersive")) toggleFullscreen(false);
});

// Shortcut keys come from the language file, so each language can pick its
// own letters. Digits, arrows, Escape and Space are the same everywhere.
document.addEventListener("keydown", e => {
  if (screen !== "game" || e.target.closest("input,select,textarea") || e.ctrlKey || e.metaKey || e.altKey) return;
  // digits by the key's code, so Shift+2 stays "2" (and not "@")
  const k = /^Digit\d$/.test(e.code) ? e.code.slice(5) : e.key.length === 1 ? e.key.toUpperCase() : e.key;
  if (!$("#alert").hidden) { if (k === "Escape") $("#alert").hidden = true; return; }
  if (k === t("key.inventory")) { e.preventDefault(); return toggleInv(); }
  if (k === "Escape") {
    if (invOpen) return toggleInv(false);
    if (building) { building = null; return renderBuildBar(); }
    if (pinnedMenu) { pinnedMenu = null; return renderCellMenu(); }
    return command({ tipo: "cancelar" });
  }
  if (invOpen) return;
  if (k === " ") { e.preventDefault(); paused = !paused; return renderTop(); }
  if (k === t("key.showField")) return $("#showField").click();
  if (k === "+" || k === "=") return zoomAt(1.25);
  if (k === "-" || k === "_") return zoomAt(0.8);
  if (k === t("key.follow")) return setFollow(true);
  if (k === t("key.fullscreen")) return toggleFullscreen();
  const arrows = { ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, -1], ArrowDown: [0, 1] }[k];
  if (arrows) { e.preventDefault(); setFollow(false); cam.x += arrows[0] * cs * 3; cam.y += arrows[1] * cs * 3; clampCam(); return renderCellMenu(); }
  const c = hover || pinnedMenu;
  if (!c) return;
  const a = options(c).actions.find(a => a.key === k);
  if (a) { e.preventDefault(); order(a, e.shiftKey); }
});

function renderCellMenu() {
  const el = $("#cellMenu"), c = hover || pinnedMenu;
  if (!c || !S || invOpen) { el.hidden = true; return; }
  const o = options(c);
  const label = a => `${esc(msg(a.label))}${a.detail ? `<small>${esc(msg(a.detail))}</small>` : ""}`;
  const rows = o.actions.map((a, k) => pinnedMenu
    ? `<div class="action-row"><button class="action" data-action="${k}"><span>${label(a)}</span></button><button class="more" data-action="${k}" data-queue="1" aria-label="${esc(t("menu.queueLabel"))}">${esc(t("menu.queue"))}</button></div>`
    : `<button class="action ${a === o.default ? "default" : ""}" data-action="${k}"><span class="key">${esc(a.key)}</span><span>${label(a)}</span></button>`).join("");
  const notes = o.notes.map(n => `<div class="note ${n.kind}">${esc(msg(n.msg))}</div>`).join("");
  const hint = t(pinnedMenu ? "menu.hintTouch" : "menu.hintMouse");
  html(el, `<div class="title"><span>${esc(o.title ? msg(o.title) : "")} <span class="small muted">(${c.x}, ${c.y})</span></span>${pinnedMenu ? `<button data-closemenu="1" aria-label="${esc(t("common.close"))}">×</button>` : ""}</div>${notes}${rows || `<div class="note">${esc(t("menu.nothing"))}</div>`}<div class="footer">${esc(hint)}</div>`);
  el.__actions = o.actions;
  el.hidden = false;
  // with a mouse the menu only informs (it can't cover the neighboring cell); on touch it is clickable
  el.classList.toggle("pinned", !!pinnedMenu);
  if (pinnedMenu) return; // on touch the menu is a sheet fixed under the board
  // places it next to the cell (in screen coordinates), without leaving the board
  const w = el.offsetWidth, h = el.offsetHeight;
  let x = (c.x + 1) * cs - cam.x + 12, y = (c.y + 1) * cs - cam.y;
  if (x + w > width - 56) x = c.x * cs - cam.x - w - 8;
  x = Math.max(4, x);
  y = Math.max(4, Math.min(y, height - h - 4));
  el.style.left = x + "px"; el.style.top = y + "px";
}

// ---------- panels
function html(el, h) { if (el.__h !== h) { el.__h = h; el.innerHTML = h; } }
const clock = tick => { const s = Math.floor(tick / N.TPS); return `${String(Math.floor(s / 60)).padStart(2, "0")}:${String(s % 60).padStart(2, "0")}`; };

function renderAll() { renderTop(); renderBuildBar(); renderShortcuts(); renderPanels(); }
function renderPanels() {
  if (!S) return;
  renderMission(); renderInventory(); renderResearch(); renderCell();
  renderEras(); renderLog(); renderFooter(); renderBuildBar(); renderTop(); renderCellMenu();
  if (invOpen) { renderSlots(); renderCraft(); }
}

function renderTop() {
  $("#pauseCam").textContent = paused ? "▶" : "⏸";
  html($("#topBar"), screen === "game"
    ? `<span class="era-badge">${esc(t("top.era", { n: S.era, era: C.eras[S.era - 1].nome }))}</span><button class="btn" id="pause">${esc(t(paused ? "top.resume" : "top.pause"))}</button><button class="btn" data-go="menu">${esc(t("top.menu"))}</button>`
    : `<button class="btn primary" data-go="game">${esc(t("common.backToGame"))}</button>`);
}

function renderMission() {
  const goals = N.missionProgress(S, C);
  let h = `<div class="name">${esc(C.missao.nome)}<span>${esc(t(`mission.status.${S.missao.estado}`))}</span></div>`;
  goals.forEach((g, k) => {
    const name = esc(goalName(k));
    if (g.tipo === "producao") {
      const pct = Math.min(100, (g.feito * 100) / g.quantidade);
      h += `<div class="goal"><div class="label"><span>${name}</span><span>${Math.min(g.feito, g.quantidade)} / ${g.quantidade}</span></div><div class="bar"><i style="width:${pct}%"></i></div></div>`;
    } else {
      const scale = 150, cls = g.pct < g.minimoPct ? "bad" : g.pct < g.minimoPct + 15 ? "warn" : "";
      h += `<div class="goal"><div class="label"><span>${esc(t("mission.coverGoal", { goal: goalName(k), pct: g.minimoPct }))}</span><span>${g.pct}%</span></div><div class="bar ${cls}"><i style="width:${Math.min(100, (g.pct * 100) / scale)}%"></i><b style="left:${(g.minimoPct * 100) / scale}%"></b></div></div>`;
    }
  });
  html($("#mission"), h);
}

function renderBuildBar() {
  const unlocked = buildable();
  html($("#buildBar"), Object.entries(C.elementos).map(([id, e]) => {
    const k = unlocked.indexOf(id);
    const title = t("build.tooltip", { tip: e.dica || "", cost: itemList(e.custo, C, " + ") });
    return `<button data-build="${id}" class="${building === id ? "on" : ""}" ${k >= 0 ? "" : "disabled"} title="${esc(title)}">${k >= 0 ? `<span class="key">${k + 1}</span>` : ""}${esc(e.aparencia.simbolo)} ${esc(e.nome)}${k >= 0 ? "" : " 🔒"}</button>`;
  }).join(""));
  const n = S.jog.fila.length;
  html($("#queue"), n ? `<span>${esc(t("queue.tasks", { count: n }))}</span><button class="btn" id="cancelQueue">${esc(t("queue.stop"))} <span class="key">Esc</span></button>` : "");
  let hint = t("hint.mouse");
  if (building) { const e = C.elementos[building]; hint = t("hint.building", { element: e.nome, tip: e.dica || "" }); }
  else if (!matchMedia("(hover:hover)").matches) hint = t("hint.touch");
  html($("#hint"), prefs.hints ? esc(hint) : "");
}

function renderInventory() {
  const ids = Object.keys(C.itens).filter(it => S.inv[it]);
  html($("#inventory"), ids.map(it => `<span class="chip"><span class="d" style="background:${C.itens[it].cor}"></span>${esc(C.itens[it].nome)} <span class="n">${S.inv[it]}</span></span>`).join("") || `<span class="muted small">${esc(t("inventory.empty"))}</span>`);
  html($("#playerInfo"), esc(t("inventory.player", { player: C.jogador.nome, speed: formatNumber(N.cellsPerSecond(C)), count: C.jogador.alcance })));
}

function renderSlots() {
  const ids = Object.keys(C.itens).filter(it => S.inv[it]);
  const empty = Math.max(0, 20 - ids.length);
  html($("#slots"), ids.map(it => `<div class="slot" title="${esc(C.itens[it].nome)}"><span class="n">${S.inv[it]}</span><span class="d" style="background:${C.itens[it].cor}"></span><span class="name">${esc(C.itens[it].nome)}</span></div>`).join("")
    + `<div class="slot empty"></div>`.repeat(empty));
  html($("#invPlayer"), $("#playerInfo").innerHTML);
}

function toggleInv(open = !invOpen) {
  invOpen = open;
  $("#invScreen").hidden = !open;
  if (open) { renderSlots(); renderCraft(); $("#closeInv").focus(); }
  renderCellMenu();
}

function renderShortcuts() {
  const actions = ACTIONS.map(a => `<span class="key">${esc(t(`key.${a}`))}</span><span>${esc(t(`shortcut.${a}`))}</span>`).join("");
  const rest = [["1–9", "shortcut.build"], ["Shift", "shortcut.shift"], [t("key.inventory"), "shortcut.inventory"], ["Esc", "shortcut.escape"], [t("key.space"), "shortcut.pause"], [t("key.showField"), "shortcut.showField"]];
  html($("#shortcuts"), actions + rest.map(([k, d]) => `<span class="key">${esc(k)}</span><span>${esc(t(d))}</span>`).join(""));
}

function renderCraft() {
  html($("#craft"), Object.entries(C.receitas).filter(([, r]) => r.mao).map(([id, r]) => {
    const can = n => Object.entries(r.entra).every(([it, q]) => (S.inv[it] || 0) >= q * n);
    return `<div class="row"><div><b>${esc(r.nome)}</b><div class="recipe">${items(r.entra)} → ${items(r.sai)}</div></div><div class="actions"><button class="btn" data-craft="${id}" data-n="1" ${can(1) ? "" : "disabled"}>+1</button><button class="btn" data-craft="${id}" data-n="5" ${can(5) ? "" : "disabled"}>+5</button></div></div>`;
  }).join(""));
}

function renderResearch() {
  html($("#research"), Object.entries(C.tecnologias).map(([id, tech]) => {
    const done = S.techs.includes(id);
    const missing = (tech.requer || []).filter(r => !S.techs.includes(r));
    const can = !done && !missing.length && Object.entries(tech.custo).every(([it, n]) => (S.inv[it] || 0) >= n);
    const sub = done ? t("research.done") : missing.length ? t("research.requires", { techs: missing.map(r => C.tecnologias[r].nome).join(", ") }) : t("research.cost", { description: tech.descricao || "", cost: itemList(tech.custo, C) });
    return `<div class="tech ${done ? "done" : ""}"><div><b>${done ? "✓ " : ""}${esc(tech.nome)}</b><span>${esc(sub)}</span></div>${done ? "" : `<button class="btn ${can ? "primary" : ""}" data-research="${id}" ${can ? "" : "disabled"}>${esc(t("research.button"))}</button>`}</div>`;
  }).join(""));
}

function renderCell() {
  const c = hover || selCell;
  if (!c) { html($("#cell"), `<dt>${esc(t("cell.tip"))}</dt><dd>${esc(t("cell.tipText"))}</dd>`); $("#cellTitle").textContent = t("cell.title"); return; }
  $("#cellTitle").textContent = t("cell.titleAt", { x: c.x, y: c.y });
  const i = N.idx(S, c.x, c.y), cov = S.cov[i], dep = S.dep[i], e = N.entityAt(S, c.x, c.y);
  let h = `<dt>${esc(t("cell.substrate"))}</dt><dd>${esc(C.substratos[S.sub[i]].nome)}</dd>`;
  h += `<dt>${esc(t("cell.deposit"))}</dt><dd>${dep ? esc(t("cell.depositLeft", { deposit: C.depositos[dep].nome, n: S.depQtd[i] })) : "—"}</dd>`;
  let coverText = "—";
  if (cov) {
    const d = C.coberturas[cov];
    coverText = esc(d.cresce ? t("cell.growing", { cover: d.nome, n: S.cres[i], total: d.cresce.segundos }) : d.nome);
  }
  h += `<dt>${esc(t("cell.cover"))}</dt><dd>${coverText}</dd>`;
  if (e) {
    const pr = C.elementos[e.p];
    const r = e.rec && C.receitas[e.rec];
    const status = e.parado ? `<span class="stopped">${esc(msg(e.stoppedMsg ?? e.parado))}</span>`
      : esc(r ? t("cell.making", { recipe: r.nome, pct: Math.floor((e.prog * 100) / r.ticks) }) : pr.extrai ? t("cell.extracting", { pct: Math.floor((e.prog * 100) / pr.extrai.ticks) }) : t("cell.ready"));
    h += `<dt>${esc(t("cell.occupant"))}</dt><dd>${esc(pr.nome)} · ${status}</dd>`;
    if (pr.energia) h += `<dt>${esc(t("cell.fuel"))}</dt><dd>${items(e.comb)}</dd>`;
    if (pr.processo) h += `<dt>${esc(t("cell.input"))}</dt><dd>${items(e.ent)}</dd>`;
    h += `<dt>${esc(t(pr.inventario?.guarda ? "cell.stored" : "cell.output"))}</dt><dd>${items(e.sai)}</dd>`;
  } else h += `<dt>${esc(t("cell.occupant"))}</dt><dd>—</dd>`;
  const k = N.chunkOf(S, C, c.x, c.y);
  for (const [fid, F] of Object.entries(C.campos)) {
    const v = S.campos[fid][k];
    const note = F.dano && v > F.dano.limiar ? "cell.fieldDamage" : F.inibeCrescimento && v > F.inibeCrescimento ? "cell.fieldInhibits" : "cell.field";
    h += `<dt>${esc(F.nome)}</dt><dd>${esc(t(note, { n: Math.floor(v / 100) }))}</dd>`;
  }
  html($("#cell"), h);
}

function renderEras() {
  html($("#eras"), C.eras.map((e, k) => `<div class="era ${k + 1 === S.era ? "current" : k + 1 < S.era ? "past" : ""}" title="${esc(e.resumo)}">${k + 1}. ${esc(e.nome)}${k + 1 > S.era ? " 🔒" : ""}</div>`).join(""));
}

// The world log keeps each entry's message; entries from before the
// messages existed (old saves) only have the Portuguese text.
const logClass = { ruim: "bad", bom: "good" };
function renderLog() {
  html($("#log"), S.log.map(l => `<li class="${logClass[l.cls] || ""}"><span class="t">${clock(l.t)}</span>${esc(l.msg ? msg(l.msg) : l.txt)}</li>`).join(""));
}

function renderFooter() {
  const second = Math.floor(S.tick / N.TPS); // the world hash is recomputed once per second of play
  if (second !== hashSecond) { hashSecond = second; seenHash = N.stateHash(S); }
  html($("#footer"), `<span>${esc(t("footer.time", { time: clock(S.tick), tick: S.tick }))}</span><span>${esc(t("footer.commands", { count: G.cmds.length }))}</span><span>${esc(t("footer.hash"))} <code>${seenHash}</code></span><button class="linkbtn" id="checkReplay">${esc(t("footer.checkReplay"))}</button><span>${esc(t("footer.package", { id: C.id, version: C.versao }))}</span>`);
}

// ---------- clicks on the panels (delegation)
document.addEventListener("click", e => {
  const b = e.target.closest("button"); if (!b) return;
  const d = b.dataset;
  if (d.go) return go(d.go);
  if (d.build) { building = building === d.build ? null : d.build; return renderBuildBar(); }
  if (d.action) { const a = $("#cellMenu").__actions?.[Number(d.action)]; pinnedMenu = null; order(a, e.shiftKey || !!d.queue); return renderCellMenu(); }
  if (d.closemenu) { pinnedMenu = null; return renderCellMenu(); }
  if (b.id === "cancelQueue") return command({ tipo: "cancelar" });
  if (b.id === "openInv" || b.id === "openInv2") return toggleInv(true);
  if (b.id === "closeInv") return toggleInv(false);
  if (d.craft) { for (let k = 0; k < Number(d.n); k++) if (!command({ tipo: "fabricar", receita: d.craft }, k > 0).ok) break; return; }
  if (d.research) return command({ tipo: "pesquisar", tecnologia: d.research });
  if (b.id === "pause") { paused = !paused; return renderTop(); }
  if (b.id === "checkReplay") return checkReplay();
});
$("#showField").onclick = () => { showField = !showField; $("#showField").classList.toggle("on", showField); $("#showField").setAttribute("aria-pressed", showField); };
$("#newDefault").onclick = () => { newDefault(); go("game"); toast(t("toast.newGame")); };
$("#howToPlay").onclick = () => { go("game"); howToPlay(); };

function checkReplay() {
  const t0 = performance.now();
  const S2 = N.replay(C, G.cfg, G.cmds, S.tick);
  const a = N.stateHash(S), b = N.stateHash(S2), ms = Math.round(performance.now() - t0);
  const same = a === b;
  alertBox(t(same ? "replay.sameTitle" : "replay.differentTitle"),
    tHtml(same ? "replay.sameBody" : "replay.differentBody", { count: G.cmds.length, tick: S.tick, ms, game: a, replay: b }),
    t("common.ok"), same ? "" : "bad");
}

// ---------- screens
const SCREENS = { game: "#screenGame", menu: "#screenMenu", settings: "#screenSettings", setup: "#screenSetup", save: "#screenSave" };
function go(name) {
  screen = name;
  for (const [n, id] of Object.entries(SCREENS)) $(id).hidden = n !== name;
  if (name === "settings") renderPrefs();
  if (name === "setup") renderSetup();
  if (name === "save") renderSave();
  if (name === "game") { resize(); renderPanels(); }
  renderTop();
  window.scrollTo(0, 0);
}

// Re-renders everything after the language changes.
function refreshLanguage() {
  applyLanguage();
  if (C) { renderWorldName(); renderAll(); }
  $("#fullscreen").setAttribute("aria-label", t(document.body.classList.contains("immersive") ? "board.exitFullscreen" : "board.fullscreen"));
  if (screen === "settings") renderPrefs();
  if (screen === "setup") renderSetup();
  if (screen === "save") renderSave();
}

// Settings (preferences of this device; they don't change the world)
function renderPrefs() {
  const select = (id, label, help, opts) => `<div class="field"><label for="p_${id}">${esc(label)}</label><select id="p_${id}">${opts.map(([v, txt]) => `<option value="${esc(v)}" ${String(prefs[id]) === String(v) ? "selected" : ""}>${esc(txt)}</option>`).join("")}</select><span class="help">${esc(help)}</span></div>`;
  const speed = v => t("settings.speedValue", { name: t(`settings.speed.${v}`), n: formatNumber(v) });
  $("#prefsForm").innerHTML =
    select("language", t("settings.language"), t("settings.languageHelp"), [["auto", t("settings.languageAuto", { language: LANGUAGES.find(l => l.id === detectLanguage(navigator.languages || [navigator.language])).name })], ...LANGUAGES.map(l => [l.id, l.name])]) +
    select("speed", t("settings.speedLabel"), t("settings.speedHelp"), [0.5, 1, 2, 4].map(v => [v, speed(v)])) +
    select("hints", t("settings.hints"), t("settings.hintsHelp"), [[true, t("settings.show")], [false, t("settings.hide")]]);
  for (const id of ["language", "speed", "hints"]) $("#p_" + id).onchange = ev => {
    const v = ev.target.value;
    prefs[id] = id === "speed" ? Number(v) : id === "hints" ? v === "true" : v;
    write(KEY_PREFS, prefs);
    if (id === "language") refreshLanguage();
  };
  renderOffline();
}

// ---------- offline play: the service worker (sw.js) keeps a copy of the game
let installPrompt = null;
const canWorkOffline = "serviceWorker" in navigator && location.protocol !== "file:";
if (canWorkOffline) {
  const hadController = !!navigator.serviceWorker.controller;
  navigator.serviceWorker.register("sw.js").catch(err => console.warn("Offline play is unavailable:", err));
  // a new version took over: the page keeps running the old one until it reloads
  navigator.serviceWorker.addEventListener("controllerchange", () => {
    if (hadController) toast(t("toast.updated"));
    if (screen === "settings") renderOffline();
  });
}
addEventListener("beforeinstallprompt", e => { e.preventDefault(); installPrompt = e; if (screen === "settings") renderOffline(); });
addEventListener("appinstalled", () => { installPrompt = null; if (screen === "settings") renderOffline(); });
async function renderOffline() {
  const ready = canWorkOffline && !!navigator.serviceWorker.controller &&
    (await caches.keys().catch(() => [])).some(k => k.startsWith("duckgr-"));
  $("#offlineStatus").textContent = t(!canWorkOffline ? "settings.offlineUnavailable" : ready ? "settings.offlineReady" : "settings.offlineNotReady");
  $("#installApp").hidden = !installPrompt;
}
$("#installApp").onclick = async () => {
  if (!installPrompt) return;
  installPrompt.prompt();
  await installPrompt.userChoice.catch(() => null);
  installPrompt = null;
  renderOffline();
};
$("#deleteSave").onclick = () => { remove(KEY_OLD_GAME); remove(KEY_SAVE); newDefault(); toast(t("toast.deleted")); };

// World setup: package, map, forest, resources and goals, with a preview
let setup = null;
function renderSetup() {
  const Cs = pkg(setup?.pkg || G.pkg);
  const prod = Cs.missao.metas.find(m => m.tipo === "producao"), cov = Cs.missao.metas.find(m => m.tipo === "cobertura");
  if (!setup) setup = { pkg: Cs.id, seed: Cs.padrao.semente, size: "medio", forest: 1, resources: 1, goal: prod?.quantidade ?? 50, minimum: cov?.minimoPct ?? 50 };
  const m = setup;
  const select = (id, label, help, opts) => `<div class="field"><label for="s_${id}">${esc(label)}</label><select id="s_${id}">${opts.map(([v, txt, off]) => `<option value="${esc(v)}" ${String(m[id]) === String(v) ? "selected" : ""} ${off ? "disabled" : ""}>${esc(txt)}</option>`).join("")}</select><span class="help">${esc(help)}</span></div>`;
  $("#worldForm").innerHTML =
    select("pkg", t("setup.world"), t("setup.worldHelp"), [...PACKAGES.map(p => [p.id, pkg(p.id).nome]), ...COMING_SOON.map(id => ["", t("setup.comingSoon", { world: t(`soon.${id}.name`) }), true])]) +
    `<div class="field"><label for="s_seed">${esc(t("setup.seed"))}</label><div class="inline"><input id="s_seed" type="number" min="1" max="999999" value="${m.seed}"><button class="btn" id="s_draw" title="${esc(t("setup.drawSeed"))}">🎲</button></div><span class="help">${esc(t("setup.seedHelp"))}</span></div>` +
    select("size", t("setup.size"), t("setup.sizeHelp"), Object.entries(Cs.mapa.tamanhos).map(([k, s]) => [k, t("setup.sizeValue", { name: s.nome, w: s.largura, h: s.altura })])) +
    select("forest", t("setup.forest"), t("setup.forestHelp"), [[0.6, t("setup.forest.low")], [1, t("setup.forest.normal")], [1.5, t("setup.forest.high")]]) +
    select("resources", t("setup.resources"), t("setup.resourcesHelp"), [[0.5, t("setup.resources.low")], [1, t("setup.resources.normal")], [2, t("setup.resources.high")]]) +
    (prod ? select("goal", t("setup.goal", { goal: prod.nome }), t("setup.goalHelp"), [20, 50, 100, 200].map(v => [v, String(v)])) : "") +
    (cov ? select("minimum", t("setup.minimum", { goal: cov.nome }), t("setup.minimumHelp"), [30, 50, 70, 90].map(v => [v, `${v}%`])) : "");
  for (const id of ["pkg", "seed", "size", "forest", "resources", "goal", "minimum"]) {
    const el = $("#s_" + id); if (!el) continue;
    el.onchange = ev => { m[id] = id === "pkg" || id === "size" ? ev.target.value : Number(ev.target.value) || 1; if (id === "pkg") renderSetup(); else preview(); };
  }
  $("#s_draw").onclick = () => { m.seed = 1 + Math.floor(Math.random() * 999998); $("#s_seed").value = m.seed; preview(); };
  preview();
}
// The configuration of the new world. The goals come from the untranslated
// package: they go into the world, which must not depend on the language.
function setupCfg() {
  const Cs = untranslated(pkg(setup.pkg)), s = Cs.mapa.tamanhos[setup.size];
  const goals = Cs.missao.metas.map(x => x.tipo === "producao" ? { ...x, quantidade: setup.goal } : x.tipo === "cobertura" ? { ...x, minimoPct: setup.minimum } : x);
  return { semente: setup.seed, largura: s.largura, altura: s.altura, floresta: setup.forest, recursosIniciais: setup.resources, metas: goals };
}
function preview() {
  const Cs = pkg(setup.pkg), cfg = setupCfg(), Sp = N.newState(Cs, cfg);
  const cvp = $("#previewCanvas"), w = $("#preview").clientWidth || 600, c = Math.max(8, Math.floor(w / Sp.L)), dpr = window.devicePixelRatio || 1;
  cvp.width = Sp.L * c * dpr; cvp.height = Sp.A * c * dpr; cvp.style.width = Sp.L * c + "px"; cvp.style.height = Sp.A * c + "px";
  const x = cvp.getContext("2d"); x.setTransform(dpr, 0, 0, dpr, 0, 0);
  drawWorld(x, Sp, Cs, { cs: c, t: 0 });
  $("#previewInfo").textContent = t("setup.previewInfo", { w: Sp.L, h: Sp.A, trees: N.countMark(Sp, Cs, "floresta"), inventory: itemList(Sp.inv, Cs) });
}
$("#startSetup").onclick = () => { start({ pkg: setup.pkg, cfg: setupCfg(), cmds: [] }); go("game"); toast(t("toast.worldReady")); };

// Save and load: a .duckgr file = the save as gzip-compressed JSON
const kb = n => t("save.kb", { n: formatNumber(n / 1024, { minimumFractionDigits: 1, maximumFractionDigits: 1 }) });
async function renderSave() {
  await save();
  const sv = Save.buildSave(C, G, S);
  const parts = [
    [t(sv.checkpoint.estado ? "save.partCheckpoint" : "save.partNoCheckpoint"), sv.checkpoint.estado ? { tick: sv.checkpoint.tick, estado: sv.checkpoint.estado } : null],
    [t("save.partSince"), sv.historico.filter(c => c.t >= sv.checkpoint.tick)],
    [t("save.partHistory"), sv.historico],
  ];
  let rows = `<span>${esc(t("save.part"))}</span><span>JSON</span><span>gzip</span>`;
  for (const [name, data] of parts) {
    const j = data ? JSON.stringify(data) : "";
    const g = j ? (await Save.compress(j)).length : 0;
    rows += `<span>${esc(name)}</span><b>${kb(j.length)}</b><b>${kb(g)}</b>`;
  }
  rows += `<span class="total">${esc(t("save.wholeFile"))}</span><b class="total">${kb(lastSize.json)}</b><b class="total">${kb(lastSize.gz)}</b>`;
  const cpMin = Math.floor((S.tick - sv.checkpoint.tick) / N.TPS / 60);
  const since = sv.historico.filter(c => c.t >= sv.checkpoint.tick).length;
  const summary = t("save.summary", { count: G.cmds.length, time: clock(S.tick) }) + " " +
    (sv.checkpoint.estado ? t("save.withCheckpoint", { min: cpMin, tick: sv.checkpoint.tick, count: since }) : t("save.withoutCheckpoint"));
  $("#saveInfo").innerHTML = `<div class="sizes">${rows}</div><p class="hint" style="margin:8px 0 0">${esc(summary)}</p>`;
  // inside a frame (like in Claude), downloading files is blocked: only the code is left
  const inFrame = window.self !== window.top;
  $("#downloadSave").hidden = inFrame;
  $("#exportInfo").textContent = t(inFrame ? "save.exportInFrame" : "save.exportInfo");
}
async function saveBytes() { return Save.compress(JSON.stringify(Save.buildSave(C, G, S))); }
$("#downloadSave").onclick = async () => {
  const url = URL.createObjectURL(new Blob([await saveBytes()], { type: "application/gzip" }));
  const a = document.createElement("a");
  a.href = url; a.download = `${C.id}-${clock(S.tick).replace(":", "m")}s.duckgr`; a.click();
  setTimeout(() => URL.revokeObjectURL(url), 5000);
};
$("#copySave").onclick = async () => {
  const txt = Save.toBase64(await saveBytes());
  try { await navigator.clipboard.writeText(txt); toast(t("toast.copied", { size: kb(txt.length) })); }
  catch { const el = $("#saveCode"); el.value = txt; el.select(); toast(t("toast.copyFailed")); }
};
async function openSaveBytes(bytes) {
  try {
    const sv = JSON.parse(await Save.decompress(bytes));
    const Cs = pkg(sv.pacote?.id);
    const { S: st, ok } = Save.loadSave(Cs, sv);
    start({ pkg: Cs.id, cfg: sv.cfg, cmds: sv.historico, checkpoint: sv.checkpoint }, 0, st);
    go("game");
    if (ok) toast(t("toast.opened", { time: clock(S.tick) }));
    else alertBox(t("open.mismatchTitle"), tHtml("open.mismatchBody"), t("common.ok"), "bad");
  } catch (err) {
    const reason = err instanceof Save.SaveError ? t(err.key, err.args) : t("open.invalid");
    alertBox(t("open.failedTitle"), `<p>${esc(reason)}</p>`, t("common.ok"), "bad");
  }
}
$("#saveFile").onchange = async e => {
  const f = e.target.files[0]; if (!f) return;
  await openSaveBytes(new Uint8Array(await f.arrayBuffer()));
  e.target.value = "";
};
$("#pasteSave").onclick = () => {
  const txt = $("#saveCode").value.trim();
  if (!txt) return toast(t("toast.pasteFirst"));
  try { openSaveBytes(Save.fromBase64(txt)); } catch { alertBox(t("open.failedTitle"), tHtml("open.badCode"), t("common.ok"), "bad"); }
};

// ---------- notices
let toastTimer;
function toast(m) { const el = $("#toast"); el.textContent = m; el.hidden = false; clearTimeout(toastTimer); toastTimer = setTimeout(() => (el.hidden = true), 2200); }
function alertBox(title, body, button = t("common.gotIt"), kind = "") {
  $("#alertTitle").textContent = title; $("#alertText").innerHTML = body; $("#alertBtn").textContent = button;
  $("#alertBox").className = "box " + kind; $("#alert").hidden = false; $("#alertBtn").focus();
}
$("#alertBtn").onclick = () => { $("#alert").hidden = true; };
function howToPlay() {
  const goal = C.missao.metas.find(m => m.tipo === "producao")?.nome || t("howTo.produce");
  alertBox(t("howTo.title"), tHtml("howTo.body", { goal }), t("howTo.button"));
  write(KEY_SAW_HOW_TO, true);
}

// ---------- game: goes straight into a game, no menu in the way
async function openSaved() {
  try {
    const b64 = read(KEY_SAVE, null);
    if (b64) {
      const sv = JSON.parse(await Save.decompress(Save.fromBase64(b64)));
      const Cs = pkg(sv.pacote.id);
      const { S: st } = Save.loadSave(Cs, sv);
      start({ pkg: sv.pacote.id, cfg: sv.cfg, cmds: sv.historico, checkpoint: sv.checkpoint }, 0, st);
      return;
    }
    // old format (v0.1 to v0.3): only the command list, redone from zero
    const old = read(KEY_OLD_GAME, null);
    if (old && PACKAGES.some(p => p.id === old.pacote) && old.versao === pkg(old.pacote).versao) return start({ pkg: old.pacote, cfg: old.cfg, cmds: old.cmds }, old.tick || 0);
  } catch (err) {
    console.error("The saved game could not be loaded; starting over.", err);
  }
  newDefault();
}
applyLanguage();
await openSaved();
go("game");
if (!read(KEY_SAW_HOW_TO, false)) howToPlay();
requestAnimationFrame(frame);

// for debugging in the browser console: duckgr.state(), duckgr.pkg()
window.duckgr = { state: () => S, pkg: () => C, game: () => G, camera: () => ({ ...cam, cs }) };
