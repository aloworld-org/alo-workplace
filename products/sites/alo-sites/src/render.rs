//! Page JSON + theme → one complete HTML document (pure, infallible).
//!
//! The write gate (`alo_store::site_model`) guarantees everything stored is
//! valid, so rendering has no failure path: a section this build cannot parse
//! is **skipped with a `tracing` warning**, never a 500 — an old renderer
//! must survive a newer snapshot mid-deploy. Independently of write-side
//! validation, every text and attribute value is escaped and every href is
//! re-checked here (defense in depth): even a hostile stored value renders as
//! inert text.
//!
//! Landmark rule: `nav` sections render as `<header>` blocks before the one
//! `<main>`, `footer` sections as `<footer>` blocks after it, and everything
//! else inside `<main>` in author order. A nav authored mid-page therefore
//! still lands in the header region — the document stays valid and navigable
//! for assistive technology, which outranks literal ordering.

pub(crate) mod html;
mod script;
mod sections;
mod strings;

pub use strings::{EN, FR, NL, UiStrings, strings_for};

use alo_store::SiteCollectionSnapshot;
use alo_store::site_model::{SECTIONS_SCHEMA_VERSION, Section};
use alo_store::site_theme::SiteTheme;

use html::{esc, img_src};

/// Site-level inputs of a render: everything that is true for every page.
#[derive(Debug, Clone, Copy)]
pub struct SiteRenderContext<'a> {
    /// The site's display name (nav brand fallback, `og:site_name`, title
    /// suffix).
    pub name: &'a str,
    /// Absolute origin the site is served on, no trailing slash
    /// (e.g. `https://nordwind.alosites.com`); used for canonical/OG URLs.
    pub base_url: &'a str,
    /// Exact normalized language tag for document and feed metadata.
    pub locale: &'a str,
    /// The site's theme (logo, favicon; the preset drives the stylesheet).
    pub theme: &'a SiteTheme,
    /// Visitor-facing chrome strings ([`EN`] until more locales ship).
    pub strings: &'a UiStrings,
    /// Where `<img>`/favicon references point (public paths on the served
    /// origin; inline data URIs in the self-contained draft preview).
    pub images: ImageSources<'a>,
}

/// How the document spells its image references. Public serving uses the
/// `/assets/img/<blob_id>` path contract; the authenticated draft preview —
/// a self-contained document in a sandboxed iframe, where those paths do not
/// resolve — carries the bytes inline as `data:` URIs. `og:image` is exempt:
/// it is head metadata addressed to crawlers and always spells the absolute
/// public URL.
#[derive(Debug, Clone, Copy)]
pub enum ImageSources<'a> {
    /// `/assets/img/<blob_id>` — the crate-level public-path contract.
    PublicPaths,
    /// Blob id → `data:` URI. An id missing from the map falls back to the
    /// public path, so an unresolvable image degrades exactly like a public
    /// render rather than changing the document shape.
    Inline(&'a std::collections::HashMap<String, String>),
}

impl ImageSources<'_> {
    /// The attribute-ready `src` for a blob id.
    pub(crate) fn src(&self, blob_id: &str) -> String {
        match self {
            ImageSources::PublicPaths => img_src(blob_id),
            ImageSources::Inline(map) => map
                .get(blob_id)
                .map_or_else(|| img_src(blob_id), |uri| esc(uri)),
        }
    }
}

/// Page-level inputs of a render.
#[derive(Debug, Clone, Copy)]
pub struct PageRenderContext<'a> {
    /// Site-relative path of this page, starting with `/` (home is `/`).
    pub path: &'a str,
    /// The page's title.
    pub title: &'a str,
    /// SEO title override; when absent the title is
    /// `<page title> — <site name>`.
    pub seo_title: Option<&'a str>,
    /// SEO meta description; absent means no description/OG-description tags.
    pub seo_description: Option<&'a str>,
    /// The stored sections envelope (`{ "schema_version": …, "sections": … }`).
    pub sections: &'a serde_json::Value,
    /// Immutable Base-backed collections frozen with this publish, keyed by
    /// the stable id referenced by collection sections.
    pub collections: &'a std::collections::HashMap<String, SiteCollectionSnapshot>,
}

/// One exact translation of the current stable page identity.
#[derive(Debug, Clone, Copy)]
pub struct LanguageAlternate<'a> {
    pub locale: &'a str,
    pub path: &'a str,
    pub is_default: bool,
}

/// Reads a stored sections envelope leniently: entries that parse as a known
/// [`Section`] render; anything else is skipped with a warning. This is the
/// read-side tolerance the design note requires — the strict counterpart
/// (`SectionsEnvelope::from_value`) guards writes, never reads.
pub fn sections_lenient(stored: &serde_json::Value) -> Vec<Section> {
    if let Some(version) = stored
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
        && version != SECTIONS_SCHEMA_VERSION
    {
        tracing::warn!(
            version,
            speaks = SECTIONS_SCHEMA_VERSION,
            "rendering a sections envelope from a different schema version best-effort"
        );
    }
    let Some(entries) = stored.get("sections").and_then(serde_json::Value::as_array) else {
        tracing::warn!("stored sections value has no sections array; rendering an empty page");
        return Vec::new();
    };
    entries
        .iter()
        .filter_map(|entry| match serde_json::from_value(entry.clone()) {
            Ok(section) => Some(section),
            Err(error) => {
                let kind = entry
                    .get("type")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("<untagged>");
                tracing::warn!(section = kind, %error, "skipping unrenderable section");
                None
            }
        })
        .collect()
}

/// How the document references its stylesheet. Published pages link the
/// served `/assets/site.css`; the authenticated draft preview inlines the
/// generated sheet instead, because none of the public asset paths resolve
/// on the edit origin. Everything else about the document is identical —
/// one builder, so preview and production HTML cannot drift.
enum StylesheetRef<'a> {
    /// `<link rel="stylesheet" href="/assets/site.css">`.
    Linked,
    /// The generated stylesheet, embedded in a `<style>` block.
    Inline(&'a str),
}

/// Renders one page to a complete HTML document.
pub fn render_page(site: &SiteRenderContext<'_>, page: &PageRenderContext<'_>) -> String {
    render_document(site, page, &StylesheetRef::Linked, &[])
}

/// Renders a published localized page with direct language links and search
/// discovery metadata for the exact translations frozen beside it.
pub fn render_localized_page(
    site: &SiteRenderContext<'_>,
    page: &PageRenderContext<'_>,
    alternates: &[LanguageAlternate<'_>],
) -> String {
    render_document(site, page, &StylesheetRef::Linked, alternates)
}

/// Renders one page as a self-contained draft-preview document: the same
/// output as [`render_page`] with the stylesheet inlined in place of the
/// `/assets/site.css` link. `css` is the machine-generated sheet from
/// [`crate::stylesheet::stylesheet`] — generated purely from validated theme
/// tokens, it can never contain `</style` (the stylesheet suite pins that
/// mechanically), so embedding it verbatim is safe.
pub fn render_page_preview(
    site: &SiteRenderContext<'_>,
    page: &PageRenderContext<'_>,
    css: &str,
) -> String {
    render_document(site, page, &StylesheetRef::Inline(css), &[])
}

fn render_document(
    site: &SiteRenderContext<'_>,
    page: &PageRenderContext<'_>,
    stylesheet: &StylesheetRef<'_>,
    alternates: &[LanguageAlternate<'_>],
) -> String {
    let parsed = sections_lenient(page.sections);

    let mut header = String::new();
    let mut main = String::new();
    let mut footer = String::new();
    for (index, section) in parsed.iter().enumerate() {
        match section {
            Section::Nav(nav) => sections::nav(&mut header, site, nav, index),
            Section::Footer(f) => sections::footer(&mut footer, site, f),
            other => sections::body_section(&mut main, site, page, other, index),
        }
    }

    let mut out = String::with_capacity(16 * 1024);
    out.push_str("<!doctype html>\n");
    out.push_str(&format!("<html lang=\"{}\">\n", esc(site.locale)));
    push_head(&mut out, site, page, &parsed, stylesheet, alternates);
    out.push_str("<body>\n");
    out.push_str(&format!(
        "<a class=\"skip-link\" href=\"#main\">{}</a>\n",
        esc(site.strings.skip_to_content)
    ));
    push_language_switcher(&mut out, site, alternates);
    out.push_str(&header);
    out.push_str("<main id=\"main\">\n");
    out.push_str(&main);
    out.push_str("</main>\n");
    out.push_str(&footer);
    if wants_script(&parsed) {
        out.push_str(script::BEHAVIOR_SCRIPT);
    }
    out.push_str("</body>\n</html>\n");
    out
}

/// Renders a site's not-found document: the same chrome and stylesheet as
/// the site's pages, so a lost visitor stays inside the brand — heading,
/// explanation, and a link home, marked `noindex`. Page-agnostic by design:
/// the public service builds it once per publish and serves it (status 404)
/// for every unknown path on a live site.
pub fn render_not_found(site: &SiteRenderContext<'_>) -> String {
    let title = format!("{} — {}", site.strings.not_found_title, site.name);
    let mut out = String::with_capacity(2 * 1024);
    out.push_str("<!doctype html>\n");
    out.push_str(&format!("<html lang=\"{}\">\n", esc(site.locale)));
    out.push_str("<head>\n<meta charset=\"utf-8\">\n");
    out.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n");
    out.push_str(&format!("<title>{}</title>\n", esc(&title)));
    out.push_str("<meta name=\"robots\" content=\"noindex\">\n");
    if let Some(favicon) = &site.theme.favicon {
        out.push_str(&format!(
            "<link rel=\"icon\" href=\"{}\">\n",
            site.images.src(favicon.as_str())
        ));
    }
    out.push_str("<link rel=\"stylesheet\" href=\"/assets/site.css\">\n</head>\n<body>\n");
    out.push_str(&format!(
        "<a class=\"skip-link\" href=\"#main\">{}</a>\n",
        esc(site.strings.skip_to_content)
    ));
    out.push_str("<main id=\"main\">\n<section class=\"s-hero\">\n");
    out.push_str(&format!("<h1>{}</h1>\n", esc(site.strings.not_found_title)));
    out.push_str(&format!(
        "<p class=\"subheading\">{}</p>\n",
        esc(site.strings.not_found_text)
    ));
    out.push_str(&format!(
        "<p class=\"actions\"><a class=\"button\" href=\"/\">{}</a></p>\n",
        esc(site.strings.not_found_home)
    ));
    out.push_str("</section>\n</main>\n</body>\n</html>\n");
    out
}

/// Renders the unlock screen of a password-protected page (S2.06a): the
/// site's own chrome around one password field, posting back to the page's
/// own URL — so a visitor who gets the password simply lands on the page.
///
/// It carries the site's theme but none of the page's content, not even its
/// title: everything behind the password stays behind it, including the fact
/// that the page is called "Prices for Acme". Marked `noindex`, and no
/// JavaScript — the form works with scripting switched off.
///
/// `notice` is the one-line message above the field (a wrong password, too
/// many attempts) or `None` on the first ask; `path` is the page's own path,
/// which is the form's action and is always one of this publish's canonical
/// paths (never visitor input).
#[must_use]
pub fn render_password_challenge(
    site: &SiteRenderContext<'_>,
    path: &str,
    notice: Option<&str>,
) -> String {
    let title = format!("{} — {}", site.strings.protected_title, site.name);
    let mut out = String::with_capacity(2 * 1024);
    out.push_str("<!doctype html>\n");
    out.push_str(&format!("<html lang=\"{}\">\n", esc(site.locale)));
    out.push_str("<head>\n<meta charset=\"utf-8\">\n");
    out.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n");
    out.push_str(&format!("<title>{}</title>\n", esc(&title)));
    out.push_str("<meta name=\"robots\" content=\"noindex\">\n");
    if let Some(favicon) = &site.theme.favicon {
        out.push_str(&format!(
            "<link rel=\"icon\" href=\"{}\">\n",
            site.images.src(favicon.as_str())
        ));
    }
    out.push_str("<link rel=\"stylesheet\" href=\"/assets/site.css\">\n</head>\n<body>\n");
    out.push_str("<main id=\"main\">\n<section class=\"s-contact_form\">\n");
    out.push_str(&format!("<h1>{}</h1>\n", esc(site.strings.protected_title)));
    out.push_str(&format!(
        "<p class=\"subheading\">{}</p>\n",
        esc(site.strings.protected_text)
    ));
    if let Some(notice) = notice {
        out.push_str(&format!(
            "<p class=\"form-error\" role=\"alert\">{}</p>\n",
            esc(notice)
        ));
    }
    out.push_str(&format!(
        "<form method=\"post\" action=\"{}\">\n",
        esc(path)
    ));
    out.push_str(&format!(
        "<p><label for=\"site-password\">{}</label>\n",
        esc(site.strings.protected_password)
    ));
    out.push_str(
        "<input id=\"site-password\" name=\"password\" type=\"password\" \
         autocomplete=\"current-password\" required autofocus></p>\n",
    );
    out.push_str(&format!(
        "<p><button type=\"submit\">{}</button></p>\n",
        esc(site.strings.protected_open)
    ));
    out.push_str("</form>\n</section>\n</main>\n</body>\n</html>\n");
    out
}

/// Whether the page has anything for the behavior script to do: a nav (menu
/// toggle) or a form with a working submit. A page without either ships zero
/// JavaScript.
fn wants_script(sections: &[Section]) -> bool {
    sections.iter().any(|section| match section {
        Section::Nav(_) => true,
        Section::ContactForm(form) => form.form_id.is_some(),
        _ => false,
    })
}

/// The `<head>`: charset/viewport, title, description, canonical, OG tags,
/// favicon, and the one stylesheet.
fn push_head(
    out: &mut String,
    site: &SiteRenderContext<'_>,
    page: &PageRenderContext<'_>,
    parsed: &[Section],
    stylesheet: &StylesheetRef<'_>,
    alternates: &[LanguageAlternate<'_>],
) {
    let title = match page.seo_title {
        Some(seo) => seo.to_owned(),
        None => format!("{} — {}", page.title, site.name),
    };
    let canonical = format!("{}{}", site.base_url, page.path);

    out.push_str("<head>\n<meta charset=\"utf-8\">\n");
    out.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n");
    out.push_str(&format!("<title>{}</title>\n", esc(&title)));
    if let Some(description) = page.seo_description {
        out.push_str(&format!(
            "<meta name=\"description\" content=\"{}\">\n",
            esc(description)
        ));
    }
    out.push_str(&format!(
        "<link rel=\"canonical\" href=\"{}\">\n",
        esc(&canonical)
    ));
    for alternate in alternates {
        out.push_str(&format!(
            "<link rel=\"alternate\" hreflang=\"{}\" href=\"{}{}\">\n",
            esc(alternate.locale),
            esc(site.base_url),
            esc(alternate.path)
        ));
    }
    if let Some(default) = alternates.iter().find(|alternate| alternate.is_default) {
        out.push_str(&format!(
            "<link rel=\"alternate\" hreflang=\"x-default\" href=\"{}{}\">\n",
            esc(site.base_url),
            esc(default.path)
        ));
    }
    out.push_str("<meta property=\"og:type\" content=\"website\">\n");
    out.push_str(&format!(
        "<meta property=\"og:site_name\" content=\"{}\">\n",
        esc(site.name)
    ));
    out.push_str(&format!(
        "<meta property=\"og:title\" content=\"{}\">\n",
        esc(&title)
    ));
    if let Some(description) = page.seo_description {
        out.push_str(&format!(
            "<meta property=\"og:description\" content=\"{}\">\n",
            esc(description)
        ));
    }
    out.push_str(&format!(
        "<meta property=\"og:url\" content=\"{}\">\n",
        esc(&canonical)
    ));
    if let Some(blob) =
        first_hero_image(parsed).or_else(|| site.theme.logo.as_ref().map(alo_store::BlobId::as_str))
    {
        // og:image is crawler metadata: always the absolute public URL,
        // never an inline data URI (see `ImageSources`).
        out.push_str(&format!(
            "<meta property=\"og:image\" content=\"{}{}\">\n",
            esc(site.base_url),
            img_src(blob)
        ));
    }
    if let Some(favicon) = &site.theme.favicon {
        out.push_str(&format!(
            "<link rel=\"icon\" href=\"{}\">\n",
            site.images.src(favicon.as_str())
        ));
    }
    match stylesheet {
        StylesheetRef::Linked => {
            out.push_str("<link rel=\"stylesheet\" href=\"/assets/site.css\">\n");
        }
        StylesheetRef::Inline(css) => {
            out.push_str("<style>\n");
            out.push_str(css);
            out.push_str("</style>\n");
        }
    }
    out.push_str("</head>\n");
}

fn push_language_switcher(
    out: &mut String,
    site: &SiteRenderContext<'_>,
    alternates: &[LanguageAlternate<'_>],
) {
    if alternates.len() < 2 {
        return;
    }
    out.push_str(&format!(
        "<nav class=\"language-switcher\" aria-label=\"{}\">\n",
        esc(site.strings.language_switcher_label)
    ));
    for alternate in alternates {
        let current = alternate.locale == site.locale;
        out.push_str(&format!(
            "<a href=\"{}\" hreflang=\"{}\" lang=\"{}\"{}>{}</a>\n",
            esc(alternate.path),
            esc(alternate.locale),
            esc(alternate.locale),
            if current {
                " aria-current=\"page\""
            } else {
                ""
            },
            esc(&alternate.locale.to_uppercase())
        ));
    }
    out.push_str("</nav>\n");
}

/// The page's first-choice OG image: its first illustrated hero. The caller
/// falls back to the site logo so shared links still carry the brand when a
/// page has no hero artwork.
fn first_hero_image(sections: &[Section]) -> Option<&str> {
    sections.iter().find_map(|section| match section {
        Section::Hero(hero) => hero.image.as_ref().map(|image| image.blob_id.as_str()),
        _ => None,
    })
}
