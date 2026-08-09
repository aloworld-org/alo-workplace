//! Search discovery documents are pure renderer output. These goldens pin
//! exact interoperable bytes separately from routing and storage.

use alo_sites::seo::{render_robots, render_sitemap};

#[test]
fn sitemap_matches_the_discovery_contract() {
    assert_eq!(
        render_sitemap([
            "https://north.sites.test/",
            "https://north.sites.test/about",
            "https://north.sites.test/blog",
            "https://north.sites.test/blog/research-&-development",
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
