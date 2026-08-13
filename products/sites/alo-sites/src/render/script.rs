//! The two scripts of a rendered page: the behavior script (menu toggle +
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
    toggle.addEventListener("click", function () {
      var expanded = toggle.getAttribute("aria-expanded") === "true";
      toggle.setAttribute("aria-expanded", expanded ? "false" : "true");
    });
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
pub(super) const EDIT_STYLE: &str = r#"<style>[data-alo-text]{outline:1px dashed color-mix(in srgb,currentColor 40%,transparent);outline-offset:3px;border-radius:2px}[data-alo-text]:hover{outline-style:solid}[data-alo-text]:focus{outline:2px solid currentColor;outline-offset:3px}[data-alo-text]:focus-visible{outline:2px solid currentColor}</style>
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
pub(super) const EDIT_SCRIPT: &str = r#"<script>(function () {
  "use strict";
  var MAX = 5000;
  var fields = document.querySelectorAll("[data-alo-text]");
  if (!fields.length) { return; }
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
            BEHAVIOR_SCRIPT.len() < 2048,
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
    /// its own rather than none at all.
    #[test]
    fn the_edit_script_stays_small_and_never_ships_to_a_visitor() {
        assert!(
            EDIT_SCRIPT.len() + EDIT_STYLE.len() < 4096,
            "edit mode is {} bytes",
            EDIT_SCRIPT.len() + EDIT_STYLE.len()
        );
        assert!(!BEHAVIOR_SCRIPT.contains("data-alo-text"));
        assert!(!BEACON_SCRIPT.contains("data-alo-text"));
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
