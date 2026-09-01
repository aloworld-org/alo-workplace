//! One HTML fragment builder per section type.
//!
//! Markup is semantic and CSS-free: landmarks and heading levels carry the
//! structure (`h1` only in a hero, `h2` per section, `h3` per item), classes
//! are stable `s-<kind>` hooks for the generated stylesheet, and every
//! `<img>` carries `alt` (empty means decorative, straight from the model).
//! All text goes through [`esc`], every link target through [`safe_href`].

use alo_store::site_custom_code::CustomCodeSection;
use alo_store::site_layout::GridColumns;
use alo_store::site_model::{
    BookingSection, CatalogSection, CollectionSection, ContactFormSection, CtaSection, FaqSection,
    FeaturesSection, FooterSection, GallerySection, HeroSection, ImageSide, Link, NavSection,
    PricingSection, Section, SectionPresentation, SiteImage, TeamSection, TestimonialsSection,
    TextImageSection, TransitionDirection, TransitionEffect, TransitionSection, TransitionSpeed,
    TransitionTrigger,
};
use alo_store::{
    SiteBookingSnapshot, SiteCatalogSnapshot, SiteCatalogSnapshotItem, SiteCollectionSnapshot,
};

use crate::images::ImageSlot;

use super::html::{esc, safe_href, safe_video_src};
use super::money::format_price;
use super::{PageRenderContext, SiteRenderContext};

/// Which typed field a rendered element came from — the whole of direct
/// manipulation on the editor side (ADR 0042).
///
/// Two annotations, both only in the editable draft preview. A section's root
/// element carries `data-alo-section="<index>"`, which is the coordinate a
/// `reorder_section` operation names, so a section dragged on the page and a
/// section the assistant moves are the same change (S3.01b). And every
/// element whose text is **exactly one
/// typed string** carries `data-alo-text="<section index><JSON pointer>"`,
/// e.g. `data-alo-text="2/items/0/title"`. That is the same coordinate a
/// `rewrite_copy` edit operation names, so a person typing on the page and a
/// model proposing a rewrite produce the identical change — there is no second
/// edit path (`alo_ai::SiteEditOperation::RewriteCopy`).
///
/// Two rules keep it honest:
///
/// - **One element, one string.** An element that also carries a second
///   property (a testimonial's `figcaption`, which holds the author *and* the
///   role; a tier's price, which holds the amount *and* the period) is not
///   marked: making it editable would let one gesture rewrite two properties,
///   and the resulting diff would be a guess. Those keep the prop form until
///   their markup gives each string its own element.
/// - **Nothing but the preview is annotated.** Published bytes are identical
///   with and without this module: [`Marks::at`] returns the empty string
///   unless the document is the editable preview, which the golden suite pins.
///
/// A pointer is built from fixed literals and array indices — never from
/// tenant data — so it needs no escaping to sit in an attribute.
#[derive(Clone, Copy)]
pub(super) struct Marks {
    /// Position of this section in the page envelope: the `index` half of an
    /// edit target, and the id prefix sections already needed for their form
    /// fields.
    pub(super) index: usize,
    editable: bool,
    anchor: &'static str,
    occurrence: usize,
}

impl Marks {
    /// Marks for the section at `index`; `editable` only in the draft preview
    /// the editor renders.
    pub(super) fn new(
        index: usize,
        editable: bool,
        anchor: &'static str,
        occurrence: usize,
    ) -> Self {
        Self {
            index,
            editable,
            anchor,
            occurrence,
        }
    }

    /// The same section with nothing editable: what a `custom_code` block
    /// gets. The assistant may not write custom code either
    /// (`alo_ai::site_edits` refuses every `set_prop`/`rewrite_copy` aimed at
    /// one), and the two paths have to agree — an outline inviting a click the
    /// edit door would refuse is worse than no outline.
    pub(super) fn sealed(self) -> Self {
        Self {
            editable: false,
            ..self
        }
    }

    /// The attribute tying this element to `pointer` inside the section, ready
    /// to sit inside a start tag (it carries its own leading space). Empty
    /// outside the editable preview.
    pub(super) fn at(self, pointer: &str) -> String {
        if !self.editable {
            return String::new();
        }
        format!(" data-alo-text=\"{}{pointer}\"", self.index)
    }

    /// The attribute naming this section's **position in the page envelope**,
    /// on the element that is the whole section — the coordinate a
    /// `reorder_section` operation names (S3.01b).
    ///
    /// One element per section, so a drag has exactly one thing to pick up and
    /// the editor has exactly one number to send. `sealed()` does not remove
    /// it: a custom-code block may be moved and deleted like any other section
    /// (`alo_ai::site_edits` refuses only *writing* one), so it is draggable
    /// even though nothing inside it is typed into.
    pub(super) fn block(self) -> String {
        if !self.editable {
            return String::new();
        }
        format!(" data-alo-section=\"{}\"", self.index)
    }

    fn anchor(self) -> String {
        if self.occurrence == 1 {
            self.anchor.to_owned()
        } else {
            format!("{}-{}", self.anchor, self.occurrence)
        }
    }
}

/// Opens a section's root element: `<section class="s-hero">`, plus its
/// position when the document is the editable preview. Every section type goes
/// through here, so "one element per section, carrying its index" is a
/// property of the renderer rather than of sixteen remembered call sites.
pub(super) fn open_section(out: &mut String, tag: &str, class: &str, m: Marks) {
    open_section_with_style(out, tag, class, None, m);
}

fn open_section_with_style(
    out: &mut String,
    tag: &str,
    class: &str,
    style: Option<&str>,
    m: Marks,
) {
    let anchor = if tag == "section" {
        format!(" id=\"{}\"", m.anchor())
    } else {
        String::new()
    };
    let style = style.map_or_else(String::new, |value| format!(" style=\"{value}\""));
    out.push_str(&format!(
        "<{tag} class=\"{class}\"{anchor}{style}{}>\n",
        m.block()
    ));
}

/// The section's classes with its chosen layout appended — the one place a
/// resize (ADR 0042, S3.01c) becomes markup. An unset choice appends nothing,
/// so a page stored before the schema gained the property renders exactly the
/// bytes it always did, and the stylesheet's own default is what a section
/// without a class gets.
fn with_layout(base: &str, layout: Option<&'static str>) -> String {
    match layout {
        Some(class) => format!("{base} {class}"),
        None => base.to_owned(),
    }
}

/// The class for a card grid's chosen column count.
fn columns_class(columns: Option<GridColumns>) -> Option<&'static str> {
    columns.map(GridColumns::class)
}

/// A `nav` section, rendered as a `<header>` landmark. The brand link shows
/// the theme logo when one is set, the site name otherwise; the toggle
/// button is wired by the behavior script and hidden (menu expanded) when
/// JavaScript is unavailable.
pub(super) fn nav(
    out: &mut String,
    site: &SiteRenderContext<'_>,
    page: &PageRenderContext<'_>,
    s: &NavSection,
    m: Marks,
) {
    let menu_id = format!("nav-menu-{}", m.index);
    let style = s.appearance.as_ref().map(|appearance| {
        format!(
            " style=\"--nav-bg:var(--{});--nav-text:var(--{});--nav-hover:var(--{})\"",
            theme_role(appearance.background),
            theme_role(appearance.text),
            theme_role(appearance.hover),
        )
    });
    out.push_str(&format!(
        "<header class=\"s-nav\"{}{}>\n",
        style.unwrap_or_default(),
        m.block()
    ));
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
        push_nav_link(out, link, "", page.path);
        out.push_str("</li>\n");
    }
    if let Some(cta) = &s.cta {
        out.push_str("<li>");
        push_nav_link(out, cta, "button", page.path);
        out.push_str("</li>\n");
    }
    out.push_str("</ul>\n</nav>\n</header>\n");
}

fn theme_role(role: alo_store::site_model::ThemeColorRole) -> &'static str {
    use alo_store::site_model::ThemeColorRole;
    match role {
        ThemeColorRole::Background => "bg",
        ThemeColorRole::Text => "text",
        ThemeColorRole::Border => "border",
        ThemeColorRole::Accent1 => "accent-1",
        ThemeColorRole::Accent2 => "accent-2",
        ThemeColorRole::Accent3 => "accent-3",
        ThemeColorRole::Accent4 => "accent-4",
        ThemeColorRole::Accent5 => "accent-5",
    }
}

fn theme_on_role(role: alo_store::site_model::ThemeColorRole) -> &'static str {
    use alo_store::site_model::ThemeColorRole;
    match role {
        ThemeColorRole::Background => "on-bg",
        ThemeColorRole::Text => "on-text",
        ThemeColorRole::Border => "on-border",
        ThemeColorRole::Accent1 => "on-accent-1",
        ThemeColorRole::Accent2 => "on-accent-2",
        ThemeColorRole::Accent3 => "on-accent-3",
        ThemeColorRole::Accent4 => "on-accent-4",
        ThemeColorRole::Accent5 => "on-accent-5",
    }
}

fn open_presented_section(
    out: &mut String,
    class: &str,
    presentation: Option<&SectionPresentation>,
    m: Marks,
) {
    let Some(p) = presentation else {
        open_section(out, "section", class, m);
        return;
    };
    let layout = match p.layout {
        alo_store::site_model::SectionLayoutStyle::Clean => "section-clean",
        alo_store::site_model::SectionLayoutStyle::Cards => "section-cards",
        alo_store::site_model::SectionLayoutStyle::Minimal => "section-minimal",
        alo_store::site_model::SectionLayoutStyle::Editorial => "section-editorial",
    };
    let spacing = match p.spacing {
        alo_store::site_model::SectionSpacing::Compact => "section-spacing-compact",
        alo_store::site_model::SectionSpacing::Standard => "section-spacing-standard",
        alo_store::site_model::SectionSpacing::Generous => "section-spacing-generous",
    };
    let width = match p.width {
        alo_store::site_model::SectionWidth::Narrow => "section-width-narrow",
        alo_store::site_model::SectionWidth::Balanced => "section-width-balanced",
        alo_store::site_model::SectionWidth::Wide => "section-width-wide",
    };
    let alignment = match p.alignment {
        alo_store::site_model::SectionAlignment::Left => "section-align-left",
        alo_store::site_model::SectionAlignment::Center => "section-align-center",
    };
    let entrance = match p.entrance {
        alo_store::site_model::SectionEntrance::None => "",
        alo_store::site_model::SectionEntrance::FadeUp => " section-motion section-enter-fade-up",
        alo_store::site_model::SectionEntrance::SlideIn => " section-motion section-enter-slide-in",
        alo_store::site_model::SectionEntrance::ScaleIn => " section-motion section-enter-scale-in",
        alo_store::site_model::SectionEntrance::Reveal => " section-motion section-enter-reveal",
    };
    let speed = match p.speed {
        TransitionSpeed::Quick => "section-speed-quick",
        TransitionSpeed::Smooth => "section-speed-smooth",
        TransitionSpeed::Relaxed => "section-speed-relaxed",
    };
    let button_text = p
        .button_text
        .map_or_else(|| theme_on_role(p.button), theme_role);
    let hover_text = p
        .button_hover_text
        .map_or_else(|| theme_on_role(p.button_hover), theme_role);
    let classes = format!(
        "{class} section-presented {layout} {spacing} {width} {alignment} {speed}{entrance}"
    );
    let style = format!(
        "--section-bg:var(--{});--section-text:var(--{});--section-button:var(--{});--section-button-text:var(--{});--section-button-hover:var(--{});--section-button-hover-text:var(--{})",
        theme_role(p.background),
        theme_role(p.text),
        theme_role(p.button),
        button_text,
        theme_role(p.button_hover),
        hover_text,
    );
    open_section_with_style(out, "section", &classes, Some(&style), m);
}

/// A `footer` section, rendered as a `<footer>` landmark.
pub(super) fn footer(out: &mut String, site: &SiteRenderContext<'_>, s: &FooterSection, m: Marks) {
    open_section(out, "footer", "s-footer", m);
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
        out.push_str(&format!("<p{}>{}</p>\n", m.at("/text"), esc(text)));
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
    m: Marks,
) {
    match section {
        Section::Nav(s) => nav(out, site, page, s, m),
        Section::Footer(s) => footer(out, site, s, m),
        Section::Hero(s) => hero(out, site, s, m),
        Section::Features(s) => features(out, s, m),
        Section::TextImage(s) => text_image(out, site, s, m),
        Section::Gallery(s) => gallery(out, site, s, m),
        Section::Testimonials(s) => testimonials(out, s, m),
        Section::Pricing(s) => pricing(out, s, m),
        Section::Team(s) => team(out, site, s, m),
        Section::Faq(s) => faq(out, s, m),
        Section::Cta(s) => cta(out, s, m),
        Section::ContactForm(s) => contact_form(out, site, s, m),
        Section::Collection(s) => collection(out, site, s, page.collections, m),
        Section::Catalog(s) => catalog(out, site, s, page.catalogs, m),
        Section::Booking(s) => booking(out, site, s, page.bookings, m),
        Section::Tickets(s) => tickets(out, site, s, m),
        Section::Shop(s) => shop(out, site, s, m),
        Section::Transition(s) => transition(out, s, m),
        Section::CustomCode(s) => custom_code(out, site, s, m),
    }
}

fn transition(out: &mut String, s: &TransitionSection, m: Marks) {
    let effect = match s.effect {
        TransitionEffect::Fade => "fade",
        TransitionEffect::Slide => "slide",
        TransitionEffect::Scale => "scale",
        TransitionEffect::Reveal => "reveal",
    };
    let direction = match s.direction {
        TransitionDirection::Up => "up",
        TransitionDirection::Down => "down",
        TransitionDirection::Left => "left",
        TransitionDirection::Right => "right",
    };
    let speed = match s.speed {
        TransitionSpeed::Quick => "quick",
        TransitionSpeed::Smooth => "smooth",
        TransitionSpeed::Relaxed => "relaxed",
    };
    let trigger = match s.trigger {
        TransitionTrigger::Early => "early",
        TransitionTrigger::Balanced => "balanced",
        TransitionTrigger::Late => "late",
    };
    out.push_str(&format!(
        "<div class=\"s-transition\" data-effect=\"{effect}\" data-direction=\"{direction}\" data-speed=\"{speed}\" data-trigger=\"{trigger}\" data-out=\"{}\" aria-hidden=\"true\"{}></div>\n",
        s.animate_out,
        m.block(),
    ));
}

/// A `tickets` section: the door to the site's live ticket shop.
///
/// The section itself is static bytes — an optional heading, the owner's own
/// line, and one link — because everything a buyer decides on (what is on
/// sale, the price, what is left) is live Billing state served on `/tix`,
/// one navigation away and never cached. That is the same trade the booking
/// section makes with its free times, and it is what keeps published pages
/// immutable while prices are not.
fn tickets(
    out: &mut String,
    site: &SiteRenderContext<'_>,
    section: &alo_store::site_model::TicketsSection,
    m: Marks,
) {
    open_presented_section(out, "s-tickets", section.presentation.as_ref(), m);
    push_opt_heading(out, section.heading.as_deref(), m);
    if let Some(body) = &section.body {
        out.push_str(&format!("<p{}>{}</p>\n", m.at("/body"), esc(body)));
    }
    out.push_str(&format!(
        "<p class=\"actions\"><a class=\"button\" href=\"/tix\">{}</a></p>\n",
        esc(site.strings.tickets_see_offer)
    ));
    out.push_str("</section>\n");
}

/// A `shop` section: the door to the site's live stock shop.
///
/// Static bytes for the same reason the tickets section is (item S3.05a3):
/// what is for sale, its price and what is on the shelf are the owning
/// seams' live answers, served on `/shop` one navigation away and never
/// cached — a published page stays immutable while prices and shelves move.
fn shop(
    out: &mut String,
    site: &SiteRenderContext<'_>,
    section: &alo_store::site_model::ShopSection,
    m: Marks,
) {
    open_presented_section(out, "s-shop", section.presentation.as_ref(), m);
    push_opt_heading(out, section.heading.as_deref(), m);
    if let Some(body) = &section.body {
        out.push_str(&format!("<p{}>{}</p>\n", m.at("/body"), esc(body)));
    }
    out.push_str(&format!(
        "<p class=\"actions\"><a class=\"button\" href=\"/shop\">{}</a></p>\n",
        esc(site.strings.shop_see_offer)
    ));
    out.push_str("</section>\n");
}

/// A `custom_code` section: the tenant's own markup, style, and script, served
/// as a **complete document inside a sandboxed frame** rather than as part of
/// this page.
///
/// The frame is where the isolation lives. `sandbox` never carries
/// `allow-same-origin`, so the document inside gets an opaque origin and cannot
/// read this page, its cookies, or its storage; the policy it declares starts
/// at `default-src 'none'`, so it cannot fetch anything, from anywhere. Both
/// strings come from [`CustomCodeCapabilities`] rather than from this file, so
/// what the write gate promised and what the browser is told can never drift.
///
/// The whole document is `srcdoc`, attribute-escaped: there is no second URL to
/// serve, nothing to cache separately, and no way for the value to end the
/// attribute it sits in. Independently of the write gate, a stored part that
/// would close its own `<style>`/`<script>` block is dropped here with a
/// warning — a snapshot published before that rule existed still renders inert.
fn custom_code(
    out: &mut String,
    site: &SiteRenderContext<'_>,
    section: &CustomCodeSection,
    m: Marks,
) {
    open_section(out, "section", "s-custom-code", m);
    push_opt_heading(out, section.heading.as_deref(), m.sealed());
    out.push_str(&format!(
        "<iframe class=\"custom-frame\" title=\"{}\" sandbox=\"{}\" loading=\"lazy\" \
         style=\"height:{}px\" srcdoc=\"{}\"></iframe>\n",
        esc(&section.title),
        section.capabilities.sandbox_attribute(),
        section.height_px,
        esc(&custom_code_document(site, section))
    ));
    out.push_str("</section>\n");
}

/// The document inside the frame, before attribute escaping.
fn custom_code_document(site: &SiteRenderContext<'_>, section: &CustomCodeSection) -> String {
    let mut doc = format!(
        "<!doctype html><html lang=\"{}\"><head><meta charset=\"utf-8\">\
         <meta http-equiv=\"Content-Security-Policy\" content=\"{}\">\
         <meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
         <style>html,body{{margin:0;font-family:inherit}}",
        esc(site.locale),
        esc(&section.capabilities.content_security_policy()),
    );
    if let Some(css) = section.css.as_deref().and_then(|css| inlinable("css", css)) {
        doc.push_str(css);
    }
    doc.push_str("</style></head><body>");
    doc.push_str(&section.html);
    // A script is inlined only when the capability that runs it was declared:
    // without `allow-scripts` the frame would not execute it anyway, and
    // shipping bytes the browser is contractually required to ignore is a lie
    // in the page source.
    if section.capabilities.scripts
        && let Some(js) = section.js.as_deref().and_then(|js| inlinable("js", js))
    {
        doc.push_str("<script>");
        doc.push_str(js);
        doc.push_str("</script>");
    }
    doc.push_str("</body></html>");
    doc
}

/// The write gate's `</` rule, re-checked at render time: a value inlined into
/// a `<style>` or `<script>` block may not end that block. A stored part that
/// does is dropped whole — the block renders without its style or its
/// behaviour, which is visibly wrong and therefore fixable, rather than
/// half-parsed.
fn inlinable<'a>(field: &'static str, value: &'a str) -> Option<&'a str> {
    if value.contains("</") {
        tracing::warn!(
            field,
            "stored custom code would close its own block; dropping it"
        );
        return None;
    }
    Some(value)
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
    m: Marks,
) {
    open_presented_section(out, "s-catalog", section.presentation.as_ref(), m);
    push_opt_heading(out, section.heading.as_deref(), m);
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
                ordering.then_some(m.index),
            );
        }
        out.push_str("</ul>\n</div>\n");
    }
    if ordering {
        push_order_fields(out, site, m.index);
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
    m: Marks,
) {
    let t = site.strings;
    open_presented_section(out, "s-booking", section.presentation.as_ref(), m);
    push_opt_heading(out, section.heading.as_deref(), m);
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
        esc(t.booking_choose_day),
        index = m.index
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
    m: Marks,
) {
    open_presented_section(out, "s-collection", section.presentation.as_ref(), m);
    push_opt_heading(out, section.heading.as_deref(), m);
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

fn hero(out: &mut String, site: &SiteRenderContext<'_>, s: &HeroSection, m: Marks) {
    let mut classes = vec!["s-hero"];
    if let Some(layout) = s.layout {
        classes.push(layout.class());
    }
    if let Some(height) = s.height {
        classes.push(height.class());
    }
    if let Some(alignment) = s.alignment {
        classes.push(alignment.class());
    }
    if let Some(width) = s.content_width {
        classes.push(width.class());
    }
    let mut animated = false;
    if let Some(class) = s.text_animation.and_then(|animation| animation.class()) {
        classes.push(class);
        animated = true;
    }
    if let Some(class) = s.media_animation.and_then(|animation| animation.class()) {
        classes.push(class);
        animated = true;
    }
    if animated {
        classes.push(
            s.animation_speed
                .unwrap_or(alo_store::site_model::HeroAnimationSpeed::Smooth)
                .class(),
        );
    }
    if s.image.is_some()
        && matches!(
            s.layout,
            Some(alo_store::site_model::HeroLayout::SplitRight)
                | Some(alo_store::site_model::HeroLayout::SplitLeft)
        )
    {
        classes.push("hero-has-image");
    }
    let appearance = s.appearance.as_ref().map(|appearance| {
        classes.push("hero-custom-appearance");
        format!(
            "--hero-bg:var(--{});--hero-text:var(--{});\
             --hero-primary:var(--{});--hero-primary-text:var(--{});\
             --hero-primary-hover:var(--{});--hero-primary-hover-text:var(--{});\
             --hero-secondary:var(--{});--hero-secondary-text:var(--{});\
             --hero-secondary-hover:var(--{});--hero-secondary-hover-text:var(--{})",
            theme_role(appearance.background),
            theme_on_role(appearance.background),
            theme_role(appearance.primary_button),
            appearance
                .primary_button_text
                .map_or_else(|| theme_on_role(appearance.primary_button), theme_role),
            theme_role(appearance.primary_button_hover),
            appearance.primary_button_hover_text.map_or_else(
                || theme_on_role(appearance.primary_button_hover),
                theme_role,
            ),
            theme_role(appearance.secondary_button),
            appearance
                .secondary_button_text
                .map_or_else(|| theme_on_role(appearance.secondary_button), theme_role,),
            theme_role(appearance.secondary_button_hover),
            appearance.secondary_button_hover_text.map_or_else(
                || theme_on_role(appearance.secondary_button_hover),
                theme_role,
            ),
        )
    });
    open_section_with_style(out, "section", &classes.join(" "), appearance.as_deref(), m);
    if matches!(
        s.text_animation,
        Some(alo_store::site_model::HeroTextAnimation::WordReveal)
    ) {
        out.push_str(&format!("<h1{}>", m.at("/heading")));
        for (index, word) in s.heading.split_whitespace().enumerate() {
            if index > 0 {
                out.push(' ');
            }
            out.push_str(&format!(
                "<span class=\"hero-word\" style=\"--hero-word-delay:{}ms\">{}</span>",
                index * 70,
                esc(word)
            ));
        }
        out.push_str("</h1>\n");
    } else {
        out.push_str(&format!(
            "<h1{}>{}</h1>\n",
            m.at("/heading"),
            esc(&s.heading)
        ));
    }
    if let Some(subheading) = &s.subheading {
        out.push_str(&format!(
            "<p class=\"subheading\"{}>{}</p>\n",
            m.at("/subheading"),
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
    if matches!(
        s.layout,
        Some(alo_store::site_model::HeroLayout::VideoBackground)
    ) && let Some(src) = s.video_url.as_deref().and_then(safe_video_src)
    {
        let poster = s.image.as_ref().map_or_else(String::new, |image| {
            format!(" poster=\"{}\"", site.images.src(image.blob_id.as_str()))
        });
        out.push_str(&format!(
            "<video class=\"hero-video\" autoplay muted loop playsinline preload=\"metadata\" aria-hidden=\"true\" tabindex=\"-1\"{poster}><source src=\"{src}\"></video>\n"
        ));
    }
    if let Some(image) = &s.image {
        push_figure(out, site, image, ImageSlot::Banner);
    }
    out.push_str("</section>\n");
}

fn features(out: &mut String, s: &FeaturesSection, m: Marks) {
    let layout = match s
        .layout
        .unwrap_or(alo_store::site_model::FeaturesLayout::Grid)
    {
        alo_store::site_model::FeaturesLayout::Grid => "features-grid",
        alo_store::site_model::FeaturesLayout::Bento => "features-bento",
        alo_store::site_model::FeaturesLayout::List => "features-list",
        alo_store::site_model::FeaturesLayout::Steps => "features-steps",
        alo_store::site_model::FeaturesLayout::Spotlight => "features-spotlight",
    };
    open_presented_section(
        out,
        &format!(
            "{} {layout}",
            with_layout("s-features", columns_class(s.columns))
        ),
        s.presentation.as_ref(),
        m,
    );
    push_opt_heading(out, s.heading.as_deref(), m);
    if let Some(intro) = &s.intro {
        out.push_str(&format!(
            "<p class=\"intro\"{}>{}</p>\n",
            m.at("/intro"),
            esc(intro)
        ));
    }
    out.push_str("<ul class=\"grid\">\n");
    for (i, item) in s.items.iter().enumerate() {
        out.push_str(&format!(
            "<li>\n<h3{}>{}</h3>\n<p{}>{}</p>\n</li>\n",
            m.at(&format!("/items/{i}/title")),
            esc(&item.title),
            m.at(&format!("/items/{i}/body")),
            esc(&item.body)
        ));
    }
    out.push_str("</ul>\n</section>\n");
}

fn text_image(out: &mut String, site: &SiteRenderContext<'_>, s: &TextImageSection, m: Marks) {
    let side = match s.image_side {
        ImageSide::Left => "image-left",
        ImageSide::Right => "image-right",
    };
    let layout = match s
        .layout
        .unwrap_or(alo_store::site_model::TextImageLayout::Split)
    {
        alo_store::site_model::TextImageLayout::Split => "text-image-split",
        alo_store::site_model::TextImageLayout::Overlap => "text-image-overlap",
        alo_store::site_model::TextImageLayout::Framed => "text-image-framed",
        alo_store::site_model::TextImageLayout::Editorial => "text-image-editorial",
        alo_store::site_model::TextImageLayout::FullBleed => "text-image-full-bleed",
    };
    open_presented_section(
        out,
        &with_layout(
            &format!("s-text-image {side} {layout}"),
            s.split.map(alo_store::site_layout::ColumnSplit::class),
        ),
        s.presentation.as_ref(),
        m,
    );
    push_figure(out, site, &s.image, ImageSlot::Half);
    out.push_str("<div class=\"text\">\n");
    push_opt_heading(out, s.heading.as_deref(), m);
    out.push_str(&format!("<p{}>{}</p>\n", m.at("/body"), esc(&s.body)));
    out.push_str("</div>\n</section>\n");
}

fn gallery(out: &mut String, site: &SiteRenderContext<'_>, s: &GallerySection, m: Marks) {
    open_presented_section(
        out,
        &with_layout("s-gallery", columns_class(s.columns)),
        s.presentation.as_ref(),
        m,
    );
    push_opt_heading(out, s.heading.as_deref(), m);
    out.push_str("<ul class=\"grid\">\n");
    for image in &s.images {
        out.push_str("<li>");
        push_figure(out, site, image, ImageSlot::Card);
        out.push_str("</li>\n");
    }
    out.push_str("</ul>\n</section>\n");
}

fn testimonials(out: &mut String, s: &TestimonialsSection, m: Marks) {
    open_presented_section(out, "s-testimonials", s.presentation.as_ref(), m);
    push_opt_heading(out, s.heading.as_deref(), m);
    out.push_str("<ul>\n");
    for (i, item) in s.items.iter().enumerate() {
        out.push_str("<li>\n<figure class=\"testimonial\">\n");
        // The `figcaption` below holds the author *and* the role, so neither
        // is marked: see [`Marks`] on why one element may carry one string.
        out.push_str(&format!(
            "<blockquote><p{}>{}</p></blockquote>\n",
            m.at(&format!("/items/{i}/quote")),
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

fn pricing(out: &mut String, s: &PricingSection, m: Marks) {
    open_presented_section(out, "s-pricing", s.presentation.as_ref(), m);
    push_opt_heading(out, s.heading.as_deref(), m);
    if let Some(intro) = &s.intro {
        out.push_str(&format!(
            "<p class=\"intro\"{}>{}</p>\n",
            m.at("/intro"),
            esc(intro)
        ));
    }
    out.push_str("<ul class=\"tiers\">\n");
    for (i, tier) in s.tiers.iter().enumerate() {
        let class = if tier.highlighted {
            "tier highlighted"
        } else {
            "tier"
        };
        out.push_str(&format!("<li class=\"{class}\">\n"));
        out.push_str(&format!(
            "<h3{}>{}</h3>\n",
            m.at(&format!("/tiers/{i}/name")),
            esc(&tier.name)
        ));
        // Price and period share one paragraph, so neither is marked ([`Marks`]).
        out.push_str(&format!("<p class=\"price\">{}", esc(&tier.price)));
        if let Some(period) = &tier.period {
            out.push_str(&format!(" <span class=\"period\">{}</span>", esc(period)));
        }
        out.push_str("</p>\n");
        if let Some(description) = &tier.description {
            out.push_str(&format!(
                "<p class=\"description\"{}>{}</p>\n",
                m.at(&format!("/tiers/{i}/description")),
                esc(description)
            ));
        }
        if !tier.features.is_empty() {
            out.push_str("<ul class=\"tier-features\">\n");
            for (f, feature) in tier.features.iter().enumerate() {
                out.push_str(&format!(
                    "<li{}>{}</li>\n",
                    m.at(&format!("/tiers/{i}/features/{f}")),
                    esc(feature)
                ));
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

fn team(out: &mut String, site: &SiteRenderContext<'_>, s: &TeamSection, m: Marks) {
    open_presented_section(
        out,
        &with_layout("s-team", columns_class(s.columns)),
        s.presentation.as_ref(),
        m,
    );
    push_opt_heading(out, s.heading.as_deref(), m);
    out.push_str("<ul class=\"grid\">\n");
    for (i, member) in s.members.iter().enumerate() {
        out.push_str("<li>\n");
        if let Some(photo) = &member.photo {
            push_figure(out, site, photo, ImageSlot::Card);
        }
        out.push_str(&format!(
            "<h3{}>{}</h3>\n",
            m.at(&format!("/members/{i}/name")),
            esc(&member.name)
        ));
        if let Some(role) = &member.role {
            out.push_str(&format!(
                "<p class=\"role\"{}>{}</p>\n",
                m.at(&format!("/members/{i}/role")),
                esc(role)
            ));
        }
        if let Some(bio) = &member.bio {
            out.push_str(&format!(
                "<p class=\"bio\"{}>{}</p>\n",
                m.at(&format!("/members/{i}/bio")),
                esc(bio)
            ));
        }
        out.push_str("</li>\n");
    }
    out.push_str("</ul>\n</section>\n");
}

fn faq(out: &mut String, s: &FaqSection, m: Marks) {
    open_presented_section(out, "s-faq", s.presentation.as_ref(), m);
    push_opt_heading(out, s.heading.as_deref(), m);
    for (i, item) in s.items.iter().enumerate() {
        // <details>/<summary> is a native, scriptless accordion.
        out.push_str(&format!(
            "<details>\n<summary{}>{}</summary>\n<p{}>{}</p>\n</details>\n",
            m.at(&format!("/items/{i}/question")),
            esc(&item.question),
            m.at(&format!("/items/{i}/answer")),
            esc(&item.answer)
        ));
    }
    out.push_str("</section>\n");
}

fn cta(out: &mut String, s: &CtaSection, m: Marks) {
    open_presented_section(out, "s-cta", s.presentation.as_ref(), m);
    out.push_str(&format!(
        "<h2{}>{}</h2>\n",
        m.at("/heading"),
        esc(&s.heading)
    ));
    if let Some(body) = &s.body {
        out.push_str(&format!("<p{}>{}</p>\n", m.at("/body"), esc(body)));
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
fn contact_form(out: &mut String, site: &SiteRenderContext<'_>, s: &ContactFormSection, m: Marks) {
    let index = m.index;
    open_presented_section(out, "s-contact-form", s.presentation.as_ref(), m);
    push_opt_heading(out, s.heading.as_deref(), m);
    if let Some(body) = &s.body {
        out.push_str(&format!("<p{}>{}</p>\n", m.at("/body"), esc(body)));
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

/// A header link with its page relationship exposed to sighted and assistive
/// technology users. Only exact site-relative paths are current; anchors,
/// external URLs and actions remain ordinary links.
fn push_nav_link(out: &mut String, link: &Link, class: &str, page_path: &str) {
    let current = (link.href == page_path).then_some(" aria-current=\"page\"");
    if class.is_empty() {
        out.push_str(&format!(
            "<a href=\"{}\"{}>{}</a>",
            safe_href(&link.href),
            current.unwrap_or_default(),
            esc(&link.label)
        ));
    } else {
        out.push_str(&format!(
            "<a class=\"{class}\" href=\"{}\"{}>{}</a>",
            safe_href(&link.href),
            current.unwrap_or_default(),
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
    match image
        .shape
        .and_then(alo_store::site_layout::ImageShape::class)
    {
        Some(shape) => out.push_str(&format!("<figure class=\"{shape}\">")),
        None => out.push_str("<figure>"),
    }
    out.push_str(&format!("<img src=\"{src}\""));
    if let Some(srcset) = srcset {
        out.push_str(&format!(" srcset=\"{srcset}\" sizes=\"{}\"", slot.sizes()));
    }
    if slot.lazy() {
        out.push_str(" loading=\"lazy\" decoding=\"async\"");
    }
    out.push_str(&format!(" alt=\"{}\"></figure>\n", esc(&image.alt)));
}

/// The section's optional `<h2>`, editable in place like every other single
/// typed string.
fn push_opt_heading(out: &mut String, heading: Option<&str>, m: Marks) {
    if let Some(heading) = heading {
        out.push_str(&format!("<h2{}>{}</h2>\n", m.at("/heading"), esc(heading)));
    }
}
