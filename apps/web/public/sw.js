// Minimal "app shell" service worker for Mneme's web mode.
//
// Scope, deliberately narrow:
// - NEVER intercepts `/api/*` — those must always hit the companion server
//   fresh (this app's data is a live local Well, not something to serve stale
//   from a cache).
// - For everything else (the built JS/CSS bundle, icons, index.html): try the
//   network first, fall back to a cached copy if the device is offline. This
//   just lets the app shell still open when you're briefly offline — it does
//   NOT make your notes available offline (they live on the desktop machine).
const CACHE_NAME = 'mneme-shell-v1';

self.addEventListener('install', (event) => {
  self.skipWaiting();
  event.waitUntil(
    caches
      .open(CACHE_NAME)
      .then((cache) => cache.addAll(['/', '/manifest.webmanifest']))
      .catch(() => {
        /* best-effort precache; a cold cache still works via the fetch handler below */
      }),
  );
});

self.addEventListener('activate', (event) => {
  event.waitUntil(
    caches
      .keys()
      .then((keys) => Promise.all(keys.filter((k) => k !== CACHE_NAME).map((k) => caches.delete(k))))
      .then(() => self.clients.claim()),
  );
});

self.addEventListener('fetch', (event) => {
  const { request } = event;
  const url = new URL(request.url);

  // Only handle our own origin, GET requests, and never /api/*.
  if (url.origin !== self.location.origin || request.method !== 'GET' || url.pathname.startsWith('/api/')) {
    return;
  }

  event.respondWith(
    fetch(request)
      .then((response) => {
        const copy = response.clone();
        caches.open(CACHE_NAME).then((cache) => cache.put(request, copy));
        return response;
      })
      .catch(() => caches.match(request).then((cached) => cached || caches.match('/'))),
  );
});
