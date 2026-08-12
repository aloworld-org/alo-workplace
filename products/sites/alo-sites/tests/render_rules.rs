//! Behavioral rules of the renderer: head metadata, landmark structure,
//! lenient reads (skip-with-log), and the escaping/href defenses that must
//! hold even for stored values the write gate would never have admitted.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use alo_sites::render::{
    EN, FR, ImageSources, NL, PageRenderContext, SiteRenderContext, render_page,
    render_page_preview, render_password_challenge, sections_lenient,
};
use alo_sites::stylesheet::stylesheet;
use alo_store::site_theme::{SiteTheme, THEME_PRESETS};
use serde_json::json;
use std::collections::HashMap;

/// Renders arbitrary stored-sections JSON on a default-theme site.
fn render(sections: &serde_json::Value) -> String {
    let theme = SiteTheme::new();
    render_with(&theme, sections, None, None)
}

fn render_with(
    theme: &SiteTheme,
    sections: &serde_json::Value,
    seo_title: Option<&str>,
    seo_description: Option<&str>,
) -> String {
    let site = SiteRenderContext {
        name: "Nordwind Coffee Roasters",
        base_url: "https://nordwind.alosites.com",
        locale: "en",
        theme,
        strings: &EN,
        images: ImageSources::PublicPaths,
    };
    let collections = HashMap::new();
    let page = PageRenderContext {
        path: "/about",
        title: "About",
        seo_title,
        seo_description,
        sections,
        collections: &collections,
        catalogs: &HashMap::new(),
    };
    render_page(&site, &page)
}

fn hero_page() -> serde_json::Value {
    json!({
        "schema_version": 1,
        "sections": [
            {"type": "nav", "links": [{"label": "Home", "href": "/"}]},
            {"type": "hero", "heading": "Hello", "image":
                {"blob_id": "9hK3vQ2mR8pT1xWz4bC5dg", "alt": "The drum"}},
            {"type": "footer", "text": "© Nordwind", "links": []},
        ]
    })
}

#[test]
fn head_carries_title_description_canonical_and_og() {
    let html = render_with(
        &SiteTheme::new(),
        &hero_page(),
        Some("About Nordwind"),
        Some("Who we are."),
    );
    for expected in [
        "<title>About Nordwind</title>",
        "<meta name=\"description\" content=\"Who we are.\">",
        "<link rel=\"canonical\" href=\"https://nordwind.alosites.com/about\">",
        "<meta property=\"og:type\" content=\"website\">",
        "<meta property=\"og:site_name\" content=\"Nordwind Coffee Roasters\">",
        "<meta property=\"og:title\" content=\"About Nordwind\">",
        "<meta property=\"og:description\" content=\"Who we are.\">",
        "<meta property=\"og:url\" content=\"https://nordwind.alosites.com/about\">",
        "<meta property=\"og:image\" content=\"https://nordwind.alosites.com/assets/img/9hK3vQ2mR8pT1xWz4bC5dg\">",
        "<link rel=\"stylesheet\" href=\"/assets/site.css\">",
        "<html lang=\"en\">",
    ] {
        assert!(html.contains(expected), "missing {expected} in:\n{html}");
    }
}

#[test]
fn default_title_joins_page_and_site_name_and_description_is_omitted() {
    let html = render(&hero_page());
    assert!(html.contains("<title>About — Nordwind Coffee Roasters</title>"));
    assert!(!html.contains("name=\"description\""));
    assert!(!html.contains("og:description"));
}

#[test]
fn landmarks_wrap_main_in_order_with_skip_link_first() {
    let html = render(&hero_page());
    let skip = html.find("class=\"skip-link\"").unwrap();
    let header = html.find("<header class=\"s-nav\">").unwrap();
    let main = html.find("<main id=\"main\">").unwrap();
    let footer = html.find("<footer class=\"s-footer\">").unwrap();
    assert!(skip < header && header < main && main < footer);
    // The hero lives inside main, not before it.
    assert!(html.find("<section class=\"s-hero\">").unwrap() > main);
}

#[test]
fn unknown_sections_and_newer_versions_render_best_effort() {
    let stored = json!({
        "schema_version": 2,
        "sections": [
            {"type": "carousel", "speed": "fast"},
            {"type": "cta", "heading": "Still here",
             "button": {"label": "Go", "href": "/go"}},
            {"not even": "tagged"},
        ]
    });
    let parsed = sections_lenient(&stored);
    assert_eq!(parsed.len(), 1, "only the cta parses");
    let html = render(&stored);
    assert!(html.contains("Still here"));
    assert!(!html.contains("carousel"));
}

#[test]
fn envelope_without_a_sections_array_renders_an_empty_document() {
    let html = render(&json!({"schema_version": 1}));
    assert!(html.contains("<main id=\"main\">\n</main>"));
    assert!(html.starts_with("<!doctype html>"));
    assert!(html.ends_with("</html>\n"));
}

#[test]
fn script_content_renders_escaped_never_live() {
    let stored = json!({
        "schema_version": 1,
        "sections": [{"type": "hero", "heading": "<script>alert(1)</script>"}]
    });
    let html = render(&stored);
    // The only script a published page carries is the static beacon; a
    // heading that spells one renders as text.
    assert_eq!(html.matches("<script>").count(), 1);
    assert!(html.contains("navigator.sendBeacon"));
    assert!(!html.contains("<script>alert(1)"));
    assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
}

#[test]
fn attribute_injection_renders_escaped() {
    let stored = json!({
        "schema_version": 1,
        "sections": [{"type": "gallery", "images": [
            {"blob_id": "9hK3vQ2mR8pT1xWz4bC5dg",
             "alt": "\" onmouseover=\"alert(1)"}
        ]}]
    });
    let html = render(&stored);
    assert!(!html.contains("\" onmouseover=\""));
    assert!(html.contains("&quot; onmouseover=&quot;"));
}

#[test]
fn unsafe_stored_href_renders_inert() {
    // The write gate rejects these; a hostile or pre-rule stored value must
    // still come out inert.
    let stored = json!({
        "schema_version": 1,
        "sections": [{"type": "cta", "heading": "Click",
            "button": {"label": "Go", "href": "javascript:alert(1)"}}]
    });
    let html = render(&stored);
    assert!(!html.to_ascii_lowercase().contains("javascript:"));
    assert!(html.contains("<a class=\"button\" href=\"#\">Go</a>"));
}

#[test]
fn contact_form_without_form_id_renders_text_only() {
    let stored = json!({
        "schema_version": 1,
        "sections": [{"type": "contact_form", "heading": "Write us"}]
    });
    let html = render(&stored);
    assert!(html.contains("<h2>Write us</h2>"));
    assert!(!html.contains("<form"));
    // A form with no submit is nothing for the behavior script to do; the
    // beacon is unconditional and is the only script left. It looks for
    // conversion points with the same selector — and finds none here, so a
    // page without a form reports no conversion either.
    assert!(
        !html.contains("fetch("),
        "the behavior script was appended for a page with nothing to submit"
    );
    assert_eq!(html.matches("<script>").count(), 1);
}

#[test]
fn contact_form_without_custom_message_gets_the_default_data_success() {
    let stored = json!({
        "schema_version": 1,
        "sections": [{"type": "contact_form", "form_id": "f4K9sL2wN7qR5tYx8vB1cA"}]
    });
    let html = render(&stored);
    assert!(html.contains("data-success=\"Thanks — your message has been sent.\""));
}

/// The page's entire JavaScript is two static inline blocks: the behavior
/// script, present only when there is a menu to toggle or a form to submit,
/// and the beacon, present on every published page. A page with neither a
/// menu nor a working form ships only the beacon.
#[test]
fn behavior_script_is_included_exactly_when_needed() {
    let with_nav = render(&hero_page());
    assert_eq!(with_nav.matches("<script>").count(), 2);
    let script_at = with_nav.find("<script>").unwrap();
    assert!(script_at > with_nav.find("</footer>").unwrap());
    assert!(script_at < with_nav.find("</body>").unwrap());
    for wired in [
        "classList.add(\"js\")",
        "aria-expanded",
        "form[action^=\"/f/\"]",
    ] {
        assert!(with_nav.contains(wired), "script lost its {wired} wiring");
    }

    let with_form = render(&json!({
        "schema_version": 1,
        "sections": [{"type": "contact_form", "form_id": "f4K9sL2wN7qR5tYx8vB1cA"}]
    }));
    assert_eq!(with_form.matches("<script>").count(), 2);

    let static_only = render(&json!({
        "schema_version": 1,
        "sections": [{"type": "hero", "heading": "Quiet"}]
    }));
    assert_eq!(static_only.matches("<script>").count(), 1);
    assert!(
        !static_only.contains("classList.add(\"js\")"),
        "a page with nothing to toggle or submit ships no behavior script"
    );
    assert!(static_only.contains("navigator.sendBeacon"));
}

#[test]
fn contact_form_posts_to_the_form_path_with_honeypot_and_fixed_fields() {
    let stored = json!({
        "schema_version": 1,
        "sections": [{"type": "contact_form",
            "form_id": "f4K9sL2wN7qR5tYx8vB1cA",
            "success_message": "Thanks!"}]
    });
    let html = render(&stored);
    assert!(html.contains(
        "<form action=\"/f/f4K9sL2wN7qR5tYx8vB1cA\" method=\"post\" data-success=\"Thanks!\">"
    ));
    // The fixed v1 field contract the forms backend will accept.
    for field in ["name=\"name\"", "name=\"email\"", "name=\"message\""] {
        assert!(html.contains(field), "missing {field}");
    }
    // The honeypot: present, hidden from assistive tech, out of tab order.
    assert!(html.contains(
        "<p class=\"hp\" aria-hidden=\"true\"><label for=\"form-0-website\">Website</label>"
    ));
    assert!(html.contains("name=\"website\" type=\"text\" tabindex=\"-1\" autocomplete=\"off\""));
}

#[test]
fn theme_logo_and_favicon_reach_nav_and_head() {
    let theme = SiteTheme::from_value(json!({
        "schema_version": 1,
        "preset": "terra",
        "logo": "L0g0aaaaaaaaaaaaaaaaaa",
        "favicon": "Fav1conaaaaaaaaaaaaaaa",
    }))
    .unwrap();
    let html = render_with(&theme, &hero_page(), None, None);
    assert!(html.contains("<link rel=\"icon\" href=\"/assets/img/Fav1conaaaaaaaaaaaaaaa\">"));
    assert!(html.contains(
        "<a class=\"brand\" href=\"/\"><img class=\"logo\" src=\"/assets/img/L0g0aaaaaaaaaaaaaaaaaa\" alt=\"Nordwind Coffee Roasters\"></a>"
    ));
    let without_hero_art = render_with(
        &theme,
        &json!({"schema_version": 1, "sections": []}),
        None,
        None,
    );
    assert!(without_hero_art.contains(
        "<meta property=\"og:image\" content=\"https://nordwind.alosites.com/assets/img/L0g0aaaaaaaaaaaaaaaaaa\">"
    ));
    // Without a logo, the brand link is the site name as text.
    let bare = render(&hero_page());
    assert!(bare.contains("<a class=\"brand\" href=\"/\">Nordwind Coffee Roasters</a>"));
    assert!(!bare.contains("rel=\"icon\""));
}

/// Inline image sources (the draft preview's spelling): an id in the map
/// renders as its data URI, an id missing from the map falls back to the
/// public path — and `og:image` always stays the absolute public URL, since
/// it is crawler metadata, not something the document displays.
#[test]
fn inline_image_sources_swap_srcs_per_id_and_never_touch_og_image() {
    let map = std::collections::HashMap::from([(
        "9hK3vQ2mR8pT1xWz4bC5dg".to_owned(),
        "data:image/png;base64,QUJD".to_owned(),
    )]);
    let theme = SiteTheme::from_value(json!({
        "schema_version": 1,
        "preset": "north",
        "favicon": "f4K9sL2wN7qR5tYx8vB1cA"
    }))
    .unwrap();
    let site = SiteRenderContext {
        name: "Nordwind Coffee Roasters",
        base_url: "https://nordwind.alosites.com",
        locale: "en",
        theme: &theme,
        strings: &EN,
        images: ImageSources::Inline(&map),
    };
    let collections = HashMap::new();
    let page = PageRenderContext {
        path: "/about",
        title: "About",
        seo_title: None,
        seo_description: None,
        sections: &hero_page(),
        collections: &collections,
        catalogs: &HashMap::new(),
    };
    let html = render_page(&site, &page);
    // The hero image is in the map: rendered inline.
    assert!(html.contains("<img src=\"data:image/png;base64,QUJD\" alt=\"The drum\">"));
    assert!(!html.contains("src=\"/assets/img/9hK3vQ2mR8pT1xWz4bC5dg\""));
    // The favicon is not in the map: public-path fallback.
    assert!(html.contains("<link rel=\"icon\" href=\"/assets/img/f4K9sL2wN7qR5tYx8vB1cA\">"));
    // og:image ignores the map by design.
    assert!(html.contains(
        "og:image\" content=\"https://nordwind.alosites.com/assets/img/9hK3vQ2mR8pT1xWz4bC5dg\""
    ));
}

/// The draft preview is the published document with the stylesheet inlined
/// and the analytics beacon left off — byte-for-byte, for every shipped
/// preset. This is the no-drift pin: if the two ever diverge beyond those two
/// serving concerns, editing would preview something publishing does not
/// produce.
///
/// The beacon is the second exception for the same reason as the first: both
/// address the published origin, and neither resolves behind the editor's
/// sandboxed iframe. An editor moving sections around must never be counted
/// as somebody reading the site.
#[test]
fn preview_is_the_published_document_with_the_stylesheet_inlined() {
    for preset in THEME_PRESETS {
        let theme =
            SiteTheme::from_value(json!({"schema_version": 1, "preset": preset.id})).unwrap();
        let site = SiteRenderContext {
            name: "Nordwind Coffee Roasters",
            base_url: "https://nordwind.alosites.com",
            locale: "en",
            theme: &theme,
            strings: &EN,
            images: ImageSources::PublicPaths,
        };
        let collections = HashMap::new();
        let page = PageRenderContext {
            path: "/about",
            title: "About",
            seo_title: None,
            seo_description: Some("Who we are."),
            sections: &hero_page(),
            collections: &collections,
            catalogs: &HashMap::new(),
        };
        let css = stylesheet(&theme);
        let published = render_page(&site, &page);
        let preview = render_page_preview(&site, &page, &css);
        assert!(
            published.contains("navigator.sendBeacon(\"/_alo/collect\""),
            "{}: a published page must carry the beacon",
            preset.id
        );
        let beacon_start = published
            .find("<script>(function () {\n  \"use strict\";\n  var since")
            .expect("the published document must carry exactly one beacon block");
        let beacon = &published[beacon_start..];
        assert!(
            beacon.ends_with("</script>\n</body>\n</html>\n"),
            "{}: the beacon must be the last thing before </body>",
            preset.id
        );
        let expected = published
            .replace(
                "<link rel=\"stylesheet\" href=\"/assets/site.css\">\n",
                &format!("<style>\n{css}</style>\n"),
            )
            .replace(&beacon[..beacon.len() - "</body>\n</html>\n".len()], "");
        assert_eq!(preview, expected, "{}: preview drifted", preset.id);
        assert!(!preview.contains("/assets/site.css"), "{}", preset.id);
        assert!(
            !preview.contains("/_alo/collect"),
            "{}: the draft preview must not count itself as traffic",
            preset.id
        );
    }
}

/// The unlock screen of a protected page (S2.06a): the site's own chrome, one
/// password field posting back to the page, nothing about the page itself, and
/// the visitor's own language.
#[test]
fn the_unlock_screen_asks_for_a_password_and_reveals_nothing_else() {
    let theme = SiteTheme::new();
    for (strings, locale, heading, notice) in [
        (
            &EN,
            "en",
            "This page is protected",
            "does not open this page",
        ),
        (
            &FR,
            "fr",
            "Cette page est protégée",
            "n’ouvre pas cette page",
        ),
        (
            &NL,
            "nl",
            "Deze pagina is beveiligd",
            "opent deze pagina niet",
        ),
    ] {
        let site = SiteRenderContext {
            name: "Nordwind & Co",
            base_url: "https://nordwind.alosites.com",
            locale,
            theme: &theme,
            strings,
            images: ImageSources::PublicPaths,
        };
        let asked = render_password_challenge(&site, "/prices", None);
        assert!(
            asked.contains(&format!("<html lang=\"{locale}\">")),
            "{asked}"
        );
        assert!(asked.contains(heading), "{asked}");
        assert!(
            asked.contains("<meta name=\"robots\" content=\"noindex\">"),
            "an unlock screen is not content to index: {asked}"
        );
        assert!(
            asked.contains("<form method=\"post\" action=\"/prices\">"),
            "the form posts back to the page it stands in front of: {asked}"
        );
        assert!(
            asked.contains("name=\"password\"") && asked.contains("type=\"password\""),
            "{asked}"
        );
        assert!(
            asked.contains("Nordwind &amp; Co"),
            "the site name is escaped like everywhere else: {asked}"
        );
        assert!(
            !asked.contains("<script") && !asked.contains("data-success"),
            "the screen works without scripting: {asked}"
        );
        assert!(
            !asked.contains(notice),
            "no notice on the first ask: {asked}"
        );
        assert!(
            !asked.contains("aria-invalid") && !asked.contains("aria-describedby"),
            "nothing has gone wrong on the first ask: {asked}"
        );

        let refused = render_password_challenge(&site, "/prices", strings.protected_wrong.into());
        assert!(refused.contains(notice), "{refused}");
        assert!(refused.contains("role=\"alert\""), "{refused}");
        assert!(
            refused.contains("id=\"site-password-notice\"")
                && refused.contains("aria-describedby=\"site-password-notice\"")
                && refused.contains("aria-invalid=\"true\""),
            "a visitor back on the field hears why it refused: {refused}"
        );
    }
}

/// A published section image is responsive: the ladder in `srcset`, the slot
/// width in `sizes`, and a `src` that still works for anything that honors
/// neither. Grid cards defer their load; a hero — often the largest element
/// painted — does not.
#[test]
fn published_section_images_carry_the_derivative_ladder() {
    let html = render(&json!({
        "schema_version": 1,
        "sections": [
            {"type": "hero", "heading": "Roasted here",
             "image": {"blob_id": "9hK3vQ2mR8pT1xWz4bC5dg", "alt": "The drum"}},
            {"type": "gallery", "images": [
                {"blob_id": "9hK3vQ2mR8pT1xWz4bC5dg", "alt": "The drum"}]}
        ]
    }));
    assert!(
        html.contains(
            "srcset=\"/assets/img/9hK3vQ2mR8pT1xWz4bC5dg/w480 480w, \
             /assets/img/9hK3vQ2mR8pT1xWz4bC5dg/w960 960w, \
             /assets/img/9hK3vQ2mR8pT1xWz4bC5dg/w1440 1440w\""
        ),
        "{html}"
    );
    assert!(
        html.contains("sizes=\"(min-width: 70rem) 67.5rem, 100vw\" alt=\"The drum\""),
        "the hero fills the content column and is not deferred: {html}"
    );
    assert!(
        html.contains(
            "sizes=\"(min-width: 70rem) 17rem, (min-width: 48rem) 33vw, 100vw\" \
             loading=\"lazy\" decoding=\"async\""
        ),
        "a gallery tile is a card, and cards defer: {html}"
    );
    // The unframed fallback stays the original bytes at their own path.
    assert!(html.contains("<img src=\"/assets/img/9hK3vQ2mR8pT1xWz4bC5dg\" srcset="));
}

/// A cropped image may not fall back to the unframed original: that is the
/// picture *before* the owner framed it. Its `src` is the widest derivative,
/// and every candidate carries the same rectangle.
#[test]
fn a_framed_image_falls_back_to_the_frame_never_to_the_whole_photo() {
    let html = render(&json!({
        "schema_version": 1,
        "sections": [{"type": "hero", "heading": "Roasted here", "image": {
            "blob_id": "9hK3vQ2mR8pT1xWz4bC5dg",
            "alt": "The drum",
            "crop": {"x_bp": 2500, "y_bp": 1000, "width_bp": 5000, "height_bp": 6000},
            "focal": {"x_bp": 5000, "y_bp": 4000}
        }}]
    }));
    assert!(
        html.contains("<img src=\"/assets/img/9hK3vQ2mR8pT1xWz4bC5dg/c2500-1000-5000-6000/w1440\""),
        "{html}"
    );
    assert!(html.contains("/assets/img/9hK3vQ2mR8pT1xWz4bC5dg/c2500-1000-5000-6000/w480 480w"));
    assert!(
        !html.contains("src=\"/assets/img/9hK3vQ2mR8pT1xWz4bC5dg\""),
        "the unframed original is never the fallback of a cropped image: {html}"
    );
}

/// The draft preview carries its bytes inline; there is no origin behind the
/// sandboxed iframe to fetch a derivative from, so it offers no ladder at all
/// rather than a set of paths that would all 404.
#[test]
fn the_inline_preview_offers_no_derivatives() {
    let map = std::collections::HashMap::from([(
        "9hK3vQ2mR8pT1xWz4bC5dg".to_owned(),
        "data:image/png;base64,QUJD".to_owned(),
    )]);
    let theme = SiteTheme::new();
    let site = SiteRenderContext {
        name: "Nordwind Coffee Roasters",
        base_url: "https://nordwind.alosites.com",
        locale: "en",
        theme: &theme,
        strings: &EN,
        images: ImageSources::Inline(&map),
    };
    let collections = HashMap::new();
    let sections = json!({
        "schema_version": 1,
        "sections": [{"type": "hero", "heading": "Roasted here", "image": {
            "blob_id": "9hK3vQ2mR8pT1xWz4bC5dg",
            "alt": "The drum",
            "crop": {"x_bp": 2500, "y_bp": 1000, "width_bp": 5000, "height_bp": 6000}
        }}]
    });
    let page = PageRenderContext {
        path: "/",
        title: "Home",
        seo_title: None,
        seo_description: None,
        sections: &sections,
        collections: &collections,
        catalogs: &HashMap::new(),
    };
    let html = render_page(&site, &page);
    assert!(html.contains("<img src=\"data:image/png;base64,QUJD\" alt=\"The drum\">"));
    assert!(
        !html.contains("srcset") && !html.contains("sizes="),
        "{html}"
    );
}

/// Defense in depth on the new attribute: a stored blob id that the write
/// gate would never have admitted still cannot close the `srcset` quote and
/// start an event handler. Every candidate path is escaped exactly like the
/// `src` beside it.
#[test]
fn a_hostile_blob_id_cannot_escape_the_srcset_attribute() {
    let html = render(&json!({
        "schema_version": 1,
        "sections": [{"type": "gallery", "images": [
            {"blob_id": "x\" onerror=\"alert(1)", "alt": "Broken"}]}]
    }));
    assert!(
        !html.contains("onerror=\"alert(1)\""),
        "the handler must never be an attribute: {html}"
    );
    assert!(
        html.contains("x&quot; onerror=&quot;alert(1)/w480 480w"),
        "the id renders as inert text inside the attribute: {html}"
    );
}
