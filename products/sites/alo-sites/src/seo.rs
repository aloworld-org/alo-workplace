//! Pure renderers for the two search-engine discovery documents served by
//! every live alo Site. Keeping byte generation out of the HTTP layer makes
//! the public contract small, deterministic and golden-testable.

/// The sitemap protocol permits at most 50,000 URLs in one document. The
/// caller reserves the available slots for pages before adding blog routes.
pub const SITEMAP_URL_LIMIT: usize = 50_000;

/// Renders one Sitemap XML document from canonical absolute URLs.
#[must_use]
pub fn render_sitemap<'a>(urls: impl IntoIterator<Item = &'a str>) -> String {
    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n",
    );
    for url in urls.into_iter().take(SITEMAP_URL_LIMIT) {
        xml.push_str("  <url><loc>");
        push_xml_escaped(&mut xml, url);
        xml.push_str("</loc></url>\n");
    }
    xml.push_str("</urlset>\n");
    xml
}

/// Renders the permissive crawler policy and points crawlers at this exact
/// site's sitemap. `base_url` is constructed from the validated Host scope.
#[must_use]
pub fn render_robots(base_url: &str) -> String {
    format!("User-agent: *\nAllow: /\nSitemap: {base_url}/sitemap.xml\n")
}

fn push_xml_escaped(output: &mut String, value: &str) {
    for character in value.chars() {
        match character {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '\"' => output.push_str("&quot;"),
            '\'' => output.push_str("&apos;"),
            _ => output.push(character),
        }
    }
}
