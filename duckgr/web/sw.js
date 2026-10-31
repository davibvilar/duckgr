// Service worker: keeps a copy of the whole game (pages, scripts, the wasm
// core, fonts, icons) so it opens and plays without a network after the
// first visit, in the browser or installed as an app.
//
// tools/build-site.mjs fills PRECACHE with the site's files and a version
// made from their contents. Without it (npm run serve) nothing is cached and
// every request goes to the network as usual.
const PRECACHE = null;

const CACHE = PRECACHE && `duckgr-${PRECACHE.version}`;

self.addEventListener("install", e => {
  if (!PRECACHE) return self.skipWaiting();
  // "reload" skips the browser's HTTP cache, which could hand over files of the previous version
  const requests = PRECACHE.files.map(f => new Request(f, { cache: "reload" }));
  e.waitUntil(caches.open(CACHE).then(c => c.addAll(requests)).then(() => self.skipWaiting()));
});

// A new version replaces the old copy whole, so files of two versions never mix.
self.addEventListener("activate", e => {
  e.waitUntil(caches.keys()
    .then(keys => Promise.all(keys.filter(k => k.startsWith("duckgr-") && k !== CACHE).map(k => caches.delete(k))))
    .then(() => self.clients.claim()));
});

self.addEventListener("fetch", e => {
  if (!PRECACHE || e.request.method !== "GET" || new URL(e.request.url).origin !== location.origin) return;
  e.respondWith(caches.open(CACHE)
    .then(c => c.match(e.request, { ignoreSearch: true }))
    .then(hit => hit || fetch(e.request)));
});
