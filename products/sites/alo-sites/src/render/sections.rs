//! One HTML fragment builder per section type.
//!
//! Markup is semantic and CSS-free: landmarks and heading levels carry the
//! structure (`h1` only in a hero, `h2` per section, `h3` per item), classes
//! are stable `s-<kind>` hooks for the generated stylesheet, and every
//! `<img>` carries `alt` (empty means decorative, straight from the model).
//! All text goes through [`esc`], every link target through [`safe_href`].

use alo_store::site_model::{
    BookingSection, CatalogSection, CollectionSection, ContactFormSection, CtaSection, FaqSection,
    FeaturesSection, FooterSection, GallerySection, HeroSection, ImageSide, Link, NavSection,
    PricingSection, Section, SiteImage, TeamSection, TestimonialsSection, TextImageSection,
};
use alo_store::{
    SiteBookingSnapshot, SiteCatalogSnapshot, SiteCatalogSnapshotItem, SiteCollectionSnapshot,
};

use crate::images::ImageSlot;

use super::html::{esc, safe_href};
use super::money::format_price;
use super::{PageRenderContext, SiteRenderContext};

/// A `nav` section, rendered as a `<header>` landmark. The brand link shows
/// the theme logo when one is set, the site name otherwise; the toggle
/// button is wired by the behavior script and hidden (menu expanded) when
/// JavaScript is unavailable.
pub(super) fn nav(out: &mut String, site: &SiteRenderContext<'_>, s: &NavSection, index: usize) {
    let menu_id = format!("nav-menu-{index}");
    out.push_str("<header class=\"s-nav\">\n");
    out.push_str(&format!(
        "<nav aria-label=\"{}\">\n",
        esc(site.strings.nav_label)
    ));
    match &site.theme.logo {
        Some(logo) => out.push_str(&format!(
            "<a class=\"brand\" href=\"/\"><img class=\"logo\" src=\"{}\" alt=\"{}\"></a>\n",
            site.images.src(logo.as_str()),
            esc(site.name)
        )),
        None => out.push_str(&format!(
            "<a class=\"brand\" href=\"/\">{}</a>\n",
            esc(site.name)
        )),
    }
    out.push_str(&format!(
        "<button class=\"nav-toggle\" type=\"button\" aria-expanded=\"false\" aria-controls=\"{menu_id}\">{}</button>\n",
        esc(site.strings.menu)
    ));
    out.push_str(&format!("<ul id=\"{menu_id}\">\n"));
    for link in &s.links {
        out.push_str("<li>");
        push_link(out, link, "");
        out.push_str("</li>\n");
    }
    if let Some(cta) = &s.cta {
        out.push_str("<li>");
        push_link(out, cta, "button");
        out.push_str("</li>\n");
    }
    out.push_str("</ul>\n</nav>\n</header>\n");
}

/// A `footer` section, rendered as a `<footer>` landmark.
pub(super) fn footer(out: &mut String, site: &SiteRenderContext<'_>, s: &FooterSection) {
    out.push_str("<footer class=\"s-footer\">\n");
    if !s.links.is_empty() {
        out.push_str(&format!(
            "<nav aria-label=\"{}\">\n<ul>\n",
            esc(site.strings.footer_nav_label)
        ));
        for link in &s.links {
            out.push_str("<li>");
            push_link(out, link, "");
            out.push_str("</li>\n");
        }
        out.push_str("</ul>\n</nav>\n");
    }
    if let Some(text) = &s.text {
        out.push_str(&format!("<p>{}</p>\n", esc(text)));
    }
    out.push_str("</footer>\n");
}

/// Every section that lives inside `<main>`. Nav/footer never reach here
/// (the document assembler routes them to their landmarks); if one ever
/// does, it renders in place rather than vanish.
pub(super) fn body_section(
    out: &mut String,
    site: &SiteRenderContext<'_>,
    page: &PageRenderContext<'_>,
    section: &Section,
    index: usize,
) {
    match section {
        Section::Nav(s) => nav(out, site, s, index),
        Section::Footer(s) => footer(out, site, s),
        Section::Hero(s) => hero(out, site, s),
        Section::Features(s) => features(out, s),
        Section::TextImage(s) => text_image(out, site, s),
        Section::Gallery(s) => gallery(out, site, s),
        Section::Testimonials(s) => testimonials(out, s),
        Section::Pricing(s) => pricing(out, s),
        Section::Team(s) => team(out, site, s),
        Section::Faq(s) => faq(out, s),
        Section::Cta(s) => cta(out, s),
        Section::ContactForm(s) => contact_form(out, site, s, index),
        Section::Collection(s) => collection(out, site, s, page.collections),
        Section::Catalog(s) => catalog(out, site, s, page.catalogs, index),
        Section::Booking(s) => booking(out, site, s, page.bookings, index),
    }
}

/// A `catalog` section: what the site offers, exactly as it was frozen into
/// this publish. Items are grouped under their category in the catalog's own
/// order, and the items belonging to no category close the list. A section may
/// name one category, in which case only that group renders — a menu page can
/// show starters and mains as two sections without duplicating the catalog.
///
/// When the catalog was published with ordering switched on, the whole section
/// is one `POST` form: each available item carries a quantity field, and the
/// contact fields close it. It works with no JavaScript at all — the browser
/// posts it and lands on the service's own result page — because an order
/// nobody can place is worse than no order form.
fn catalog(
    out: &mut String,
    site: &SiteRenderContext<'_>,
    section: &CatalogSection,
    snapshots: &std::collections::HashMap<String, SiteCatalogSnapshot>,
    index: usize,
) {
    out.push_str("<section class=\"s-catalog\">\n");
    push_opt_heading(out, section.heading.as_deref());
    let Some(snapshot) = snapshots.get(section.catalog_id.as_str()) else {
        tracing::warn!(
            catalog = %section.catalog_id,
            "published page references a missing catalog snapshot"
        );
        push_catalog_empty(out, site);
        return;
    };
    let wanted = section.category.as_deref();
    let mut groups: Vec<(Option<&str>, Vec<&SiteCatalogSnapshotItem>)> = Vec::new();
    for category in &snapshot.categories {
        if wanted.is_some_and(|wanted| wanted != category.slug) {
            continue;
        }
        let items: Vec<&SiteCatalogSnapshotItem> = snapshot
            .items
            .iter()
            .filter(|item| item.category.as_deref() == Some(category.slug.as_str()))
            .collect();
        if !items.is_empty() {
            groups.push((Some(category.name.as_str()), items));
        }
    }
    if wanted.is_none() {
        let known: Vec<&str> = snapshot
            .categories
            .iter()
            .map(|category| category.slug.as_str())
            .collect();
        // An item whose category vanished between two publishes still belongs
        // on the page; it joins the ungrouped items rather than disappearing.
        let loose: Vec<&SiteCatalogSnapshotItem> = snapshot
            .items
            .iter()
            .filter(|item| {
                item.category
                    .as_deref()
                    .is_none_or(|slug| !known.contains(&slug))
            })
            .collect();
        if !loose.is_empty() {
            groups.push((None, loose));
        }
    }
    if groups.is_empty() {
        push_catalog_empty(out, site);
        return;
    }
    // Ordering needs something orderable: a catalog published with ordering on
    // but every item sold out renders as a plain list rather than a form the
    // visitor could only submit empty.
    let ordering = snapshot.orders_enabled
        && groups
            .iter()
            .any(|(_, items)| items.iter().any(|item| !item.sold_out));
    if ordering {
        out.push_str(&format!(
            "<form class=\"catalog-order\" action=\"/o/{}\" method=\"post\">\n",
            esc(snapshot.catalog_id.as_str())
        ));
    }
    for (name, items) in groups {
        out.push_str("<div class=\"catalog-group\">\n");
        if let Some(name) = name {
            out.push_str(&format!("<h3>{}</h3>\n", esc(name)));
        }
        out.push_str("<ul class=\"catalog-list\">\n");
        for item in items {
            catalog_item(
                out,
                site,
                item,
                &snapshot.currency,
                ordering.then_some(index),
            );
        }
        out.push_str("</ul>\n</div>\n");
    }
    if ordering {
        push_order_fields(out, site, index);
        out.push_str("</form>\n");
    }
    out.push_str("</section>\n");
}

/// The contact half of an order form: the honeypot no human fills, who to
/// answer, an optional note — and, above the button, the sentence that says
/// this is a request and not a purchase.
fn push_order_fields(out: &mut String, site: &SiteRenderContext<'_>, index: usize) {
    let t = site.strings;
    out.push_str("<div class=\"order-details\">\n");
    out.push_str(&format!(
        "<p class=\"hp\" aria-hidden=\"true\"><label for=\"order-{index}-website\">{}</label><input id=\"order-{index}-website\" name=\"website\" type=\"text\" tabindex=\"-1\" autocomplete=\"off\"></p>\n",
        esc(t.form_website)
    ));
    out.push_str(&format!(
        "<p><label for=\"order-{index}-name\">{}</label><input id=\"order-{index}-name\" name=\"name\" type=\"text\" required maxlength=\"200\" autocomplete=\"name\"></p>\n",
        esc(t.form_name)
    ));
    out.push_str(&format!(
        "<p><label for=\"order-{index}-email\">{}</label><input id=\"order-{index}-email\" name=\"email\" type=\"email\" required maxlength=\"254\" autocomplete=\"email\"></p>\n",
        esc(t.form_email)
    ));
    out.push_str(&format!(
        "<p><label for=\"order-{index}-phone\">{}</label><input id=\"order-{index}-phone\" name=\"phone\" type=\"tel\" maxlength=\"40\" autocomplete=\"tel\"></p>\n",
        esc(t.order_phone)
    ));
    out.push_str(&format!(
        "<p><label for=\"order-{index}-note\">{}</label><textarea id=\"order-{index}-note\" name=\"note\" maxlength=\"2000\"></textarea></p>\n",
        esc(t.order_note)
    ));
    out.push_str(&format!(
        "<p class=\"order-no-payment\">{}</p>\n",
        esc(t.order_no_payment)
    ));
    out.push_str(&format!(
        "<p><button type=\"submit\">{}</button></p>\n",
        esc(t.order_send)
    ));
    out.push_str("</div>\n");
}

fn catalog_item(
    out: &mut String,
    site: &SiteRenderContext<'_>,
    item: &SiteCatalogSnapshotItem,
    currency: &str,
    // The section index when this item can be ordered — it prefixes the field
    // ids so two catalog sections on one page never collide.
    ordering: Option<usize>,
) {
    out.push_str(&format!(
        "<li class=\"catalog-item\" id=\"item-{}\">\n",
        esc(&item.slug)
    ));
    if let Some(image) = &item.image {
        // What the picture shows, when the owner wrote it. Without it the
        // name is the honest fallback — it names the thing photographed, and
        // an empty `alt` here would claim the picture carries nothing.
        out.push_str(&format!(
            "<img src=\"{}\" alt=\"{}\">\n",
            site.images.src(image.as_str()),
            esc(item.image_alt.as_deref().unwrap_or(&item.name))
        ));
    }
    out.push_str(&format!("<h4>{}</h4>\n", esc(&item.name)));
    if let Some(price) = item.price_cents {
        out.push_str(&format!(
            "<p class=\"catalog-price\">{}",
            esc(&format_price(price, currency, site.strings))
        ));
        if let Some(note) = &item.price_note {
            out.push_str(&format!(" <span class=\"price-note\">{}</span>", esc(note)));
        }
        out.push_str("</p>\n");
    }
    if item.sold_out {
        out.push_str(&format!(
            "<p class=\"catalog-unavailable\">{}</p>\n",
            esc(site.strings.catalog_sold_out)
        ));
    }
    if let Some(description) = &item.description {
        out.push_str(&format!(
            "<p class=\"catalog-description\">{}</p>\n",
            esc(description)
        ));
    }
    // A sold-out item carries no quantity field: the public order door refuses
    // it anyway, so offering the field would be a promise the page cannot keep.
    if let Some(index) = ordering.filter(|_| !item.sold_out) {
        let slug = esc(&item.slug);
        out.push_str(&format!(
            "<p class=\"catalog-qty\"><label for=\"order-{index}-qty-{slug}\">{}</label><input id=\"order-{index}-qty-{slug}\" name=\"qty-{slug}\" type=\"number\" min=\"0\" max=\"{max}\" step=\"1\" value=\"0\" inputmode=\"numeric\"></p>\n",
            esc(site.strings.order_quantity),
            max = alo_store::ORDER_MAX_QUANTITY
        ));
    }
    out.push_str("</li>\n");
}

/// The one empty state a catalog section has — the same sentence whether the
/// snapshot is missing, filtered to nothing, or genuinely empty, so a visitor
/// never learns anything about the tenant's editing state from it.
fn push_catalog_empty(out: &mut String, site: &SiteRenderContext<'_>) {
    out.push_str(&format!(
        "<p class=\"catalog-empty\">{}</p>\n</section>\n",
        esc(site.strings.catalog_empty)
    ));
}

/// A `booking` section: something a visitor may book, as the publish froze it.
///
/// The page says what is offered — the name, how long it takes, where it
/// happens — and asks for a day; the free times themselves are read live on
/// `/b/<booking id>`, because a published page is cached bytes and a free
/// afternoon is not. That is one navigation away from the answer and works with
/// no JavaScript at all, which is the same trade the order form makes.
///
/// A service switched off before the publish says so instead of offering a form
/// that could only fail.
fn booking(
    out: &mut String,
    site: &SiteRenderContext<'_>,
    section: &BookingSection,
    snapshots: &std::collections::HashMap<String, SiteBookingSnapshot>,
    index: usize,
) {
    let t = site.strings;
    out.push_str("<section class=\"s-booking\">\n");
    push_opt_heading(out, section.heading.as_deref());
    let Some(snapshot) = snapshots.get(section.booking_id.as_str()) else {
        tracing::warn!(
            booking = %section.booking_id,
            "published page references a missing booking snapshot"
        );
        out.push_str(&format!(
            "<p class=\"booking-closed\">{}</p>\n</section>\n",
            esc(t.booking_closed)
        ));
        return;
    };
    out.push_str(&format!("<h3>{}</h3>\n", esc(&snapshot.name)));
    out.push_str(&format!(
        "<p class=\"booking-length\">{} {}</p>\n",
        snapshot.duration_minutes,
        esc(t.booking_minutes)
    ));
    if let Some(description) = &snapshot.description {
        out.push_str(&format!(
            "<p class=\"booking-description\">{}</p>\n",
            esc(description)
        ));
    }
    if let Some(location) = &snapshot.location {
        out.push_str(&format!(
            "<p class=\"booking-where\">{}: {}</p>\n",
            esc(t.booking_where),
            esc(location)
        ));
    }
    if !snapshot.active {
        out.push_str(&format!(
            "<p class=\"booking-closed\">{}</p>\n</section>\n",
            esc(t.booking_closed)
        ));
        return;
    }
    out.push_str(&format!(
        "<form class=\"booking-day\" action=\"/b/{}\" method=\"get\">\n",
        esc(snapshot.booking_id.as_str())
    ));
    out.push_str(&format!(
        "<p><label for=\"booking-{index}-date\">{}</label>\
         <input id=\"booking-{index}-date\" name=\"date\" type=\"date\" required></p>\n",
        esc(t.booking_choose_day)
    ));
    out.push_str(&format!(
        "<p><button type=\"submit\">{}</button></p>\n",
        esc(t.booking_see_times)
    ));
    out.push_str("</form>\n</section>\n");
}

fn collection(
    out: &mut String,
    site: &SiteRenderContext<'_>,
    section: &CollectionSection,
    snapshots: &std::collections::HashMap<String, SiteCollectionSnapshot>,
) {
    out.push_str("<section class=\"s-collection\">\n");
    push_opt_heading(out, section.heading.as_deref());
    let Some(snapshot) = snapshots.get(section.collection_id.as_str()) else {
        tracing::warn!(
            collection = %section.collection_id,
            "published page references a missing collection snapshot"
        );
        out.push_str(&format!(
            "<p class=\"collection-empty\">{}</p>\n",
            esc(site.strings.collection_empty)
        ));
        out.push_str("</section>\n");
        return;
    };
    if snapshot.items.is_empty() {
        out.push_str(&format!(
            "<p class=\"collection-empty\">{}</p>\n",
            esc(site.strings.collection_empty)
        ));
        out.push_str("</section>\n");
        return;
    }
    out.push_str("<ul class=\"collection-grid\">\n");
    for item in &snapshot.items {
        out.push_str("<li class=\"collection-card\"");
        if let Some(slug) = &item.slug {
            out.push_str(&format!(" id=\"collection-{}\"", esc(slug)));
        }
        out.push_str(">\n");
        if let Some(image) = &item.image {
            out.push_str(&format!(
                "<img src=\"{}\" alt=\"{}\">\n",
                site.images.src(image.as_str()),
                esc(&item.title)
            ));
        }
        match &item.link {
            Some(link) => out.push_str(&format!(
                "<h3><a href=\"{}\">{}</a></h3>\n",
                safe_href(link),
                esc(&item.title)
            )),
            None => out.push_str(&format!("<h3>{}</h3>\n", esc(&item.title))),
        }
        if let Some(summary) = &item.summary {
            out.push_str(&format!(
                "<p class=\"collection-summary\">{}</p>\n",
                esc(summary)
            ));
        }
        if let Some(body) = &item.body {
            out.push_str(&format!("<p class=\"collection-body\">{}</p>\n", esc(body)));
        }
        if let Some(date) = &item.published_at {
            out.push_str(&format!(
                "<time datetime=\"{}\">{}</time>\n",
                esc(date),
                esc(date)
            ));
        }
        out.push_str("</li>\n");
    }
    out.push_str("</ul>\n</section>\n");
}

fn hero(out: &mut String, site: &SiteRenderContext<'_>, s: &HeroSection) {
    out.push_str("<section class=\"s-hero\">\n");
    out.push_str(&format!("<h1>{}</h1>\n", esc(&s.heading)));
    if let Some(subheading) = &s.subheading {
        out.push_str(&format!(
            "<p class=\"subheading\">{}</p>\n",
            esc(subheading)
        ));
    }
    if s.primary_cta.is_some() || s.secondary_cta.is_some() {
        out.push_str("<p class=\"actions\">");
        if let Some(link) = &s.primary_cta {
            push_link(out, link, "button");
        }
        if let Some(link) = &s.secondary_cta {
            push_link(out, link, "button secondary");
        }
        out.push_str("</p>\n");
    }
    if let Some(image) = &s.image {
        push_figure(out, site, image, ImageSlot::Banner);
    }
    out.push_str("</section>\n");
}

fn features(out: &mut String, s: &FeaturesSection) {
    out.push_str("<section class=\"s-features\">\n");
    push_opt_heading(out, s.heading.as_deref());
    if let Some(intro) = &s.intro {
        out.push_str(&format!("<p class=\"intro\">{}</p>\n", esc(intro)));
    }
    out.push_str("<ul class=\"grid\">\n");
    for item in &s.items {
        out.push_str(&format!(
            "<li>\n<h3>{}</h3>\n<p>{}</p>\n</li>\n",
            esc(&item.title),
            esc(&item.body)
        ));
    }
    out.push_str("</ul>\n</section>\n");
}

fn text_image(out: &mut String, site: &SiteRenderContext<'_>, s: &TextImageSection) {
    let side = match s.image_side {
        ImageSide::Left => "image-left",
        ImageSide::Right => "image-right",
    };
    out.push_str(&format!("<section class=\"s-text-image {side}\">\n"));
    push_figure(out, site, &s.image, ImageSlot::Half);
    out.push_str("<div class=\"text\">\n");
    push_opt_heading(out, s.heading.as_deref());
    out.push_str(&format!("<p>{}</p>\n", esc(&s.body)));
    out.push_str("</div>\n</section>\n");
}

fn gallery(out: &mut String, site: &SiteRenderContext<'_>, s: &GallerySection) {
    out.push_str("<section class=\"s-gallery\">\n");
    push_opt_heading(out, s.heading.as_deref());
    out.push_str("<ul class=\"grid\">\n");
    for image in &s.images {
        out.push_str("<li>");
        push_figure(out, site, image, ImageSlot::Card);
        out.push_str("</li>\n");
    }
    out.push_str("</ul>\n</section>\n");
}

fn testimonials(out: &mut String, s: &TestimonialsSection) {
    out.push_str("<section class=\"s-testimonials\">\n");
    push_opt_heading(out, s.heading.as_deref());
    out.push_str("<ul>\n");
    for item in &s.items {
        out.push_str("<li>\n<figure class=\"testimonial\">\n");
        out.push_str(&format!(
            "<blockquote><p>{}</p></blockquote>\n",
            esc(&item.quote)
        ));
        out.push_str(&format!("<figcaption>{}", esc(&item.author)));
        if let Some(role) = &item.role {
            out.push_str(&format!(" <span class=\"role\">{}</span>", esc(role)));
        }
        out.push_str("</figcaption>\n</figure>\n</li>\n");
    }
    out.push_str("</ul>\n</section>\n");
}

fn pricing(out: &mut String, s: &PricingSection) {
    out.push_str("<section class=\"s-pricing\">\n");
    push_opt_heading(out, s.heading.as_deref());
    if let Some(intro) = &s.intro {
        out.push_str(&format!("<p class=\"intro\">{}</p>\n", esc(intro)));
    }
    out.push_str("<ul class=\"tiers\">\n");
    for tier in &s.tiers {
        let class = if tier.highlighted {
            "tier highlighted"
        } else {
            "tier"
        };
        out.push_str(&format!("<li class=\"{class}\">\n"));
        out.push_str(&format!("<h3>{}</h3>\n", esc(&tier.name)));
        out.push_str(&format!("<p class=\"price\">{}", esc(&tier.price)));
        if let Some(period) = &tier.period {
            out.push_str(&format!(" <span class=\"period\">{}</span>", esc(period)));
        }
        out.push_str("</p>\n");
        if let Some(description) = &tier.description {
            out.push_str(&format!(
                "<p class=\"description\">{}</p>\n",
                esc(description)
            ));
        }
        if !tier.features.is_empty() {
            out.push_str("<ul class=\"tier-features\">\n");
            for feature in &tier.features {
                out.push_str(&format!("<li>{}</li>\n", esc(feature)));
            }
            out.push_str("</ul>\n");
        }
        if let Some(link) = &tier.cta {
            push_link(out, link, "button");
            out.push('\n');
        }
        out.push_str("</li>\n");
    }
    out.push_str("</ul>\n</section>\n");
}

fn team(out: &mut String, site: &SiteRenderContext<'_>, s: &TeamSection) {
    out.push_str("<section class=\"s-team\">\n");
    push_opt_heading(out, s.heading.as_deref());
    out.push_str("<ul class=\"grid\">\n");
    for member in &s.members {
        out.push_str("<li>\n");
        if let Some(photo) = &member.photo {
            push_figure(out, site, photo, ImageSlot::Card);
        }
        out.push_str(&format!("<h3>{}</h3>\n", esc(&member.name)));
        if let Some(role) = &member.role {
            out.push_str(&format!("<p class=\"role\">{}</p>\n", esc(role)));
        }
        if let Some(bio) = &member.bio {
            out.push_str(&format!("<p class=\"bio\">{}</p>\n", esc(bio)));
        }
        out.push_str("</li>\n");
    }
    out.push_str("</ul>\n</section>\n");
}

fn faq(out: &mut String, s: &FaqSection) {
    out.push_str("<section class=\"s-faq\">\n");
    push_opt_heading(out, s.heading.as_deref());
    for item in &s.items {
        // <details>/<summary> is a native, scriptless accordion.
        out.push_str(&format!(
            "<details>\n<summary>{}</summary>\n<p>{}</p>\n</details>\n",
            esc(&item.question),
            esc(&item.answer)
        ));
    }
    out.push_str("</section>\n");
}

fn cta(out: &mut String, s: &CtaSection) {
    out.push_str("<section class=\"s-cta\">\n");
    out.push_str(&format!("<h2>{}</h2>\n", esc(&s.heading)));
    if let Some(body) = &s.body {
        out.push_str(&format!("<p>{}</p>\n", esc(body)));
    }
    out.push_str("<p class=\"actions\">");
    push_link(out, &s.button, "button");
    out.push_str("</p>\n</section>\n");
}

/// The contact form. Without a `form_id` the section renders its text only —
/// "the section without a working submit". With one, the form posts to
/// `/f/<form_id>` and carries the fixed v1 field contract — `name`, `email`,
/// `message`, plus the visually-hidden `website` honeypot (a submission
/// filling it is bot traffic, silently dropped by the forms backend).
fn contact_form(
    out: &mut String,
    site: &SiteRenderContext<'_>,
    s: &ContactFormSection,
    index: usize,
) {
    out.push_str("<section class=\"s-contact-form\">\n");
    push_opt_heading(out, s.heading.as_deref());
    if let Some(body) = &s.body {
        out.push_str(&format!("<p>{}</p>\n", esc(body)));
    }
    if let Some(form_id) = &s.form_id {
        let t = site.strings;
        // data-success is always present (custom message or the localized
        // default) — it is the script's only success copy.
        let success = s.success_message.as_deref().unwrap_or(t.form_success);
        out.push_str(&format!(
            "<form action=\"/f/{}\" method=\"post\" data-success=\"{}\">\n",
            esc(form_id),
            esc(success)
        ));
        out.push_str(&format!(
            "<p class=\"hp\" aria-hidden=\"true\"><label for=\"form-{index}-website\">{}</label><input id=\"form-{index}-website\" name=\"website\" type=\"text\" tabindex=\"-1\" autocomplete=\"off\"></p>\n",
            esc(t.form_website)
        ));
        out.push_str(&format!(
            "<p><label for=\"form-{index}-name\">{}</label><input id=\"form-{index}-name\" name=\"name\" type=\"text\" required maxlength=\"300\"></p>\n",
            esc(t.form_name)
        ));
        out.push_str(&format!(
            "<p><label for=\"form-{index}-email\">{}</label><input id=\"form-{index}-email\" name=\"email\" type=\"email\" required maxlength=\"320\"></p>\n",
            esc(t.form_email)
        ));
        out.push_str(&format!(
            "<p><label for=\"form-{index}-message\">{}</label><textarea id=\"form-{index}-message\" name=\"message\" required maxlength=\"5000\"></textarea></p>\n",
            esc(t.form_message)
        ));
        out.push_str(&format!(
            "<p><button type=\"submit\">{}</button></p>\n",
            esc(t.form_send)
        ));
        out.push_str("</form>\n");
    }
    out.push_str("</section>\n");
}

// ---- shared fragments -------------------------------------------------------

/// `<a>` with a safe href; `class` may be empty.
fn push_link(out: &mut String, link: &Link, class: &str) {
    if class.is_empty() {
        out.push_str(&format!(
            "<a href=\"{}\">{}</a>",
            safe_href(&link.href),
            esc(&link.label)
        ));
    } else {
        out.push_str(&format!(
            "<a class=\"{class}\" href=\"{}\">{}</a>",
            safe_href(&link.href),
            esc(&link.label)
        ));
    }
}

/// `<figure><img></figure>` — `alt` is always present; empty means the model
/// marked the image decorative.
///
/// On a published page the image is responsive: `srcset` offers the
/// derivative ladder ([`crate::images`]) and `sizes` says how wide the slot
/// will be, so a phone downloads a phone-sized photo. The `src` is the
/// fallback for anything that ignores `srcset` — the original for an unframed
/// photo, the widest derivative for a cropped one, because the original is
/// the picture *before* the owner framed it. The draft preview carries the
/// bytes inline and offers neither attribute.
fn push_figure(out: &mut String, site: &SiteRenderContext<'_>, image: &SiteImage, slot: ImageSlot) {
    let (src, srcset) = site.images.figure_src(image);
    out.push_str(&format!("<figure><img src=\"{src}\""));
    if let Some(srcset) = srcset {
        out.push_str(&format!(" srcset=\"{srcset}\" sizes=\"{}\"", slot.sizes()));
    }
    if slot.lazy() {
        out.push_str(" loading=\"lazy\" decoding=\"async\"");
    }
    out.push_str(&format!(" alt=\"{}\"></figure>\n", esc(&image.alt)));
}

/// The section's optional `<h2>`.
fn push_opt_heading(out: &mut String, heading: Option<&str>) {
    if let Some(heading) = heading {
        out.push_str(&format!("<h2>{}</h2>\n", esc(heading)));
    }
}
