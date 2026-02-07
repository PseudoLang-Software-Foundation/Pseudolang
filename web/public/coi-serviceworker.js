/*! coi-serviceworker - Enables SharedArrayBuffer via COOP/COEP headers */
/*! Based on https://github.com/nickvdh/coi-serviceworker - MIT License */

if (typeof window === "undefined") {
  self.addEventListener("install", () => self.skipWaiting());
  self.addEventListener("activate", (e) => e.waitUntil(self.clients.claim()));
  self.addEventListener("fetch", (e) => {
    if (
      e.request.cache === "only-if-cached" &&
      e.request.mode !== "same-origin"
    )
      return;
    e.respondWith(
      fetch(e.request)
        .then((r) => {
          if (r.status === 0) return r;
          const h = new Headers(r.headers);
          h.set("Cross-Origin-Embedder-Policy", "credentialless");
          h.set("Cross-Origin-Opener-Policy", "same-origin");
          return new Response(r.body, {
            status: r.status,
            statusText: r.statusText,
            headers: h,
          });
        })
        .catch((err) => console.error("coi-sw fetch error:", err)),
    );
  });
} else {
  (async () => {
    if (window.crossOriginIsolated) return;
    if (!window.navigator.serviceWorker) return;
    const reg = await navigator.serviceWorker.register(
      window.document.currentScript.src,
    );
    if (reg.active && !navigator.serviceWorker.controller) {
      window.location.reload();
    }
  })();
}
