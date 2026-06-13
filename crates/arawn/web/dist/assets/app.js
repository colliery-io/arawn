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
    if (el) {
      if (frag.mode === "append") {
        const li = document.createElement("li");
        li.innerHTML = frag.html;
        el.appendChild(li);
      } else {
        el.innerHTML = frag.html;
      }
    }
    // Optional "<url> <selector>" hint: if that selector exists on the current
    // page, re-pull it from the server and swap it in place (no full reload).
    if (frag.refresh) {
      const sp = frag.refresh.indexOf(" ");
      if (sp > 0) {
        const url = frag.refresh.slice(0, sp);
        const sel = frag.refresh.slice(sp + 1);
        const target = document.querySelector(sel);
        if (target) {
          fetch(url)
            .then((r) => r.text())
            .then((body) => {
              const doc = new DOMParser().parseFromString(body, "text/html");
              const fresh = doc.querySelector(sel);
              if (fresh) target.innerHTML = fresh.innerHTML;
            })
            .catch(() => {});
        }
      }
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

  // ── Inbox triage (GUI-S2): action buttons + keyboard nav ──────────────
  // Delegated click: any [data-action] button POSTs and swaps/removes its row.
  document.addEventListener("click", function (e) {
    const btn = e.target.closest("[data-action]");
    if (!btn) return;
    e.preventDefault();
    const row = btn.closest(".todo-row");
    fetch(btn.getAttribute("data-action"), { method: "POST" })
      .then(function (r) { return r.text(); })
      .then(function (html) {
        if (!row) return;
        if (btn.dataset.remove) row.remove();
        else row.innerHTML = html;
      })
      .catch(function () {});
  });

  // Keyboard: j/k move focus, x dismiss focused row, e expand rationale.
  document.addEventListener("keydown", function (e) {
    const rows = Array.from(document.querySelectorAll(".todo-row"));
    if (!rows.length) return;
    const cur = document.activeElement && document.activeElement.closest
      ? document.activeElement.closest(".todo-row")
      : null;
    const idx = cur ? rows.indexOf(cur) : -1;
    if (e.key === "j") {
      e.preventDefault();
      rows[idx < 0 ? 0 : Math.min(idx + 1, rows.length - 1)].focus();
    } else if (e.key === "k") {
      e.preventDefault();
      rows[idx <= 0 ? 0 : idx - 1].focus();
    } else if (e.key === "x" && cur) {
      const b = cur.querySelector("[data-remove]");
      if (b) b.click();
    } else if (e.key === "e" && cur) {
      cur.classList.toggle("expanded");
    }
  });
})();
