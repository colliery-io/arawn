// Minimal hypermedia client (ARAWN-T-0493 / GUI-F2).
//
// Connects to the server's SSE push bridge and applies each HTML fragment in
// place — no polling, no full reload. The server sends `fragment` events whose
// data is JSON {target, mode, html}; we swap by selector. This is a stand-in
// for datastar (which can't be vendored offline here); the server contract is
// identical, so datastar drops in later without server changes.
(function () {
  "use strict";
  if (!("EventSource" in window)) return;

  const source = new EventSource("/events");

  source.addEventListener("fragment", function (ev) {
    let frag;
    try {
      frag = JSON.parse(ev.data);
    } catch (_) {
      return;
    }
    const el = document.querySelector(frag.target);
    if (!el) return;
    if (frag.mode === "append") {
      const li = document.createElement("li");
      li.innerHTML = frag.html;
      el.appendChild(li);
    } else {
      el.innerHTML = frag.html;
    }
  });

  // Surface connection state on the brief-status card so a dropped server is
  // visible rather than silently stale.
  source.addEventListener("error", function () {
    const el = document.getElementById("brief-status");
    if (el && source.readyState === EventSource.CLOSED) {
      el.dataset.connection = "closed";
    }
  });
})();
