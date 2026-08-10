//! Pure renderers for the two search-engine discovery documents served by
//! every live alo Site. Keeping byte generation out of the HTTP layer makes
//! the public contract small, deterministic and golden-testable.

/// The sitemap protocol permits at most 50,000 URLs in one document. The
/// caller reserves the available slots for pages before adding blog routes.
pub const SITEMAP_URL_LIMIT: usize = 50_000;

/// One translated form of a canonical sitemap URL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SitemapAlternate {
    pub locale: String,
    pub location: String,
    /// Whether this translation is the site's default, and therefore the
    /// language-neutral `x-default` destination.
    pub is_default: bool,
}

/// One canonical sitemap URL and all exact page translations frozen beside it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SitemapUrl {
    pub location: String,
    pub alternates: Vec<SitemapAlternate>,
}

impl SitemapUrl {
    #[must_use]
    pub fn plain(location: String) -> Self {
        Self {
            location,
            alternates: Vec::new(),
        }
    }
}

/// Renders one Sitemap XML document with per-page language discovery links.
#[must_use]
pub fn render_sitemap(urls: &[SitemapUrl]) -> String {
    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\" \
         xmlns:xhtml=\"http://www.w3.org/1999/xhtml\">\n",
    );
    for url in urls.iter().take(SITEMAP_URL_LIMIT) {
        xml.push_str("  <url><loc>");
        push_xml_escaped(&mut xml, &url.location);
        xml.push_str("</loc>");
        for alternate in &url.alternates {
            xml.push_str("<xhtml:link rel=\"alternate\" hreflang=\"");
            push_xml_escaped(&mut xml, &alternate.locale);
            xml.push_str("\" href=\"");
            push_xml_escaped(&mut xml, &alternate.location);
            xml.push_str("\"/>");
            if alternate.is_default {
                xml.push_str("<xhtml:link rel=\"alternate\" hreflang=\"x-default\" href=\"");
                push_xml_escaped(&mut xml, &alternate.location);
                xml.push_str("\"/>");
            }
        }
        xml.push_str("</url>\n");
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
