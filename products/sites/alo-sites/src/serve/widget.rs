//! The visitor assistant's on-page widget (ADR 0040, item S3.02e): a launcher
//! button and a chat panel appended to every **published** HTML document of a
//! site whose assistant is switched on — never to a site whose assistant is
//! off (those pages carry zero chat bytes, and `POST /_alo/chat` would 404
//! for them anyway).
//!
//! Like the page's other scripts ([`crate::render::script`]) the style and
//! behavior blocks are static constants with zero user data interpolated,
//! which is what makes inlining them XSS-safe. The *markup* is localized —
//! built from our own [`UiStrings`] constants per locale, HTML-escaped out of
//! hygiene — and every runtime string the script needs is carried as a
//! `data-*` attribute on the root, so the script itself never needs a
//! locale-specific byte.
//!
//! Privacy and honesty, by construction:
//!
//! - The widget holds one random **visitor token in memory per page load** —
//!   the rate-limit key [`super::chat`] requires. Nothing is written to
//!   cookies or storage; two page views are two visitors, deliberately.
//! - **Citations arrive from the server and render as links** only when they
//!   are site-relative paths (leading `/`); anything else renders as plain
//!   text. Answer text always enters the DOM via `textContent`, never HTML.
//! - Every state the wire can answer has words: thinking, an answer with its
//!   sources, a refusal, unavailable (with the site's own contact page when
//!   it has one), rate-limited, and a network failure.
//!
//! Accessibility: the launcher is a real `<button>` with `aria-expanded`/
//! `aria-controls`; the panel is a labelled `role="dialog"` whose log is
//! `role="log"` (polite live region); the question field has a visually
//! hidden `<label>`; Escape closes and returns focus to the launcher; at
//! phone widths the panel spans the viewport.

use crate::render::UiStrings;
use crate::render::html::esc;

/// The widget's stylesheet: theme tokens only (`--primary`, `--surface`, …
/// from the site's own generated stylesheet), so the widget wears the site's
/// preset palette by default (ADR 0040 §5) and never invents a colour.
const STYLE: &str = "<style>#alo-chat{position:fixed;right:1rem;bottom:1rem;z-index:90;font-family:var(--font-body,system-ui,sans-serif)}\
#alo-chat-open{background:var(--primary,#1a1a1a);color:var(--on-primary,#fff);border:0;border-radius:999px;padding:.75rem 1.25rem;font-size:1rem;cursor:pointer;box-shadow:0 2px 8px rgba(0,0,0,.25)}\
#alo-chat-open:focus-visible{outline:3px solid var(--text,#1a1a1a);outline-offset:2px}\
#alo-chat-panel{position:fixed;right:1rem;bottom:4.5rem;width:min(22rem,calc(100vw - 2rem));max-height:min(30rem,calc(100vh - 6rem));display:flex;flex-direction:column;background:var(--surface,#fff);color:var(--text,#1a1a1a);border:1px solid var(--border,#ddd);border-radius:12px;box-shadow:0 8px 32px rgba(0,0,0,.3);overflow:hidden}\
#alo-chat-panel[hidden]{display:none}\
.alo-chat-head{display:flex;align-items:center;justify-content:space-between;gap:.5rem;padding:.75rem 1rem;border-bottom:1px solid var(--border,#ddd)}\
.alo-chat-head h2{margin:0;font-size:1rem;font-family:var(--font-heading,inherit)}\
#alo-chat-close{background:none;border:0;color:inherit;font-size:1.25rem;line-height:1;cursor:pointer;padding:.25rem .5rem}\
#alo-chat-close:focus-visible,#alo-chat-q:focus-visible,.alo-chat-send:focus-visible{outline:2px solid var(--primary,#1a1a1a);outline-offset:2px}\
#alo-chat-log{flex:1;overflow-y:auto;padding:.75rem 1rem;display:flex;flex-direction:column;gap:.5rem;min-height:6rem}\
.alo-chat-msg{max-width:85%;padding:.5rem .75rem;border-radius:10px;font-size:.9375rem;white-space:pre-wrap;overflow-wrap:anywhere}\
.alo-chat-visitor{align-self:flex-end;background:var(--primary,#1a1a1a);color:var(--on-primary,#fff)}\
.alo-chat-bot{align-self:flex-start;background:var(--bg,#f5f5f5);border:1px solid var(--border,#ddd)}\
.alo-chat-cites{font-size:.8125rem;margin:.25rem 0 0}\
.alo-chat-cites a{color:inherit}\
#alo-chat-form{display:flex;gap:.5rem;padding:.75rem 1rem;border-top:1px solid var(--border,#ddd)}\
#alo-chat-q{flex:1;resize:none;border:1px solid var(--border,#ddd);border-radius:8px;padding:.5rem;font:inherit;background:var(--bg,#fff);color:var(--text,#1a1a1a)}\
.alo-chat-send{background:var(--primary,#1a1a1a);color:var(--on-primary,#fff);border:0;border-radius:8px;padding:.5rem .9rem;font:inherit;cursor:pointer}\
.alo-chat-hide{position:absolute;width:1px;height:1px;overflow:hidden;clip:rect(0 0 0 0);white-space:nowrap}\
@media (max-width:480px){#alo-chat-panel{right:.5rem;left:.5rem;width:auto}}</style>\n";

/// The widget's behavior. Static — every word it shows is read back off the
/// root element's `data-*` attributes, and everything a server reply carries
/// enters the DOM as text (or as a link only when it is a site-relative
/// path). Contains no `</script>` sequence beyond its own terminator.
const SCRIPT: &str = r#"<script>(function () {
  "use strict";
  var root = document.getElementById("alo-chat");
  if (root === null) { return; }
  var open = document.getElementById("alo-chat-open");
  var panel = document.getElementById("alo-chat-panel");
  var close = document.getElementById("alo-chat-close");
  var log = document.getElementById("alo-chat-log");
  var form = document.getElementById("alo-chat-form");
  var field = document.getElementById("alo-chat-q");
  var busy = false;
  var token = (function () {
    var bytes = new Uint8Array(16);
    if (window.crypto && crypto.getRandomValues) { crypto.getRandomValues(bytes); }
    else { for (var i = 0; i < 16; i++) { bytes[i] = Math.floor(Math.random() * 256); } }
    var out = "";
    for (var j = 0; j < 16; j++) { out += (bytes[j] + 256).toString(16).slice(1); }
    return out;
  })();
  function word(name) { return root.getAttribute("data-" + name) || ""; }
  function bubble(kind) {
    var msg = document.createElement("div");
    msg.className = "alo-chat-msg alo-chat-" + kind;
    log.appendChild(msg);
    return msg;
  }
  function offer(msg, path) {
    if (typeof path !== "string" || path.charAt(0) !== "/") { return; }
    msg.appendChild(document.createTextNode(" "));
    var link = document.createElement("a");
    link.href = path;
    link.textContent = word("contact");
    msg.appendChild(link);
  }
  function cite(msg, list) {
    if (!list || !list.length) { return; }
    var line = document.createElement("p");
    line.className = "alo-chat-cites";
    line.appendChild(document.createTextNode(word("sources") + " "));
    list.forEach(function (source, position) {
      if (position > 0) { line.appendChild(document.createTextNode(" · ")); }
      var title = typeof source.title === "string" ? source.title : "";
      if (typeof source.path === "string" && source.path.charAt(0) === "/") {
        var link = document.createElement("a");
        link.href = source.path;
        link.textContent = title;
        line.appendChild(link);
      } else {
        line.appendChild(document.createTextNode(title));
      }
    });
    msg.appendChild(line);
  }
  function toggle(show) {
    panel.hidden = !show;
    open.setAttribute("aria-expanded", show ? "true" : "false");
    if (show) { field.focus(); } else { open.focus(); }
  }
  open.addEventListener("click", function () { toggle(panel.hidden); });
  close.addEventListener("click", function () { toggle(false); });
  panel.addEventListener("keydown", function (event) {
    if (event.key === "Escape") { toggle(false); }
  });
  field.addEventListener("keydown", function (event) {
    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      if (form.requestSubmit) { form.requestSubmit(); }
    }
  });
  form.addEventListener("submit", function (event) {
    event.preventDefault();
    var question = field.value.trim();
    if (question === "" || busy) { return; }
    busy = true;
    field.value = "";
    bubble("visitor").textContent = question;
    var reply = bubble("bot");
    reply.textContent = word("thinking");
    log.scrollTop = log.scrollHeight;
    fetch("/_alo/chat", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ question: question, visitor: token })
    }).then(function (response) {
      if (response.status === 429) { return { state: "rate_limited" }; }
      return response.json();
    }).then(function (body) {
      busy = false;
      if (body && body.state === "answer" && typeof body.text === "string") {
        reply.textContent = body.text;
        cite(reply, body.citations);
      } else if (body && body.state === "refusal") {
        reply.textContent = word("refusal");
        offer(reply, body.contactPath);
      } else if (body && body.state === "rate_limited") {
        reply.textContent = word("limited");
      } else if (body && body.state === "unavailable") {
        reply.textContent = word("unavailable");
        offer(reply, body.contactPath);
      } else {
        reply.textContent = word("error");
      }
      log.scrollTop = log.scrollHeight;
    }).catch(function () {
      busy = false;
      reply.textContent = word("error");
    });
  });
})();</script>
"#;

/// The complete widget fragment for one locale: localized markup carrying
/// the script's words as data attributes, then the static style and script.
#[must_use]
pub(super) fn fragment(strings: &UiStrings) -> String {
    format!(
        "<div id=\"alo-chat\" data-thinking=\"{thinking}\" data-sources=\"{sources}\" \
         data-refusal=\"{refusal}\" data-unavailable=\"{unavailable}\" data-contact=\"{contact}\" \
         data-limited=\"{limited}\" data-error=\"{error}\">\n\
         <button type=\"button\" id=\"alo-chat-open\" aria-expanded=\"false\" \
         aria-controls=\"alo-chat-panel\">{open}</button>\n\
         <section id=\"alo-chat-panel\" role=\"dialog\" aria-label=\"{title}\" hidden>\n\
         <header class=\"alo-chat-head\"><h2>{title}</h2>\
         <button type=\"button\" id=\"alo-chat-close\" aria-label=\"{close}\">&#215;</button></header>\n\
         <div id=\"alo-chat-log\" role=\"log\" aria-live=\"polite\"></div>\n\
         <form id=\"alo-chat-form\">\n\
         <label class=\"alo-chat-hide\" for=\"alo-chat-q\">{question}</label>\n\
         <textarea id=\"alo-chat-q\" rows=\"2\" maxlength=\"{cap}\" placeholder=\"{question}\" \
         required></textarea>\n\
         <button type=\"submit\" class=\"alo-chat-send\">{send}</button>\n\
         </form>\n</section>\n</div>\n{STYLE}{SCRIPT}",
        thinking = esc(strings.chat_thinking),
        sources = esc(strings.chat_sources),
        refusal = esc(strings.chat_refusal),
        unavailable = esc(strings.chat_unavailable),
        contact = esc(strings.chat_contact),
        limited = esc(strings.chat_rate_limited),
        error = esc(strings.chat_error),
        open = esc(strings.chat_open),
        title = esc(strings.chat_title),
        close = esc(strings.chat_close),
        question = esc(strings.chat_question),
        send = esc(strings.chat_send),
        cap = super::chat::CHAT_MAX_QUESTION_CHARS,
    )
}

/// Appends the widget to a complete rendered document, just inside `</body>`
/// — the same position the page's own scripts occupy. A document without a
/// `</body>` (never produced by our renderer) gets the fragment appended at
/// the end, which browsers hoist into the body anyway.
#[must_use]
pub(super) fn inject(document: String, fragment: &str) -> String {
    match document.rfind("</body>") {
        Some(position) => {
            let mut out = String::with_capacity(document.len() + fragment.len());
            out.push_str(&document[..position]);
            out.push_str(fragment);
            out.push_str(&document[position..]);
            out
        }
        None => document + fragment,
    }
}

/// [`inject`] when there is a fragment to inject; the document unchanged
/// otherwise — the shape the blog path threads through.
#[must_use]
pub(super) fn maybe_inject(document: String, fragment: Option<&str>) -> String {
    match fragment {
        Some(fragment) => inject(document, fragment),
        None => document,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::{EN, FR, NL};

    /// The widget is inlined into every published page of an assistant-on
    /// site, so it gets a byte ceiling like the page's other scripts.
    #[test]
    fn the_widget_stays_within_its_byte_budget() {
        for strings in [&EN, &FR, &NL] {
            let fragment = fragment(strings);
            assert!(
                fragment.len() < 8192,
                "widget fragment is {} bytes for {}",
                fragment.len(),
                strings.lang
            );
        }
    }

    /// One script terminator, at the very end — the property that makes
    /// inlining beside user-authored section content safe.
    #[test]
    fn the_fragment_cannot_close_its_own_blocks_early() {
        for strings in [&EN, &FR, &NL] {
            let fragment = fragment(strings);
            assert_eq!(fragment.matches("</script>").count(), 1);
            assert!(fragment.ends_with("</script>\n"));
            assert_eq!(fragment.matches("</style>").count(), 1);
        }
    }

    /// The accessibility contract of the markup, pinned: real button
    /// semantics, a labelled dialog, a live log, and a labelled field.
    #[test]
    fn the_markup_keeps_its_accessible_bones() {
        let fragment = fragment(&EN);
        for needle in [
            "aria-expanded=\"false\"",
            "aria-controls=\"alo-chat-panel\"",
            "role=\"dialog\"",
            "aria-label=\"Ask us anything\"",
            "role=\"log\"",
            "aria-live=\"polite\"",
            "<label class=\"alo-chat-hide\" for=\"alo-chat-q\">",
            "maxlength=\"2000\"",
        ] {
            assert!(fragment.contains(needle), "missing {needle}");
        }
    }

    #[test]
    fn injection_lands_inside_the_body() {
        let document = "<!doctype html>\n<html><body><main>x</main></body>\n</html>\n".to_owned();
        let out = inject(document, "<div id=\"alo-chat\"></div>");
        assert!(out.contains("</main><div id=\"alo-chat\"></div></body>"));
        assert_eq!(out.matches("alo-chat").count(), 1);
    }
}
