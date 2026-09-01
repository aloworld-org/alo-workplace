//! Golden-HTML pinning of the renderer: one golden per section type plus a
//! full-page golden of a themed site carrying all fifteen sections. Run with
//! `UPDATE_GOLDENS=1` to re-bless after a deliberate markup change, then
//! review the diff like any code change.
#![allow(clippy::unwrap_used)]

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use alo_sites::render::{EN, ImageSources, PageRenderContext, SiteRenderContext, render_page};
use alo_store::id::{BlobId, CalendarId, SiteBookingId, SiteCatalogId, SiteCollectionId};
use alo_store::site_custom_code::{CustomCodeCapabilities, CustomCodeSection};
use alo_store::site_model::{
    BookingSection, CatalogSection, CollectionSection, ContactFormSection, CtaSection, FaqItem,
    FaqSection, FeatureItem, FeaturesSection, FooterSection, GallerySection, HeroSection,
    ImageCrop, ImageFocalPoint, ImageSide, Link, NavSection, PricingSection, PricingTier,
    SECTIONS_SCHEMA_VERSION, Section, SectionAlignment, SectionEntrance, SectionLayoutStyle,
    SectionPresentation, SectionSpacing, SectionWidth, SectionsEnvelope, ShopSection, SiteImage,
    TeamMember, TeamSection, Testimonial, TestimonialsSection, TextImageSection, ThemeColorRole,
    TicketsSection, TransitionDirection, TransitionEffect, TransitionSection, TransitionSpeed,
    TransitionTrigger,
};
use alo_store::site_theme::SiteTheme;
use alo_store::{
    SiteBookingSnapshot, SiteBookingWindow, SiteCatalogSnapshot, SiteCatalogSnapshotCategory,
    SiteCatalogSnapshotItem, SiteCollectionItem, SiteCollectionSnapshot,
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
            appearance: None,
        }),
        Section::Hero(HeroSection {
            heading: "Coffee roasted the morning it ships".to_owned(),
            subheading: Some("Small-batch roastery on the harbour".to_owned()),
            image: Some(image.clone()),
            video_url: None,
            primary_cta: Some(link("Shop roasts", "/shop")),
            secondary_cta: Some(link("Our story", "/about")),
            appearance: None,
            layout: None,
            height: None,
            alignment: None,
            content_width: None,
            text_animation: None,
            media_animation: None,
            animation_speed: None,
        }),
        Section::Features(FeaturesSection {
            heading: Some("Why Nordwind".to_owned()),
            intro: Some("Three promises on every bag.".to_owned()),
            items: vec![FeatureItem {
                title: "Roasted to order".to_owned(),
                body: "Your batch goes in the drum after you order.".to_owned(),
                icon: Some("flame".to_owned()),
            }],
            columns: None,
            layout: Some(alo_store::site_model::FeaturesLayout::Bento),
            presentation: Some(SectionPresentation {
                layout: SectionLayoutStyle::Cards,
                spacing: SectionSpacing::Generous,
                width: SectionWidth::Wide,
                alignment: SectionAlignment::Center,
                background: ThemeColorRole::Accent3,
                text: ThemeColorRole::Text,
                button: ThemeColorRole::Accent1,
                button_text: None,
                button_hover: ThemeColorRole::Accent2,
                button_hover_text: Some(ThemeColorRole::Background),
                entrance: SectionEntrance::FadeUp,
                speed: TransitionSpeed::Relaxed,
            }),
        }),
        Section::TextImage(TextImageSection {
            heading: Some("The roastery".to_owned()),
            body: "A 1962 Probat drum, rebuilt by hand.".to_owned(),
            image: image.clone(),
            image_side: ImageSide::Left,
            split: None,
            layout: Some(alo_store::site_model::TextImageLayout::Overlap),
            presentation: None,
        }),
        Section::Gallery(GallerySection {
            heading: Some("Inside the roastery".to_owned()),
            // The second tile is framed (S2.07a): the golden pins what a crop
            // spells in a `srcset`, and that its `src` fallback is the framed
            // derivative rather than the unframed original.
            images: vec![image.clone(), cropped_image()],
            columns: None,
            layout: Some(alo_store::site_model::GalleryLayout::Collage),
            presentation: None,
        }),
        Section::Testimonials(TestimonialsSection {
            heading: Some("What cafés say".to_owned()),
            items: vec![Testimonial {
                quote: "The freshest beans we've ever pulled shots with.".to_owned(),
                author: "Mara Lindqvist".to_owned(),
                role: Some("Head barista, Kaffebaren".to_owned()),
            }],
            layout: Some(alo_store::site_model::TestimonialsLayout::Featured),
            presentation: None,
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
            layout: Some(alo_store::site_model::PricingLayout::Featured),
            presentation: None,
        }),
        Section::Team(TeamSection {
            heading: Some("The roasters".to_owned()),
            members: vec![TeamMember {
                name: "Jonas Meer".to_owned(),
                role: Some("Founder & head roaster".to_owned()),
                photo: Some(image.clone()),
                bio: Some("Twenty years at the drum.".to_owned()),
            }],
            columns: None,
            presentation: None,
        }),
        Section::Faq(FaqSection {
            heading: Some("Questions".to_owned()),
            items: vec![FaqItem {
                question: "How fresh is the coffee?".to_owned(),
                answer: "It ships the day it is roasted.".to_owned(),
            }],
            presentation: None,
        }),
        Section::Cta(CtaSection {
            heading: "Taste the difference".to_owned(),
            body: Some("First bag ships free.".to_owned()),
            button: link("Order now", "/order"),
            presentation: None,
        }),
        Section::ContactForm(ContactFormSection {
            heading: Some("Wholesale enquiries".to_owned()),
            body: Some("We answer within one business day.".to_owned()),
            form_id: Some("f4K9sL2wN7qR5tYx8vB1cA".to_owned()),
            success_message: Some("Thanks — talk soon.".to_owned()),
            presentation: None,
        }),
        Section::Collection(CollectionSection {
            collection_id: SiteCollectionId::new("seasonal-roasts"),
            heading: Some("Seasonal roasts".to_owned()),
            presentation: None,
        }),
        Section::Catalog(CatalogSection {
            catalog_id: SiteCatalogId::new("harbour-menu"),
            heading: Some("On the counter".to_owned()),
            category: None,
            presentation: None,
        }),
        Section::Tickets(TicketsSection {
            heading: Some("Cupping evenings".to_owned()),
            body: Some("Six seats around the roaster, once a month.".to_owned()),
            presentation: None,
        }),
        Section::Shop(ShopSection {
            heading: Some("The roastery shop".to_owned()),
            body: Some("Beans and brew gear, shipped from the roastery.".to_owned()),
            presentation: None,
        }),
        Section::Transition(TransitionSection {
            effect: TransitionEffect::Slide,
            direction: TransitionDirection::Up,
            speed: TransitionSpeed::Smooth,
            trigger: TransitionTrigger::Balanced,
            animate_out: true,
        }),
        Section::CustomCode(custom_code_block()),
        Section::Footer(FooterSection {
            text: Some("© Nordwind Coffee Roasters".to_owned()),
            links: vec![link("Imprint", "/imprint"), link("Privacy", "/privacy")],
        }),
    ]
}

/// The corpus's custom-code block: markup, style, and a script, with the one
/// capability that runs the script.
fn custom_code_block() -> CustomCodeSection {
    CustomCodeSection {
        heading: Some("Roast timer".to_owned()),
        title: "A timer counting down the current roast".to_owned(),
        html: "<p id=\"left\">12:00</p><button type=\"button\" id=\"go\">Start</button>".to_owned(),
        css: Some("#left { font-size: 3rem; }".to_owned()),
        js: Some("document.getElementById('go').addEventListener('click', () => {});".to_owned()),
        capabilities: CustomCodeCapabilities {
            scripts: true,
            inline_images: false,
        },
        height_px: 220,
    }
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
        bookings: &HashMap::new(),
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
    assert_eq!(
        sections.len(),
        18,
        "corpus must cover every rendered body variant"
    );
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
        presentation: None,
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
        bookings: &HashMap::new(),
    };
    assert_golden("section_collection_empty.html", &render_page(&site, &page));
}

/// A booking section renders what is offered and the day field that leads to
/// the free times — never the times themselves, which are live state and would
/// be wrong the moment the page was cached.
#[test]
fn booking_section_has_a_stable_public_golden() {
    let theme = SiteTheme::new();
    let value = envelope_value(vec![Section::Booking(BookingSection {
        booking_id: SiteBookingId::new("studio-consultation"),
        heading: Some("Come and talk to us".to_owned()),
        presentation: None,
    })]);
    let site = SiteRenderContext {
        name: SITE_NAME,
        base_url: BASE_URL,
        locale: "en",
        theme: &theme,
        strings: &EN,
        images: ImageSources::PublicPaths,
    };
    let snapshot = SiteBookingSnapshot {
        booking_id: SiteBookingId::new("studio-consultation"),
        name: "Consultation".to_owned(),
        description: Some("Half an hour, in the studio.".to_owned()),
        calendar: CalendarId::new("cal-owner"),
        time_zone: "Europe/Brussels".to_owned(),
        duration_minutes: 30,
        buffer_minutes: 0,
        notice_minutes: 120,
        horizon_days: 60,
        location: Some("Second floor, ring the bell".to_owned()),
        hours: vec![SiteBookingWindow {
            weekday: 3,
            start_minute: 540,
            end_minute: 660,
        }],
        fields: Vec::new(),
        active: true,
    };
    let bookings = HashMap::from([(snapshot.booking_id.as_str().to_owned(), snapshot.clone())]);
    let page = PageRenderContext {
        path: "/",
        title: "Home",
        seo_title: None,
        seo_description: None,
        sections: &value,
        collections: &HashMap::new(),
        catalogs: &HashMap::new(),
        bookings: &bookings,
    };
    assert_golden("section_booking.html", &render_page(&site, &page));

    // Switched off before the publish: the offer stands, the form does not.
    let closed = SiteBookingSnapshot {
        active: false,
        ..snapshot
    };
    let bookings = HashMap::from([(closed.booking_id.as_str().to_owned(), closed)]);
    let page = PageRenderContext {
        bookings: &bookings,
        ..page
    };
    assert_golden("section_booking_closed.html", &render_page(&site, &page));
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
        presentation: None,
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
        bookings: &HashMap::new(),
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
        presentation: None,
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
        bookings: &HashMap::new(),
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
        presentation: None,
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
        bookings: &HashMap::new(),
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
        bookings: &HashMap::new(),
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
        bookings: &HashMap::new(),
    };
    assert_golden("seo_defaults.html", &render_page(&site, &page));
}

/// The custom-code block's frame, checked as a contract rather than as a
/// picture: the isolation is a handful of attribute values, and every one of
/// them is asserted here. A change that quietly hands the frame its origin
/// back, or drops the policy, fails this test long before it reaches a site.
#[test]
fn a_custom_code_block_is_served_inside_a_locked_frame() {
    let html = render_default(vec![Section::CustomCode(custom_code_block())]);

    // The block is a frame with an accessible name, sized by the author, and
    // sandboxed with the one token its declared capability earns.
    assert!(html.contains("<iframe class=\"custom-frame\""), "{html}");
    assert!(
        html.contains("title=\"A timer counting down the current roast\""),
        "the frame must carry its accessible name: {html}"
    );
    assert!(html.contains("sandbox=\"allow-scripts\""), "{html}");
    assert!(html.contains("style=\"height:220px\""), "{html}");
    for escape in [
        "allow-same-origin",
        "allow-top-navigation",
        "allow-popups",
        "allow-modals",
        "allow-downloads",
    ] {
        assert!(
            !html.contains(escape),
            "the frame was handed {escape}: {html}"
        );
    }

    // The document inside is carried in `srcdoc`, so the page makes no second
    // request — and it declares the closed policy before anything else.
    //
    // The quotes are escaped twice on purpose, because there are two parsers:
    // the page's parser turns `&amp;#39;` back into `&#39;` when it reads the
    // attribute, and the frame's own parser turns that into `'` when it reads
    // the policy. One layer of escaping would leave the policy value ending at
    // its first quote.
    assert!(
        html.contains(
            "srcdoc=\"&lt;!doctype html&gt;&lt;html lang=&quot;en&quot;&gt;&lt;head&gt;\
             &lt;meta charset=&quot;utf-8&quot;&gt;&lt;meta http-equiv=\
             &quot;Content-Security-Policy&quot; content=&quot;default-src &amp;#39;none&amp;#39;; \
             base-uri &amp;#39;none&amp;#39;; form-action &amp;#39;none&amp;#39;; style-src \
             &amp;#39;unsafe-inline&amp;#39;; script-src &amp;#39;unsafe-inline&amp;#39;&quot;&gt;"
        ),
        "the frame's document must open with the capability policy: {html}"
    );

    // The block's own code is inside the frame's document and nowhere else:
    // the page around it never gains a script or a style of the tenant's.
    assert!(
        html.contains("&lt;script&gt;document.getElementById(&#39;go&#39;)"),
        "{html}"
    );
    assert!(
        !html.contains("<script>document.getElementById"),
        "the block's script escaped into the page itself: {html}"
    );
    assert!(
        !html.contains("<p id=\"left\">12:00</p>"),
        "the block's markup escaped into the page itself: {html}"
    );
}

/// A block that declares nothing gets a frame that can do nothing: an empty
/// `sandbox`, a policy with no `script-src`, and — the part worth pinning — no
/// `<script>` block at all, rather than bytes the browser is required to
/// ignore.
#[test]
fn a_custom_code_block_without_capabilities_carries_no_script() {
    let mut block = custom_code_block();
    block.js = None;
    block.capabilities = CustomCodeCapabilities::default();
    let html = render_default(vec![Section::CustomCode(block)]);

    assert!(html.contains("sandbox=\"\""), "{html}");
    assert!(!html.contains("script-src"), "{html}");
    assert!(!html.contains("&lt;script&gt;"), "{html}");
    assert!(
        html.contains(
            "content=&quot;default-src &amp;#39;none&amp;#39;; base-uri &amp;#39;none&amp;#39;; \
             form-action &amp;#39;none&amp;#39;; style-src &amp;#39;unsafe-inline&amp;#39;&quot;"
        ),
        "{html}"
    );
}

/// Defense in depth for a snapshot published before the write gate forbade it:
/// a stored part that would close its own `<style>`/`<script>` block is dropped
/// rather than inlined, so the frame's document cannot be re-parsed into
/// something else.
#[test]
fn stored_code_that_would_close_its_own_block_is_dropped() {
    let mut block = custom_code_block();
    block.css = Some("body { color: red; }</style><p>escaped".to_owned());
    block.js = Some("const x = 1;</script><p>escaped".to_owned());
    let html = render_default(vec![Section::CustomCode(block)]);

    assert!(
        !html.contains("escaped"),
        "a part that closes its own block must be dropped whole: {html}"
    );
    // The frame is still there, still locked, still named.
    assert!(html.contains("sandbox=\"allow-scripts\""), "{html}");
    assert!(html.contains("&lt;style&gt;html,body{margin:0"), "{html}");
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
