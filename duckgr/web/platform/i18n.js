// Interface languages. No interface text lives in the code: every text is a
// key looked up in the language files (web/platform/locales/), and the
// core's messages ({ key, args }) are rendered here too. World content
// (item names, mission texts...) comes from the package, in its base
// language, with per-language overlays from the game's own locales folder.
//
// Templates: "{name}" inserts an argument; "{name|lower}" lowers its first
// letter. A key with plural forms is an object { one, other } chosen by the
// `count` argument.

import ptBR from "./locales/pt-BR.js";
import en from "./locales/en.js";

export const LANGUAGES = [
  { id: "pt-BR", name: "Português (Brasil)", strings: ptBR },
  { id: "en", name: "English", strings: en },
];
export const DEFAULT_LANGUAGE = "en";

let current = LANGUAGES.find(l => l.id === DEFAULT_LANGUAGE);

// The best language for a list of browser preferences (navigator.languages).
export function detectLanguage(preferred = []) {
  for (const tag of preferred) {
    const lower = String(tag).toLowerCase();
    const exact = LANGUAGES.find(l => l.id.toLowerCase() === lower);
    if (exact) return exact.id;
    const base = LANGUAGES.find(l => l.id.toLowerCase().split("-")[0] === lower.split("-")[0]);
    if (base) return base.id;
  }
  return DEFAULT_LANGUAGE;
}

export function setLanguage(id) {
  current = LANGUAGES.find(l => l.id === id) || LANGUAGES.find(l => l.id === DEFAULT_LANGUAGE);
  return current.id;
}
export const language = () => current.id;

const lowerFirst = s => (s ? s[0].toLocaleLowerCase(current.id) + s.slice(1) : s);
const escapeHtml = s => String(s).replace(/[&<>"]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);

function lookup(key) {
  let v = current.strings[key];
  if (v === undefined) v = LANGUAGES.find(l => l.id === DEFAULT_LANGUAGE).strings[key];
  return v;
}

function fill(template, args, value) {
  return template.replace(/\{(\w+)(?:\|(\w+))?\}/g, (whole, name, filter) => {
    if (!(name in args)) return whole;
    const v = value(args[name]);
    return filter === "lower" ? lowerFirst(v) : v;
  });
}

function pick(entry, args) {
  if (typeof entry !== "object" || entry === null) return entry;
  const form = typeof args.count === "number" ? new Intl.PluralRules(current.id).select(args.count) : "other";
  return entry[form] ?? entry.other;
}

// The interface text for a key. `C` is the compiled (and localized) package,
// needed when the arguments point to package texts.
export function t(key, args = {}, C = null) {
  const entry = pick(lookup(key), args);
  if (entry === undefined) return key;
  return fill(entry, args, v => argText(v, C));
}

// Like t(), for templates that carry HTML: the arguments are escaped.
export function tHtml(key, args = {}, C = null) {
  const entry = pick(lookup(key), args);
  if (entry === undefined) return escapeHtml(key);
  return fill(entry, args, v => escapeHtml(argText(v, C)));
}

// A text of the package by its path (["itens", "madeira", "nome"]).
export function packageText(C, path) {
  let v = C;
  for (const k of path) v = v?.[k];
  return typeof v === "string" ? v : undefined;
}

// A message from the core: { key, args }, a package text { ref }, or plain text.
export function msgText(m, C) {
  if (m === null || m === undefined) return "";
  if (typeof m === "string") return m;
  if (m.key) return t(m.key, m.args || {}, C);
  return argText(m, C);
}

function argText(v, C) {
  if (v === null || v === undefined) return "";
  if (typeof v === "number") return String(v);
  if (typeof v === "string") return v;
  if (v.key) return msgText(v, C);
  if (v.ref) return packageText(C, v.ref) ?? v.text ?? String(v.ref[v.ref.length - 2] ?? "");
  if (v.items) return itemList(v.items, C);
  if (v.list) return v.list.map(x => argText(x, C)).join(", ");
  return String(v);
}

// "4 madeira, 1 muda" in the current language.
export function itemList(items, C, separator = ", ") {
  const parts = Object.entries(items).map(([id, n]) => t("items.amount", { n, item: C?.itens?.[id]?.nome ?? id }));
  return parts.length ? parts.join(separator) : t("items.none");
}

export function formatNumber(n, options) {
  return new Intl.NumberFormat(current.id, options).format(n);
}

// Fills the static page: data-i18n (text), data-i18n-html, data-i18n-title,
// data-i18n-aria-label and data-i18n-placeholder hold keys.
export function translatePage(root = document) {
  document.documentElement.lang = current.id;
  for (const el of root.querySelectorAll("[data-i18n]")) el.textContent = t(el.dataset.i18n);
  for (const el of root.querySelectorAll("[data-i18n-html]")) el.innerHTML = t(el.dataset.i18nHtml);
  for (const attr of ["title", "aria-label", "placeholder"]) {
    for (const el of root.querySelectorAll(`[data-i18n-${attr}]`)) el.setAttribute(attr, t(el.getAttribute(`data-i18n-${attr}`)));
  }
}

// Puts the package's texts in the current language, in place (the compiled
// package keeps its identity, which the core bridge depends on). `overlays`
// maps language → partial package with the translated texts.
const originals = new WeakMap();
export function localizePackage(C, overlays = {}) {
  if (!originals.has(C)) originals.set(C, structuredClone(C));
  assign(C, originals.get(C));
  const overlay = overlays[current.id];
  if (overlay) assign(C, overlay);
  return C;
}
// The package as compiled, before any translation. Anything that goes into
// the world (like the mission goals of a new game) must come from here, so
// the world doesn't depend on the player's language.
export const untranslated = C => originals.get(C) || C;

function assign(target, source) {
  for (const [k, v] of Object.entries(source)) {
    if (v && typeof v === "object" && target[k] && typeof target[k] === "object") assign(target[k], v);
    else target[k] = structuredClone(v);
  }
}
