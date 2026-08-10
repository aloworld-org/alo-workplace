//! Search discovery documents are pure renderer output. These goldens pin
//! exact interoperable bytes separately from routing and storage.

use alo_sites::seo::{SitemapAlternate, SitemapUrl, render_robots, render_sitemap};

#[test]
fn sitemap_matches_the_discovery_contract() {
    assert_eq!(
        render_sitemap(&[
            SitemapUrl {
                location: "https://north.sites.test/".to_owned(),
                alternates: vec![
                    SitemapAlternate {
                        locale: "en".to_owned(),
                        location: "https://north.sites.test/".to_owned(),
                        is_default: true,
                    },
                    SitemapAlternate {
                        locale: "fr".to_owned(),
                        location: "https://north.sites.test/fr".to_owned(),
                        is_default: false,
                    },
                ],
            },
            SitemapUrl::plain("https://north.sites.test/about".to_owned()),
            SitemapUrl::plain("https://north.sites.test/blog".to_owned()),
            SitemapUrl::plain("https://north.sites.test/blog/research-&-development".to_owned(),),
        ]),
        include_str!("golden/sitemap.xml")
    );
}

#[test]
fn robots_matches_the_discovery_contract() {
    assert_eq!(
        render_robots("https://north.sites.test"),
        include_str!("golden/robots.txt")
    );
}
