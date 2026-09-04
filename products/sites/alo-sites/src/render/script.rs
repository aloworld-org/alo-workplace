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
//! - **Auto-hiding navigation** — hides an opted-in header while scrolling
//!   down and restores it while scrolling up, without trapping an open menu.
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
  document.querySelectorAll(".nav-behavior-auto-hide").forEach(function (nav) {
    var previous = window.scrollY;
    window.addEventListener("scroll", function () {
      var current = window.scrollY;
      var menuOpen = nav.querySelector('.nav-toggle[aria-expanded="true"]');
      nav.classList.toggle("is-nav-hidden", !menuOpen && current > previous && current > nav.offsetHeight * 2);
      previous = current;
    }, {passive:true});
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
    document.querySelectorAll(".section-motion").forEach(function (target) {
      new IntersectionObserver(function (entries, observer) {
        if (entries[0].isIntersecting) { target.classList.add("is-visible"); observer.unobserve(target); }
      }, {rootMargin:"0px 0px -12%",threshold:.08}).observe(target);
    });
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
pub(super) const EDIT_STYLE: &str = r#"<style>[data-alo-text]{outline:1px dashed color-mix(in srgb,currentColor 40%,transparent);outline-offset:3px;border-radius:2px}[data-alo-text]:hover{outline-style:solid}[data-alo-text]:focus{outline:2px solid currentColor;outline-offset:3px}[data-alo-text]:focus-visible{outline:2px solid currentColor}main>[data-alo-section]{cursor:grab}main>[data-alo-section]:hover{outline:1px dashed color-mix(in srgb,currentColor 30%,transparent);outline-offset:6px}main>[data-alo-section]:focus{outline:2px solid currentColor;outline-offset:6px}main>[data-alo-section].alo-moving{opacity:.55;cursor:grabbing}.alo-canvas-selected{outline:2px solid #e76f51!important;outline-offset:-2px}.alo-canvas-media{cursor:move!important;outline:2px solid #e76f51;outline-offset:4px}.alo-canvas-tools{position:fixed;z-index:999;display:flex;align-items:center;gap:12px;max-width:calc(100% - 24px);padding:10px 12px;border:1px solid #e8e3dc;border-radius:16px;background:#fffdfc;color:#102a43;box-shadow:0 12px 32px rgba(16,42,67,.16);font:14px/1.2 system-ui,sans-serif}.alo-tool-group{display:grid;gap:6px}.alo-tool-name{font-size:11px;font-weight:600;color:#746c62}.alo-tool-buttons{display:flex;align-items:center;gap:6px}.alo-canvas-tools button{min-width:40px;height:40px;padding:0 12px;border:1px solid transparent;border-radius:10px;background:transparent;color:#102a43;font:600 13px/1 system-ui,sans-serif;cursor:pointer}.alo-canvas-tools button:hover,.alo-canvas-tools button:focus-visible{border-color:#e8e3dc;background:#fce9e3;color:#c9573d;outline:none}.alo-canvas-tools .alo-swatch{width:36px;min-width:36px;height:36px;padding:0;border:2px solid #fffdfc;border-radius:50%;box-shadow:0 0 0 1px #e8e3dc}.alo-canvas-tools .alo-close{border-color:#e8e3dc;background:#f1eee8}.av{display:flex;width:30px;height:22px;flex-direction:column;justify-content:center;gap:3px}.av i{display:block;height:3px;border-radius:3px;background:currentColor;opacity:.8}.av i:first-child{width:100%}.av i:nth-child(2){width:72%}.av i:last-child{width:86%}.al{align-items:flex-start}.ac{align-items:center}.ar{align-items:flex-end}.wn{width:18px}.wb{width:26px}.ww{width:34px}</style>
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
    node.contentEditable = "true";
    node.spellcheck = true;
    node.addEventListener("focus", function () {
      original = value(node);
      var section = node.closest("[data-alo-section]");
      if (section && section.classList.contains("s-hero")) {
        var target = node.getAttribute("data-alo-text").indexOf("/heading") > -1 ? "heading" : "description";
        openCanvasTools(section, target);
        parent.postMessage({ alo: "site-section-quick-edit", index: at(section), target: target }, "*");
      }
    });
    node.addEventListener("mouseup", function () {
      var selection = window.getSelection();
      if (!selection || selection.isCollapsed) { return; }
      var section = node.closest("[data-alo-section]");
      if (section && section.classList.contains("s-hero")) {
        openCanvasTools(section, node.getAttribute("data-alo-text").indexOf("/heading") > -1 ? "heading" : "description");
      }
    });
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
  var canvasLabels = {};
  var canvasState = {};
  var activeText = null;
  var selectionFrame = 0;
  function closeCanvasTools() {
    document.querySelectorAll(".alo-canvas-tools").forEach(function (tool) { tool.remove(); });
    document.querySelectorAll(".alo-canvas-selected").forEach(function (item) { item.classList.remove("alo-canvas-selected"); });
    document.querySelectorAll(".alo-canvas-media").forEach(function (item) { item.classList.remove("alo-canvas-media"); });
  }
  function canvasGroup(toolbar, label) {
    var group = document.createElement("div"); group.className = "alo-tool-group";
    group.style.gap = "7px";
    var name = document.createElement("span"); name.className = "alo-tool-name"; name.textContent = label;
    name.style.fontSize = "12px";
    var buttons = document.createElement("div"); buttons.className = "alo-tool-buttons";
    group.appendChild(name); group.appendChild(buttons); toolbar.appendChild(group); return buttons;
  }
  function canvasButton(toolbar, index, action, glyph, label, visibleLabel) {
    var button = document.createElement("button");
    button.type = "button"; button.textContent = visibleLabel ? (glyph ? glyph + "  " + label : label) : glyph;
    button.setAttribute("aria-label", label || action); button.title = label || action;
    if (action === "align_" + canvasState.alignment || action === "width_" + canvasState.width || action === "background_" + canvasState.background) {
      button.classList.add("alo-active"); button.setAttribute("aria-pressed", "true");
      button.style.borderColor = "rgb(239,182,167)"; button.style.background = "rgb(252,233,227)"; button.style.color = "rgb(185,75,52)";
    }
    button.addEventListener("click", function (event) {
      event.preventDefault(); event.stopPropagation();
      if (action === "done") {
        closeCanvasTools();
        parent.postMessage({ alo: "site-canvas-close" }, "*");
        return;
      }
      var media = document.querySelector(".alo-canvas-media");
      if (media) {
        if (action === "zoom_in") { media.style.transform = "scale(1.06)"; }
        if (action === "zoom_out") { media.style.transform = "scale(.94)"; }
        if (action.indexOf("move_") === 0) {
          var x = action === "move_left" ? -12 : action === "move_right" ? 12 : 0;
          var y = action === "move_up" ? -12 : action === "move_down" ? 12 : 0;
          media.style.transform = "translate(" + x + "px," + y + "px)";
        }
      }
      parent.postMessage({ alo: "site-hero-canvas-edit", index: index, action: action }, "*");
    });
    toolbar.appendChild(button);
    return button;
  }
  function textVisualButton(toolbar, index, action, label, visualClass) {
    var button = canvasButton(toolbar,index,action,"",label);
    var visual = document.createElement("span");
    visual.className = "av " + visualClass;
    visual.setAttribute("aria-hidden", "true");
    visual.appendChild(document.createElement("i"));
    visual.appendChild(document.createElement("i"));
    visual.appendChild(document.createElement("i"));
    button.appendChild(visual);
    return button;
  }
  function textButton(toolbar, command, glyph, label, value) {
    var button = document.createElement("button"); button.type = "button"; button.textContent = glyph;
    button.setAttribute("aria-label", label); button.title = label;
    button.addEventListener("pointerdown", function (event) { event.preventDefault(); event.stopPropagation(); });
    button.addEventListener("click", function (event) {
      event.preventDefault(); event.stopPropagation();
      if (activeText === null) { return; }
      activeText.focus(); document.execCommand("styleWithCSS", false, true);
      document.execCommand(command, false, value || null);
    });
    toolbar.appendChild(button); return button;
  }
  function textColorPicker(toolbar) {
    var picker = document.createElement("input"); picker.type = "color";
    picker.value = String.fromCharCode(35) + "102a43"; picker.setAttribute("aria-label", canvasLabels.customColor); picker.title = canvasLabels.customColor;
    picker.style.width = "40px"; picker.style.height = "40px"; picker.style.padding = "3px"; picker.style.border = "1px solid rgb(232,227,220)"; picker.style.borderRadius = "50%"; picker.style.background = "transparent"; picker.style.cursor = "pointer";
    var savedRange = null;
    picker.addEventListener("pointerdown", function () {
      var selection = window.getSelection(); savedRange = selection && selection.rangeCount ? selection.getRangeAt(0).cloneRange() : null;
    });
    picker.addEventListener("input", function () {
      if (activeText === null || savedRange === null) { return; }
      activeText.focus(); var selection = window.getSelection(); selection.removeAllRanges(); selection.addRange(savedRange);
      document.execCommand("styleWithCSS", false, true); document.execCommand("foreColor", false, picker.value);
    });
    toolbar.appendChild(picker);
  }
  function openCanvasTools(node, target) {
    closeCanvasTools(); node.classList.add("alo-canvas-selected");
    var toolbar = document.createElement("div"); toolbar.className = "alo-canvas-tools";
    var box = node.getBoundingClientRect();
    toolbar.style.top = Math.max(12, box.top + 12) + "px"; toolbar.style.right = "12px";
    toolbar.style.padding = "10px 12px"; toolbar.style.borderRadius = "16px";
    toolbar.style.flexWrap = "nowrap"; toolbar.style.alignItems = "flex-end"; toolbar.style.gap = "12px";
    toolbar.addEventListener("pointerdown", function (event) { event.stopPropagation(); });
    var index = at(node);
    toolbar.dataset.index = String(index); toolbar.dataset.target = target;
    if (target === "media") {
      toolbar.style.flexDirection = "column"; toolbar.style.alignItems = "stretch"; toolbar.style.paddingRight = "64px";
      var media = node.querySelector("figure,img,video");
      if (media) {
        media.classList.add("alo-canvas-media");
        media.onpointerdown = function (event) {
          event.preventDefault(); event.stopPropagation();
          var sx = event.clientX, sy = event.clientY;
          media.setPointerCapture(event.pointerId);
          media.onpointermove = function (move) {
            media.style.transform = "translate(" + (move.clientX - sx) + "px," + (move.clientY - sy) + "px)";
          };
          media.onpointerup = function (up) {
            media.onpointermove = null;
            var dx = up.clientX - sx, dy = up.clientY - sy;
            if (Math.max(Math.abs(dx), Math.abs(dy)) < 8) { return; }
            var action = Math.abs(dx) > Math.abs(dy)
              ? (dx < 0 ? "move_left" : "move_right")
              : (dy < 0 ? "move_up" : "move_down");
            parent.postMessage({ alo: "site-hero-canvas-edit", index: index, action: action }, "*");
          };
        };
      }
      var position = canvasGroup(toolbar, canvasLabels.position);
      position.parentElement.style.gridTemplateColumns = "104px auto"; position.parentElement.style.alignItems = "center";
      canvasButton(position,index,"move_left","←",canvasLabels.moveLeft);
      canvasButton(position,index,"move_up","↑",canvasLabels.moveUp);
      canvasButton(position,index,"move_down","↓",canvasLabels.moveDown);
      canvasButton(position,index,"move_right","→",canvasLabels.moveRight);
      var size = canvasGroup(toolbar, canvasLabels.size);
      size.parentElement.style.gridTemplateColumns = "104px auto"; size.parentElement.style.alignItems = "center";
      canvasButton(size,index,"zoom_out","−",canvasLabels.zoomOut);
      canvasButton(size,index,"zoom_in","+",canvasLabels.zoomIn);
    } else if (target === "heading" || target === "description") {
      activeText = node.querySelector(target === "heading" ? '[data-alo-text*="/heading"]' : '[data-alo-text*="/subheading"]');
      var selection = window.getSelection();
      if (selection && !selection.isCollapsed && activeText && activeText.contains(selection.anchorNode) && activeText.contains(selection.focusNode)) {
        toolbar.dataset.mode = "selection";
        var formatting = canvasGroup(toolbar, canvasLabels.formatting);
        textButton(formatting,"bold","B",canvasLabels.bold).style.fontWeight = "800";
        textButton(formatting,"italic","I",canvasLabels.italic).style.fontStyle = "italic";
        textButton(formatting,"underline","U",canvasLabels.underline).style.textDecoration = "underline";
        var textColors = canvasGroup(toolbar, canvasLabels.textColor);
        [["var(--text)",canvasLabels.dark],["var(--accent)",canvasLabels.accent],["var(--muted)",canvasLabels.surface]].forEach(function (choice) {
          var color = getComputedStyle(document.documentElement).getPropertyValue(choice[0].slice(4,-1)).trim();
          var swatch = textButton(textColors,"foreColor","",choice[1],color); swatch.classList.add("alo-swatch"); swatch.style.background = color;
        });
        textColorPicker(textColors);
      } else {
        toolbar.dataset.mode = "layout";
        var textAlignment = canvasGroup(toolbar, target === "heading" ? canvasLabels.heading : canvasLabels.description);
        textVisualButton(textAlignment,index,"align_left",canvasLabels.alignLeft,"al");
        textVisualButton(textAlignment,index,"align_center",canvasLabels.alignCenter,"ac");
        textVisualButton(textAlignment,index,"align_right",canvasLabels.alignRight,"ar");
        var width = canvasGroup(toolbar, canvasLabels.textWidth);
        textVisualButton(width,index,"width_narrow",canvasLabels.narrow,"ac wn");
        textVisualButton(width,index,"width_balanced",canvasLabels.balanced,"ac wb");
        textVisualButton(width,index,"width_wide",canvasLabels.wide,"ac ww");
      }
    } else {
      var alignment = canvasGroup(toolbar, canvasLabels.alignment);
      canvasButton(alignment,index,"align_left","",canvasLabels.alignLeft,true);
      canvasButton(alignment,index,"align_center","",canvasLabels.alignCenter,true);
      canvasButton(alignment,index,"align_right","",canvasLabels.alignRight,true);
      var colors = canvasGroup(toolbar, canvasLabels.colors);
      [["background_background","var(--background)",canvasLabels.background],["background_accent_3","var(--accent-3)",canvasLabels.surface],["background_accent_1","var(--accent)",canvasLabels.accent],["background_text","var(--text)",canvasLabels.dark]].forEach(function (choice) {
        var swatch = canvasButton(colors,index,choice[0],"",choice[2]);
        swatch.classList.add("alo-swatch"); swatch.style.background = choice[1];
      });
    }
    var close = canvasButton(toolbar,index,"done","✓",canvasLabels.done); close.classList.add("alo-close");
    if (target === "media") { close.style.position = "absolute"; close.style.top = "12px"; close.style.right = "12px"; }
    document.body.appendChild(toolbar);
  }
  document.addEventListener("selectionchange", function () {
    cancelAnimationFrame(selectionFrame);
    selectionFrame = requestAnimationFrame(function () {
      var selection = window.getSelection();
      if (!selection || selection.isCollapsed || activeText === null || !activeText.contains(selection.anchorNode) || !activeText.contains(selection.focusNode)) { return; }
      var section = activeText.closest("[data-alo-section]"); if (!section) { return; }
      var target = activeText.getAttribute("data-alo-text").indexOf("/heading") > -1 ? "heading" : "description";
      var shown = document.querySelector('.alo-canvas-tools[data-mode="selection"]');
      if (shown && shown.dataset.index === String(at(section)) && shown.dataset.target === target) { return; }
      openCanvasTools(section, target);
    });
  });
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
    node.addEventListener("dblclick", function (event) {
      if (event.target.closest &&
          event.target.closest("[data-alo-text],a,button,input,textarea,select,iframe")) {
        return;
      }
      event.preventDefault();
      openCanvasTools(node, event.target.closest && event.target.closest("figure,img,video") ? "media" : "section");
      parent.postMessage({
        alo: "site-section-quick-edit",
        index: at(node),
        target: event.target.closest && event.target.closest("figure,img,video")
          ? "media"
          : "section"
      }, "*");
    });
    node.addEventListener("mousedown", function (event) {
      node.draggable = !(event.target.closest &&
        event.target.closest("[data-alo-text],a,button,input,textarea,select,iframe,.alo-canvas-media,.alo-canvas-tools"));
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
    canvasLabels = data.canvas || {};
    canvasState = data.canvasState || {};
    blocks().forEach(function (node) {
      var label = data.labels ? data.labels[at(node)] : null;
      if (typeof label === "string") { node.setAttribute("aria-label", label); }
    });
    var wanted = typeof data.focus === "number" && main !== null
      ? main.querySelector('[data-alo-section="' + data.focus + '"]')
      : null;
    if (wanted !== null) { wanted.focus(); }
    var selected = typeof data.canvasSelection === "object" && data.canvasSelection !== null
      ? main.querySelector('[data-alo-section="' + data.canvasSelection.index + '"]')
      : null;
    if (selected !== null) { openCanvasTools(selected, data.canvasSelection.target); }
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
            BEHAVIOR_SCRIPT.len() < 4096,
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
    /// its own rather than none at all. Raised to 20 KB when image framing and
    /// Hero styling moved onto the canvas with labeled, grouped controls and
    /// visible selected states, plus selection-preserving text and colour controls;
    /// visitor script budgets remain unchanged. A ceiling is only honest while
    /// it is measured.
    #[test]
    fn the_edit_script_stays_small_and_never_ships_to_a_visitor() {
        assert!(
            EDIT_SCRIPT.len() + EDIT_STYLE.len() < 20_480,
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
