//! Golden-HTML pinning of the renderer: one golden per section type plus a
//! full-page golden of a themed site carrying all twelve sections. Run with
//! `UPDATE_GOLDENS=1` to re-bless after a deliberate markup change, then
//! review the diff like any code change.
#![allow(clippy::unwrap_used)]

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use alo_sites::render::{EN, ImageSources, PageRenderContext, SiteRenderContext, render_page};
use alo_store::id::{BlobId, SiteCatalogId, SiteCollectionId};
use alo_store::site_model::{
    CatalogSection, CollectionSection, ContactFormSection, CtaSection, FaqItem, FaqSection,
    FeatureItem, FeaturesSection, FooterSection, GallerySection, HeroSection, ImageCrop,
    ImageFocalPoint, ImageSide, Link, NavSection, PricingSection, PricingTier,
    SECTIONS_SCHEMA_VERSION, Section, SectionsEnvelope, SiteImage, TeamMember, TeamSection,
    Testimonial, TestimonialsSection, TextImageSection,
};
use alo_store::site_theme::SiteTheme;
use alo_store::{
    SiteCatalogSnapshot, SiteCatalogSnapshotCategory, SiteCatalogSnapshotItem, SiteCollectionItem,
    SiteCollectionSnapshot,
};
use serde_json::json;

const SITE_NAME: &str = "Nordwind Coffee Roasters";
const BASE_URL: &str = "https://nordwind.alosites.com";

/// One fully-populated instance of every section variant, mirroring the
/// store's schema-test corpus — deterministic content, deterministic ids.
fn full_sections() -> Vec<Section> {
    let image = SiteImage::new(
        BlobId::new("9hK3vQ2mR8pT1xWz4bC5dg"),
        "Roasting drum mid-batch",
    );
    let link = |label: &str, href: &str| Link {
        label: label.to_owned(),
        href: href.to_owned(),
    };
    vec![
        Section::Nav(NavSection {
            links: vec![link("Home", "/"), link("Pricing", "/pricing")],
            cta: Some(link("Order beans", "/order")),
        }),
        Section::Hero(HeroSection {
            heading: "Coffee roasted the morning it ships".to_owned(),
            subheading: Some("Small-batch roastery on the harbour".to_owned()),
            image: Some(image.clone()),
            primary_cta: Some(link("Shop roasts", "/shop")),
            secondary_cta: Some(link("Our story", "/about")),
        }),
        Section::Features(FeaturesSection {
            heading: Some("Why Nordwind".to_owned()),
            intro: Some("Three promises on every bag.".to_owned()),
            items: vec![FeatureItem {
                title: "Roasted to order".to_owned(),
                body: "Your batch goes in the drum after you order.".to_owned(),
                icon: Some("flame".to_owned()),
            }],
        }),
        Section::TextImage(TextImageSection {
            heading: Some("The roastery".to_owned()),
            body: "A 1962 Probat drum, rebuilt by hand.".to_owned(),
            image: image.clone(),
            image_side: ImageSide::Left,
        }),
        Section::Gallery(GallerySection {
            heading: Some("Inside the roastery".to_owned()),
            // The second tile is framed (S2.07a): the golden pins what a crop
            // spells in a `srcset`, and that its `src` fallback is the framed
            // derivative rather than the unframed original.
            images: vec![image.clone(), cropped_image()],
        }),
        Section::Testimonials(TestimonialsSection {
            heading: Some("What cafés say".to_owned()),
            items: vec![Testimonial {
                quote: "The freshest beans we've ever pulled shots with.".to_owned(),
                author: "Mara Lindqvist".to_owned(),
                role: Some("Head barista, Kaffebaren".to_owned()),
            }],
        }),
        Section::Pricing(PricingSection {
            heading: Some("Subscriptions".to_owned()),
            intro: Some("Pause or cancel any time.".to_owned()),
            tiers: vec![PricingTier {
                name: "Weekly".to_owned(),
                price: "€18/week".to_owned(),
                period: Some("billed weekly".to_owned()),
                description: Some("Two 250g bags every week.".to_owned()),
                features: vec!["Free shipping".to_owned(), "Roast-day dispatch".to_owned()],
                cta: Some(link("Start weekly", "/subscribe/weekly")),
                highlighted: true,
            }],
        }),
        Section::Team(TeamSection {
            heading: Some("The roasters".to_owned()),
            members: vec![TeamMember {
                name: "Jonas Meer".to_owned(),
                role: Some("Founder & head roaster".to_owned()),
                photo: Some(image.clone()),
                bio: Some("Twenty years at the drum.".to_owned()),
            }],
        }),
        Section::Faq(FaqSection {
            heading: Some("Questions".to_owned()),
            items: vec![FaqItem {
                question: "How fresh is the coffee?".to_owned(),
                answer: "It ships the day it is roasted.".to_owned(),
            }],
        }),
        Section::Cta(CtaSection {
            heading: "Taste the difference".to_owned(),
            body: Some("First bag ships free.".to_owned()),
            button: link("Order now", "/order"),
        }),
        Section::ContactForm(ContactFormSection {
            heading: Some("Wholesale enquiries".to_owned()),
            body: Some("We answer within one business day.".to_owned()),
            form_id: Some("f4K9sL2wN7qR5tYx8vB1cA".to_owned()),
            success_message: Some("Thanks — talk soon.".to_owned()),
        }),
        Section::Collection(CollectionSection {
            collection_id: SiteCollectionId::new("seasonal-roasts"),
            heading: Some("Seasonal roasts".to_owned()),
        }),
        Section::Catalog(CatalogSection {
            catalog_id: SiteCatalogId::new("harbour-menu"),
            heading: Some("On the counter".to_owned()),
            category: None,
        }),
        Section::Footer(FooterSection {
            text: Some("© Nordwind Coffee Roasters".to_owned()),
            links: vec![link("Imprint", "/imprint"), link("Privacy", "/privacy")],
        }),
    ]
}

/// A framed photo: the right two-thirds of the source, focal point on the
/// left of that rectangle.
fn cropped_image() -> SiteImage {
    SiteImage {
        crop: Some(ImageCrop {
            x_bp: 3333,
            y_bp: 0,
            width_bp: 6667,
            height_bp: 10000,
        }),
        focal: Some(ImageFocalPoint {
            x_bp: 4000,
            y_bp: 5000,
        }),
        ..SiteImage::new(BlobId::new("Cr0pp3dPh0t0aaaaaaaaaa"), "The cupping table")
    }
}

fn collection_snapshots() -> HashMap<String, SiteCollectionSnapshot> {
    let snapshot = SiteCollectionSnapshot {
        collection_id: SiteCollectionId::new("seasonal-roasts"),
        name: "Seasonal roasts".to_owned(),
        items: vec![SiteCollectionItem {
            title: "Harbour Blend".to_owned(),
            slug: Some("harbour-blend".to_owned()),
            summary: Some("Chocolate, hazelnut and red apple.".to_owned()),
            body: Some("A balanced roast for espresso or filter.".to_owned()),
            image: Some(BlobId::new("9hK3vQ2mR8pT1xWz4bC5dg")),
            link: Some("/shop/harbour-blend".to_owned()),
            published_at: Some("2026-08-11".to_owned()),
        }],
    };
    HashMap::from([(snapshot.collection_id.as_str().to_owned(), snapshot)])
}

/// The one frozen catalog the corpus renders: two groupings, a sold-out item,
/// an item with no price, one that belongs to no category, a photograph the
/// owner described and one nobody described yet — every branch the section
/// has, pinned in one golden.
fn catalog_snapshots() -> HashMap<String, SiteCatalogSnapshot> {
    let snapshot = SiteCatalogSnapshot {
        catalog_id: SiteCatalogId::new("harbour-menu"),
        name: "Harbour menu".to_owned(),
        currency: "EUR".to_owned(),
        orders_enabled: false,
        categories: vec![
            SiteCatalogSnapshotCategory {
                slug: "brews".to_owned(),
                name: "Brews".to_owned(),
            },
            SiteCatalogSnapshotCategory {
                slug: "beans".to_owned(),
                name: "Beans".to_owned(),
            },
        ],
        items: vec![
            SiteCatalogSnapshotItem {
                slug: "filter".to_owned(),
                name: "Filter brew".to_owned(),
                category: Some("brews".to_owned()),
                description: Some("Whatever came off the drum this morning.".to_owned()),
                price_cents: Some(350),
                price_note: Some("per cup".to_owned()),
                image: None,
                image_alt: None,
                sold_out: false,
            },
            SiteCatalogSnapshotItem {
                slug: "cold-brew".to_owned(),
                name: "Cold brew".to_owned(),
                category: Some("brews".to_owned()),
                description: None,
                price_cents: Some(4_250),
                price_note: None,
                image: None,
                image_alt: None,
                sold_out: true,
            },
            SiteCatalogSnapshotItem {
                slug: "harbour-blend".to_owned(),
                name: "Harbour blend, 1 kg".to_owned(),
                category: Some("beans".to_owned()),
                description: None,
                price_cents: Some(123_450),
                price_note: None,
                image: Some(BlobId::new("9hK3vQ2mR8pT1xWz4bC5dg")),
                // Described by the owner: the `alt` is what the photograph
                // shows, not the name printed under it.
                image_alt: Some("A kraft bag of whole beans on the counter".to_owned()),
                sold_out: false,
            },
            SiteCatalogSnapshotItem {
                slug: "subscription".to_owned(),
                name: "Standing order".to_owned(),
                category: None,
                description: Some("Tell us how much you drink; we work it out.".to_owned()),
                price_cents: None,
                price_note: None,
                // A photograph nobody described yet: the card falls back to the
                // item name rather than publishing an empty `alt`.
                image: Some(BlobId::new("Undescr1b3dPh0t0aaaaaa")),
                image_alt: None,
                sold_out: false,
            },
        ],
    };
    HashMap::from([(snapshot.catalog_id.as_str().to_owned(), snapshot)])
}

fn envelope_value(sections: Vec<Section>) -> serde_json::Value {
    SectionsEnvelope {
        schema_version: SECTIONS_SCHEMA_VERSION,
        sections,
    }
    .to_value()
    .unwrap()
}

/// Renders `sections` as the home page of an untheme'd (default-preset) site.
fn render_default(sections: Vec<Section>) -> String {
    let theme = SiteTheme::new();
    let value = envelope_value(sections);
    let site = SiteRenderContext {
        name: SITE_NAME,
        base_url: BASE_URL,
        locale: "en",
        theme: &theme,
        strings: &EN,
        images: ImageSources::PublicPaths,
    };
    let page = PageRenderContext {
        path: "/",
        title: "Home",
        seo_title: None,
        seo_description: None,
        sections: &value,
        collections: &collection_snapshots(),
        catalogs: &catalog_snapshots(),
    };
    render_page(&site, &page)
}

fn assert_golden(name: &str, actual: &str) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(name);
    if std::env::var_os("UPDATE_GOLDENS").is_some() {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, actual).unwrap();
    }
    let expected = fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("missing golden {name}; run once with UPDATE_GOLDENS=1"));
    assert_eq!(
        expected, actual,
        "golden {name} drifted — if deliberate, re-bless with UPDATE_GOLDENS=1 and review the diff"
    );
}

#[test]
fn one_golden_per_section_type() {
    let sections = full_sections();
    assert_eq!(sections.len(), 14, "corpus must cover every variant");
    for section in sections {
        let kind = section.kind();
        let html = render_default(vec![section]);
        assert_golden(&format!("section_{kind}.html"), &html);
    }
}

#[test]
fn empty_collection_has_a_stable_public_golden() {
    let theme = SiteTheme::new();
    let value = envelope_value(vec![Section::Collection(CollectionSection {
        collection_id: SiteCollectionId::new("seasonal-roasts"),
        heading: Some("Seasonal roasts".to_owned()),
    })]);
    let site = SiteRenderContext {
        name: SITE_NAME,
        base_url: BASE_URL,
        locale: "en",
        theme: &theme,
        strings: &EN,
        images: ImageSources::PublicPaths,
    };
    let snapshot = SiteCollectionSnapshot {
        collection_id: SiteCollectionId::new("seasonal-roasts"),
        name: "Seasonal roasts".to_owned(),
        items: Vec::new(),
    };
    let collections = HashMap::from([(snapshot.collection_id.as_str().to_owned(), snapshot)]);
    let page = PageRenderContext {
        path: "/",
        title: "Home",
        seo_title: None,
        seo_description: None,
        sections: &value,
        collections: &collections,
        catalogs: &HashMap::new(),
    };
    assert_golden("section_collection_empty.html", &render_page(&site, &page));
}

/// A published catalog section whose snapshot holds nothing renders one calm
/// sentence — the same one whether the catalog is empty, filtered to nothing,
/// or missing from the publish entirely, so a visitor learns nothing about the
/// tenant's editing state.
#[test]
fn empty_catalog_has_a_stable_public_golden() {
    let theme = SiteTheme::new();
    let value = envelope_value(vec![Section::Catalog(CatalogSection {
        catalog_id: SiteCatalogId::new("harbour-menu"),
        heading: Some("On the counter".to_owned()),
        category: None,
    })]);
    let site = SiteRenderContext {
        name: SITE_NAME,
        base_url: BASE_URL,
        locale: "en",
        theme: &theme,
        strings: &EN,
        images: ImageSources::PublicPaths,
    };
    let snapshot = SiteCatalogSnapshot {
        catalog_id: SiteCatalogId::new("harbour-menu"),
        name: "Harbour menu".to_owned(),
        currency: "EUR".to_owned(),
        orders_enabled: false,
        categories: Vec::new(),
        items: Vec::new(),
    };
    let catalogs = HashMap::from([(snapshot.catalog_id.as_str().to_owned(), snapshot)]);
    let page = PageRenderContext {
        path: "/",
        title: "Home",
        seo_title: None,
        seo_description: None,
        sections: &value,
        collections: &HashMap::new(),
        catalogs: &catalogs,
    };
    assert_golden("section_catalog_empty.html", &render_page(&site, &page));
}

/// A catalog published with ordering switched on renders the whole section as
/// one scriptless `POST` form: a quantity field on every available item, the
/// contact fields once, the honeypot, and the sentence saying nothing is paid
/// here. The sold-out item carries no quantity field — the public door would
/// refuse it, so the page must not offer it.
#[test]
fn an_orderable_catalog_renders_a_scriptless_order_form() {
    let theme = SiteTheme::new();
    let value = envelope_value(vec![Section::Catalog(CatalogSection {
        catalog_id: SiteCatalogId::new("harbour-menu"),
        heading: Some("Order for Saturday".to_owned()),
        category: None,
    })]);
    let site = SiteRenderContext {
        name: SITE_NAME,
        base_url: BASE_URL,
        locale: "en",
        theme: &theme,
        strings: &EN,
        images: ImageSources::PublicPaths,
    };
    let mut catalogs = catalog_snapshots();
    for snapshot in catalogs.values_mut() {
        snapshot.orders_enabled = true;
    }
    let page = PageRenderContext {
        path: "/",
        title: "Home",
        seo_title: None,
        seo_description: None,
        sections: &value,
        collections: &HashMap::new(),
        catalogs: &catalogs,
    };
    let html = render_page(&site, &page);
    assert!(
        html.contains("<form class=\"catalog-order\" action=\"/o/harbour-menu\" method=\"post\">"),
        "the section posts to its own catalog: {html}"
    );
    assert!(
        html.contains("name=\"qty-filter\""),
        "an available item is orderable: {html}"
    );
    assert!(
        !html.contains("name=\"qty-cold-brew\""),
        "a sold-out item offers no quantity field: {html}"
    );
    assert!(
        html.contains("Nothing is paid here"),
        "the page says what an order is: {html}"
    );
    assert_golden("section_catalog_orders.html", &html);
}

/// One category of a catalog, shown on its own — the second page of a menu
/// site, where starters and mains are two sections over one catalog.
#[test]
fn a_catalog_section_can_show_one_category() {
    let theme = SiteTheme::new();
    let value = envelope_value(vec![Section::Catalog(CatalogSection {
        catalog_id: SiteCatalogId::new("harbour-menu"),
        heading: Some("Beans to take home".to_owned()),
        category: Some("beans".to_owned()),
    })]);
    let site = SiteRenderContext {
        name: SITE_NAME,
        base_url: BASE_URL,
        locale: "en",
        theme: &theme,
        strings: &EN,
        images: ImageSources::PublicPaths,
    };
    let catalogs = catalog_snapshots();
    let page = PageRenderContext {
        path: "/",
        title: "Home",
        seo_title: None,
        seo_description: None,
        sections: &value,
        collections: &HashMap::new(),
        catalogs: &catalogs,
    };
    let html = render_page(&site, &page);
    assert!(html.contains("Harbour blend, 1 kg"), "{html}");
    assert!(
        !html.contains("Filter brew") && !html.contains("Standing order"),
        "a filtered section showed another category: {html}"
    );
    assert_golden("section_catalog_one_category.html", &html);
}

#[test]
fn full_page_golden_with_theme_logo_and_seo() {
    let theme = SiteTheme::from_value(json!({
        "schema_version": 1,
        "preset": "terra",
        "logo": "L0g0aaaaaaaaaaaaaaaaaa",
        "favicon": "Fav1conaaaaaaaaaaaaaaa",
    }))
    .unwrap();
    let value = envelope_value(full_sections());
    let site = SiteRenderContext {
        name: SITE_NAME,
        base_url: BASE_URL,
        locale: "en",
        theme: &theme,
        strings: &EN,
        images: ImageSources::PublicPaths,
    };
    let page = PageRenderContext {
        path: "/",
        title: "Home",
        seo_title: Some("Coffee roasted the morning it ships — Nordwind"),
        seo_description: Some("Small-batch coffee roastery on the harbour, shipping on roast day."),
        sections: &value,
        collections: &collection_snapshots(),
        catalogs: &catalog_snapshots(),
    };
    let html = render_page(&site, &page);
    // The design note's byte budget for the golden site's page.
    assert!(
        html.len() < 100 * 1024,
        "full page is {} bytes, budget is 100KB",
        html.len()
    );
    assert_golden("full_page.html", &html);
}

#[test]
fn search_defaults_golden_uses_page_site_and_theme_logo() {
    let theme = SiteTheme::from_value(json!({
        "schema_version": 1,
        "preset": "terra",
        "logo": "L0g0aaaaaaaaaaaaaaaaaa",
    }))
    .unwrap();
    let value = envelope_value(Vec::new());
    let site = SiteRenderContext {
        name: SITE_NAME,
        base_url: BASE_URL,
        locale: "en",
        theme: &theme,
        strings: &EN,
        images: ImageSources::PublicPaths,
    };
    let page = PageRenderContext {
        path: "/about",
        title: "About",
        seo_title: None,
        seo_description: None,
        sections: &value,
        collections: &collection_snapshots(),
        catalogs: &catalog_snapshots(),
    };
    assert_golden("seo_defaults.html", &render_page(&site, &page));
}

#[test]
fn every_img_on_the_full_corpus_carries_alt() {
    let html = render_default(full_sections());
    let mut imgs = 0;
    for tag in html.split("<img").skip(1) {
        imgs += 1;
        let tag = tag.split('>').next().unwrap();
        assert!(
            tag.contains(" alt=\""),
            "an <img> is missing its alt attribute: <img{tag}>"
        );
    }
    assert!(
        imgs >= 4,
        "corpus should exercise several images, saw {imgs}"
    );
}
