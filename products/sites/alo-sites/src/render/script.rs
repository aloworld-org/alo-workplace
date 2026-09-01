//! The two scripts of a rendered page: the behavior script (accessible menu disclosure +
//! form submit) and the analytics beacon.
//!
//! Together these are the **entire** JavaScript budget of a published site
//! (`docs/design/sites.md` — "near-zero JS"), pinned by a byte-budget test
//! below. Both are static constants with zero user data interpolated, which
//! is why inlining them is XSS-safe; they are inlined rather than served as
//! further asset paths so the crate's public-path contract stays at three
//! paths and a page needs no extra request. The behavior script is appended
//! only when the page has something for it to do (a nav, or a form with a
//! working submit); the beacon is appended to every published page.
//!
//! Every behavior here is progressive enhancement over a page that already
//! works scriptless:
//!
//! - **Menu toggle** — adds `js` to `<html>` (the stylesheet only collapses
//!   the mobile menu under that class, so no-JS visitors always see the
//!   expanded menu) and flips `aria-expanded` on `.nav-toggle` clicks.
//! - **Form submit** — intercepts site-form submissions (`action^="/f/"`),
//!   posts them urlencoded via `fetch`, and on success replaces the form
//!   with its `data-success` message (via `textContent` — never HTML). Any
//!   failure falls back to a native submit, so the server's response page
//!   handles errors; programmatic `submit()` does not re-fire the listener.
//! - **Analytics beacon** — reports the browser-only dimensions: a read-time
//!   bucket, outbound-link domains, click and scroll positions, and whether a
//!   conversion point was seen or begun ([`BEACON_SCRIPT`]). A visitor who
//!   runs no scripts is still counted as a page view by the server; only these
//!   dimensions go unreported.

/// The inline script block, terminated by a newline like every other
/// top-level fragment. Contains no `</script>` sequence and never touches
/// user-authored content.
pub(super) const BEHAVIOR_SCRIPT: &str = r#"<script>(function () {
  "use strict";
  document.documentElement.classList.add("js");
  document.querySelectorAll(".nav-toggle").forEach(function (toggle) {
    var menu = document.getElementById(toggle.getAttribute("aria-controls"));
    function closeMenu(restoreFocus) {
      toggle.setAttribute("aria-expanded", "false");
      if (restoreFocus) { toggle.focus(); }
    }
    toggle.addEventListener("click", function () {
      var expanded = toggle.getAttribute("aria-expanded") === "true";
      toggle.setAttribute("aria-expanded", expanded ? "false" : "true");
    });
    toggle.closest("nav").addEventListener("keydown", function (event) {
      if (event.key === "Escape" && toggle.getAttribute("aria-expanded") === "true") {
        closeMenu(true);
      }
    });
    if (menu) {
      menu.querySelectorAll("a").forEach(function (link) {
        link.addEventListener("click", function () { closeMenu(false); });
      });
    }
  });
  document.querySelectorAll('form[action^="/f/"]').forEach(function (form) {
    form.addEventListener("submit", function (event) {
      event.preventDefault();
      fetch(form.getAttribute("action"), {
        method: "POST",
        body: new URLSearchParams(new FormData(form))
      }).then(function (response) {
        if (!response.ok) { form.submit(); return; }
        var done = document.createElement("p");
        done.className = "form-success";
        done.textContent = form.getAttribute("data-success");
        form.replaceWith(done);
      }).catch(function () { form.submit(); });
    });
  });
  if ("IntersectionObserver" in window) {
    document.querySelectorAll(".s-transition").forEach(function (marker) {
      var target = marker.nextElementSibling;
      while (target && target.classList.contains("s-transition")) { target = target.nextElementSibling; }
      if (!target) { return; }
      target.classList.add("alo-transition", "alo-transition-" + marker.dataset.effect, "alo-from-" + marker.dataset.direction, "alo-speed-" + marker.dataset.speed);
      var margins = {early:"0px 0px 5%",balanced:"0px 0px -15%",late:"0px 0px -35%"};
      var repeat = marker.dataset.out === "true";
      new IntersectionObserver(function (entries, observer) {
        entries.forEach(function (entry) {
          target.classList.toggle("is-visible", entry.isIntersecting);
          if (entry.isIntersecting && !repeat) { observer.unobserve(target); }
        });
      }, {rootMargin:margins[marker.dataset.trigger] || margins.balanced,threshold:.08}).observe(target);
    });
  }
})();</script>
"#;

/// The analytics beacon, appended to every **published** page (never to the
/// authenticated draft preview, the unlock screen, or the 404 page).
///
/// It reports the traffic facts a server cannot see for itself: how long the
/// page stayed readable, which outside domain a visitor followed a link to,
/// where the page was clicked, how far down it was read, and whether a
/// conversion point on the page was reached or begun. Everything else about a
/// visit is already derived from the request at the door
/// (`crate::serve::analytics`) — and stays derived there, because a script is
/// easier to lie to than a socket.
///
/// What it deliberately does not do:
///
/// - **It carries no identity.** No cookie, no storage, no id of any kind —
///   not even the opaque daily token page views are counted with. The collect
///   endpoint cannot join two beacons from one browser, by construction.
/// - **It names no page for the read time.** A read time is a fact about the
///   site's day, not about `/prices` at 14:03. A heatmap event is the one
///   report that must name its page — an overlay is drawn over one page — and
///   it names nothing else: a click is sent as a position in permille of the
///   page, which the door reduces to one cell of a coarse grid.
/// - **It sends a size, never a screen.** The viewport width goes with a
///   heatmap event because a layout that reflows makes a shared grid
///   meaningless; the door reduces it to one of three classes and drops the
///   number.
/// - **It reports the read time once**, when the page is first hidden or
///   unloaded — "how long they read before looking away". A visitor who comes
///   back and reads on is not counted twice. The scroll depth goes with it,
///   and only when it is deeper than the last one sent.
/// - **It reports at most twenty clicks per page view**, so a page nobody can
///   stop clicking costs the endpoint twenty beacons and not twenty thousand.
/// - **The only id it ever sends is the site's own.** A conversion report
///   carries the form id the page's markup already published
///   (`<form action="/f/{id}">`) and one of two stage words — seen, or begun.
///   It never claims the third stage: a submit is counted where the submission
///   is written (`crate::serve::forms`), so no script can inflate the one
///   number an owner is most likely to act on.
/// - **It is not required for a page view to count.** Views are recorded by
///   the server; a visitor with scripting switched off is fully counted, minus
///   these browser-only dimensions.
///
/// Like [`BEHAVIOR_SCRIPT`] this is a static constant with zero user data
/// interpolated, which is what makes inlining it XSS-safe.
pub(crate) const BEACON_SCRIPT: &str = r#"<script>(function () {
  "use strict";
  var since = Date.now();
  var read = 0;
  var reported = false;
  var clicks = 0;
  var depth = 0;
  var page = "&p=" + encodeURIComponent(location.pathname) + "&w=";
  function send(body) {
    if (navigator.sendBeacon) { navigator.sendBeacon("/_alo/collect", body); }
  }
  function permille(value, total) {
    return total > 0 ? Math.max(0, Math.min(1000, Math.round((value / total) * 1000))) : 0;
  }
  function height() {
    var body = document.body;
    return Math.max(document.documentElement.scrollHeight, body ? body.scrollHeight : 0);
  }
  function shape() { return page + Math.round(window.innerWidth || 0); }
  function record() {
    read += Date.now() - since;
    since = Date.now();
    var reach = permille(window.scrollY + window.innerHeight, height());
    if (reach > depth) { depth = reach; send("d=" + reach + shape()); }
    if (reported) { return; }
    reported = true;
    send("t=" + Math.round(read / 1000));
  }
  document.addEventListener("visibilitychange", function () {
    if (document.visibilityState === "hidden") { record(); } else { since = Date.now(); }
  });
  window.addEventListener("pagehide", record);
  document.addEventListener("click", function (event) {
    var target = event.target;
    var link = target && target.closest ? target.closest("a[href]") : null;
    if (link && link.hostname && link.hostname !== location.hostname) {
      send("o=" + encodeURIComponent(link.hostname));
    }
    if (clicks < 20 && typeof event.pageX === "number") {
      clicks += 1;
      send("x=" + permille(event.pageX, document.documentElement.scrollWidth) +
        "&y=" + permille(event.pageY, height()) + shape());
    }
  }, true);
  document.querySelectorAll('form[action^="/f/"]').forEach(function (form) {
    var point = "c=" + encodeURIComponent(form.getAttribute("action").slice(3));
    send(point + "&s=view");
    form.addEventListener("input", function () { send(point + "&s=start"); }, { once: true });
  });
})();</script>
"#;

/// The outline that says "this is a text field" in the editable draft preview
/// — appended only there, never to a published page.
///
/// It uses `currentColor` and no colour of its own, so it inherits whatever
/// contrast the tenant's own theme already achieves against its background
/// instead of inventing a grey that may fail on a dark site. Focus gets a
/// solid two-pixel ring: the same affordance the app's own fields have, and
/// the one a keyboard user needs to know where they are.
///
/// Sections get the same treatment one level out, plus a grab cursor — the
/// affordance that says a section can be picked up (S3.01b). **Every
/// declaration here is layout-neutral by construction**: `outline`, `cursor`
/// and `opacity` change no box on the page. Nothing sets `position`, and that
/// is deliberate rather than an omission — `position:relative` on a section
/// would become the containing block of any absolutely positioned descendant,
/// and a preview that lays a page out differently from the published one is a
/// preview that lies.
pub(super) const EDIT_STYLE: &str = r#"<style>[data-alo-text]{outline:1px dashed color-mix(in srgb,currentColor 40%,transparent);outline-offset:3px;border-radius:2px}[data-alo-text]:hover{outline-style:solid}[data-alo-text]:focus{outline:2px solid currentColor;outline-offset:3px}[data-alo-text]:focus-visible{outline:2px solid currentColor}main>[data-alo-section]{cursor:grab}main>[data-alo-section]:hover{outline:1px dashed color-mix(in srgb,currentColor 30%,transparent);outline-offset:6px}main>[data-alo-section]:focus{outline:2px solid currentColor;outline-offset:6px}main>[data-alo-section].alo-moving{opacity:.55;cursor:grabbing}</style>
"#;

/// Direct manipulation, the page's half (ADR 0042): every element the renderer
/// marked with `data-alo-text` becomes a plain-text field, and a finished edit
/// is *reported* to the editor rather than saved here.
///
/// The document has no origin — it is `srcdoc` inside a `sandbox="allow-scripts"`
/// frame — so it cannot reach the API, and this script never tries: it posts
/// `{alo:"site-text-edit",key,text}` to the parent, which validates the key
/// against the sections it holds and applies the change through the same
/// guarded edit door a model's proposal goes through. `postMessage` must target
/// `"*"` because an opaque origin can name no other; the receiving side proves
/// the sender instead, by comparing it to its own frame's window.
///
/// What the gestures are, and why:
///
/// - **Enter commits and leaves.** These are single-line typed properties; a
///   newline in a heading is a surprise, not a paragraph. `Shift+Enter` still
///   inserts one for the properties where it means something.
/// - **Escape restores** what was there on focus, so a mistyped headline is
///   one key away from unchanged — undo without a round trip.
/// - **Blur commits** whatever is different, which is what clicking on the
///   next thing you want to edit means.
/// - **Links and forms do nothing.** In a preview a navigation cannot arrive
///   anywhere (there is no origin behind it) and would silently discard the
///   edit in progress; the published page keeps every one of them.
///
/// # Moving a section (S3.01b)
///
/// The second half of the same idea: a section is picked up on the page and
/// the page **reflows under the pointer** — the node really is moved in the
/// DOM on every `dragover`, so what is being previewed during the drag is the
/// arrangement itself, not a placeholder standing in for it. Only `<main>`'s
/// own children take part; the nav and the footer are landmarks, not stack
/// positions, and offering to drag them would offer an arrangement the
/// renderer will not honour.
///
/// - **The report is a neighbour, never a destination.** The frame posts
///   `{alo:"site-section-move",from,before}` where `before` is the *original*
///   index of the section the moved one now sits above (`null` at the end).
///   Turning that into a `reorder_section` destination is index arithmetic on
///   a list this document does not have — the editor holds the sections, so
///   the editor does the arithmetic, where it is unit-tested.
/// - **A press that starts on text is a text gesture.** `draggable` is set at
///   `mousedown` and only when the press did not begin inside an editable or
///   interactive element, so selecting a word in a headline never picks the
///   whole section up.
/// - **`Alt+ArrowUp`/`Alt+ArrowDown` is the keyboard equivalent**, on the
///   focused section itself (each is `tabindex="0"`). It is the same message
///   and therefore the same operation — there is no keyboard-only path to
///   drift.
///
/// # Resizing a section (S3.01c)
///
/// `Alt+ArrowLeft`/`Alt+ArrowRight` on the focused section steps its first
/// declared layout property — a two-column split between its allowed ratios,
/// a grid between its allowed column counts (`alo_store::site_layout`).
///
/// **What travels is a direction, never a size.**
/// `{alo:"site-section-layout",index,step}` where `step` is -1 or +1: this
/// document is never told what the values *are*, so no gesture inside it —
/// and no script that ever got into it — can name a ratio, a percentage or a
/// pixel. The editor resolves the direction against the server's declaration
/// and the section it is holding, which is where "the editor offers only the
/// declared values" is actually enforced (ADR 0042). The visible choices live
/// in the app beside the section, in the language of the person editing.
/// - **The words come from the app, not from here.** A section's accessible
///   name is posted in by the editor (`{alo:"site-edit-chrome",labels,focus}`)
///   because it is *editor* chrome: it must be in the language of the person
///   editing, which is not necessarily the language of the site. The same
///   message restores focus after a move, since applying one replaces this
///   whole document.
pub(super) const EDIT_SCRIPT: &str = r#"<script>(function () {
  "use strict";
  var MAX = 5000;
  var fields = document.querySelectorAll("[data-alo-text]");
  var original = null;
  function value(node) { return node.textContent.replace(/\s+$/, ""); }
  function commit(node) {
    var text = value(node);
    if (original === null || text === original || text.length > MAX) { return; }
    original = text;
    parent.postMessage({
      alo: "site-text-edit",
      key: node.getAttribute("data-alo-text"),
      text: text
    }, "*");
  }
  fields.forEach(function (node) {
    try { node.contentEditable = "plaintext-only"; } catch (ignored) { /* older engines */ }
    if (node.contentEditable !== "plaintext-only") { node.contentEditable = "true"; }
    node.spellcheck = true;
    node.addEventListener("focus", function () { original = value(node); });
    node.addEventListener("blur", function () { commit(node); original = null; });
    node.addEventListener("paste", function (event) {
      event.preventDefault();
      var source = event.clipboardData || window.clipboardData;
      var text = source ? source.getData("text/plain") : "";
      document.execCommand("insertText", false, text.replace(/[\r\n]+/g, " "));
    });
    node.addEventListener("keydown", function (event) {
      if (event.key === "Enter" && !event.shiftKey) { event.preventDefault(); node.blur(); }
      if (event.key === "Escape" && original !== null) {
        event.preventDefault();
        node.textContent = original;
        original = null;
        node.blur();
      }
    });
  });
  document.addEventListener("click", function (event) {
    if (event.target.closest && event.target.closest("a")) { event.preventDefault(); }
  }, true);
  document.addEventListener("submit", function (event) { event.preventDefault(); }, true);

  var main = document.querySelector("main");
  var moving = null;
  function blocks() {
    if (main === null) { return []; }
    return Array.prototype.filter.call(main.children, function (node) {
      return node.hasAttribute("data-alo-section");
    });
  }
  function at(node) { return Number(node.getAttribute("data-alo-section")); }
  function neighbour(list, position) {
    var node = list[position];
    return node ? at(node) : null;
  }
  function ask(node, before) {
    parent.postMessage({ alo: "site-section-move", from: at(node), before: before }, "*");
  }
  blocks().forEach(function (node) {
    node.tabIndex = 0;
    node.addEventListener("mousedown", function (event) {
      node.draggable = !(event.target.closest &&
        event.target.closest("[data-alo-text],a,button,input,textarea,select,iframe"));
    });
    node.addEventListener("dragstart", function (event) {
      moving = node;
      node.classList.add("alo-moving");
      if (event.dataTransfer) {
        event.dataTransfer.effectAllowed = "move";
        event.dataTransfer.setData("text/plain", node.getAttribute("data-alo-section"));
      }
    });
    node.addEventListener("dragover", function (event) {
      if (moving === null || node === moving) { return; }
      event.preventDefault();
      var box = node.getBoundingClientRect();
      main.insertBefore(
        moving,
        event.clientY > box.top + box.height / 2 ? node.nextSibling : node
      );
    });
    node.addEventListener("drop", function (event) { event.preventDefault(); });
    node.addEventListener("dragend", function () {
      node.classList.remove("alo-moving");
      node.draggable = false;
      if (moving !== node) { return; }
      moving = null;
      var list = blocks();
      ask(node, neighbour(list, list.indexOf(node) + 1));
    });
    node.addEventListener("keydown", function (event) {
      if (event.target !== node || !event.altKey) { return; }
      var list = blocks();
      var position = list.indexOf(node);
      if (event.key === "ArrowUp" && position > 0) {
        event.preventDefault();
        ask(node, at(list[position - 1]));
      } else if (event.key === "ArrowDown" && position < list.length - 1) {
        event.preventDefault();
        ask(node, neighbour(list, position + 2));
      } else if (event.key === "ArrowLeft" || event.key === "ArrowRight") {
        event.preventDefault();
        parent.postMessage({
          alo: "site-section-layout",
          index: at(node),
          step: event.key === "ArrowLeft" ? -1 : 1
        }, "*");
      }
    });
  });
  window.addEventListener("message", function (event) {
    var data = event.data;
    if (event.source !== parent || !data || data.alo !== "site-edit-chrome") { return; }
    blocks().forEach(function (node) {
      var label = data.labels ? data.labels[at(node)] : null;
      if (typeof label === "string") { node.setAttribute("aria-label", label); }
    });
    var wanted = typeof data.focus === "number" && main !== null
      ? main.querySelector('[data-alo-section="' + data.focus + '"]')
      : null;
    if (wanted !== null) { wanted.focus(); }
  });
})();</script>
"#;

#[cfg(test)]
mod tests {
    use super::*;

    /// The published page's whole JavaScript budget, pinned. "Near-zero JS"
    /// (`docs/design/sites.md`) is a promise about bytes a visitor downloads,
    /// and a promise nobody measures is a promise that erodes.
    #[test]
    fn the_page_scripts_stay_within_their_byte_budget() {
        assert!(
            BEHAVIOR_SCRIPT.len() < 3072,
            "behavior script is {} bytes",
            BEHAVIOR_SCRIPT.len()
        );
        assert!(
            BEACON_SCRIPT.len() < 2048,
            "beacon script is {} bytes",
            BEACON_SCRIPT.len()
        );
    }

    /// The edit script is not part of that budget — no visitor ever downloads
    /// it — but it is still inlined into a document, so it gets a ceiling of
    /// its own rather than none at all. Raised from 4 KB to 8 KB when moving a
    /// section joined typing on it (S3.01b): a ceiling is only honest while it
    /// is the number the thing actually needs.
    #[test]
    fn the_edit_script_stays_small_and_never_ships_to_a_visitor() {
        assert!(
            EDIT_SCRIPT.len() + EDIT_STYLE.len() < 8192,
            "edit mode is {} bytes",
            EDIT_SCRIPT.len() + EDIT_STYLE.len()
        );
        for script in [BEHAVIOR_SCRIPT, BEACON_SCRIPT] {
            assert!(!script.contains("data-alo-text"));
            assert!(!script.contains("data-alo-section"));
        }
    }

    /// Neither script may terminate its own block or interpolate anything —
    /// the reason inlining them is safe.
    #[test]
    fn neither_script_can_close_its_own_block() {
        for script in [BEHAVIOR_SCRIPT, BEACON_SCRIPT, EDIT_SCRIPT] {
            assert_eq!(script.matches("</script>").count(), 1);
            assert!(script.ends_with("</script>\n"));
        }
        assert_eq!(EDIT_STYLE.matches("</style>").count(), 1);
        assert!(EDIT_STYLE.ends_with("</style>\n"));
    }
}
