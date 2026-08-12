//! One publish, rendered: the exact bytes the service serves for a site —
//! every page document, the stylesheet, and the site's not-found page —
//! built once from the frozen snapshots and shared immutably from the cache.

use std::collections::{HashMap, HashSet};

use alo_store::site_theme::SiteTheme;
use alo_store::{PublishedSite, SiteCollectionSnapshot, SitePageSnapshot, SitePublishId};

use crate::render::{
    self, ImageSources, LanguageAlternate, PageRenderContext, SiteRenderContext, UiStrings,
    render_localized_page, strings_for,
};
use crate::stylesheet;

use super::unlock::UnlockNotice;

/// The servable output of one publish of one site.
pub struct RenderedSite {
    /// The publish these bytes were rendered from — the cache-validity key
    /// and the substance of the pages' `ETag`s.
    pub publish: SitePublishId,
    /// Complete HTML documents by site-relative path (`/`, `/about`, …).
    pages: HashMap<String, String>,
    /// Canonical page paths in navigation order. Kept separately from the
    /// render map because sitemap order must be stable across processes.
    page_paths: Vec<String>,
    /// Exact sibling-language paths for each canonical page path.
    page_alternates: HashMap<String, Vec<(String, String)>>,
    /// The page identity behind each canonical path. Every language of a page
    /// shares it, which is what makes one password protect the page in all of
    /// them ([`super::unlock`]).
    page_ids: HashMap<String, String>,
    /// The exact language of each canonical path, so an unlock screen speaks
    /// the language of the page it is standing in front of.
    page_locales: HashMap<String, String>,
    /// The site's name, theme, and origin, kept for documents that are
    /// rendered per request rather than per publish — currently the unlock
    /// screen, which depends on live protection state, not on the publish.
    name: String,
    base_url: String,
    theme: SiteTheme,
    /// The one stylesheet, served at `/assets/site.css`.
    pub css: String,
    /// The site's themed not-found document (status 404, any unknown path).
    pub not_found: String,
    /// The blob ids this publish's documents reference (theme logo/favicon +
    /// section images) — the only ids `/assets/img/<blob_id>` will serve for
    /// this site. Collected from the same frozen content the pages were
    /// rendered from, so what is servable is exactly what is shown.
    images: HashSet<String>,
    /// The derivative keys this publish's documents reference — every
    /// `<blob>/w<width>` and `<blob>/c…/w<width>` that appears in a `srcset`,
    /// and nothing else. A width or a frame the pages never asked for is not
    /// servable, so the resize pipeline can only ever be asked for work this
    /// site's own HTML already promises ([`crate::images`]).
    variants: HashSet<String>,
}

impl RenderedSite {
    /// Renders every frozen page of `site`'s current publish. `public_host`
    /// forms the canonical HTTPS origin used for canonical/OG URLs, whether
    /// it is a built-in subdomain or a connected custom domain.
    #[must_use]
    pub fn build(
        public_host: &str,
        site: &PublishedSite,
        snapshots: &[SitePageSnapshot],
        collection_snapshots: &[SiteCollectionSnapshot],
    ) -> Self {
        let theme = SiteTheme::from_stored(site.theme.clone());
        let base_url = format!("https://{public_host}");
        let collections: HashMap<String, SiteCollectionSnapshot> = collection_snapshots
            .iter()
            .cloned()
            .map(|snapshot| (snapshot.collection_id.as_str().to_owned(), snapshot))
            .collect();
        let mut variants: HashMap<String, Vec<(String, String)>> = HashMap::new();
        for snapshot in snapshots {
            variants
                .entry(snapshot.page_id.as_str().to_owned())
                .or_default()
                .push((
                    snapshot.locale.clone(),
                    localized_path(
                        &site.default_locale,
                        &snapshot.locale,
                        snapshot.is_home,
                        &snapshot.slug,
                    ),
                ));
        }
        for translations in variants.values_mut() {
            translations.sort_by_key(|(locale, _)| {
                site.enabled_locales
                    .iter()
                    .position(|enabled| enabled == locale)
                    .unwrap_or(usize::MAX)
            });
        }
        let mut pages = HashMap::with_capacity(snapshots.len());
        let mut page_paths = Vec::with_capacity(snapshots.len());
        let mut page_alternates = HashMap::with_capacity(snapshots.len());
        let mut page_ids = HashMap::with_capacity(snapshots.len());
        let mut page_locales = HashMap::with_capacity(snapshots.len());
        let mut images = HashSet::new();
        let mut image_variants = HashSet::new();
        images.extend(
            [theme.logo.as_ref(), theme.favicon.as_ref()]
                .into_iter()
                .flatten()
                .map(|blob| blob.as_str().to_owned()),
        );
        images.extend(
            collection_snapshots
                .iter()
                .flat_map(|snapshot| &snapshot.items)
                .filter_map(|item| item.image.as_ref())
                .map(|blob| blob.as_str().to_owned()),
        );
        for snapshot in snapshots {
            let path = localized_path(
                &site.default_locale,
                &snapshot.locale,
                snapshot.is_home,
                &snapshot.slug,
            );
            // The same lenient read the renderer uses, so the servable image
            // set can never disagree with what the documents reference.
            for section in render::sections_lenient(&snapshot.sections) {
                for image in section.images() {
                    images.insert(image.blob_id.as_str().to_owned());
                    image_variants.extend(crate::images::variant_keys(image));
                }
            }
            let page = PageRenderContext {
                path: &path,
                title: &snapshot.title,
                seo_title: snapshot.seo_title.as_deref(),
                seo_description: snapshot.seo_description.as_deref(),
                sections: &snapshot.sections,
                collections: &collections,
            };
            let Some(translations) = variants.get(snapshot.page_id.as_str()) else {
                tracing::warn!(
                    page = %snapshot.page_id,
                    locale = %snapshot.locale,
                    "localized snapshot was absent from its render group"
                );
                continue;
            };
            let alternates: Vec<LanguageAlternate<'_>> = translations
                .iter()
                .map(|(locale, path)| LanguageAlternate {
                    locale,
                    path,
                    is_default: locale == &site.default_locale,
                })
                .collect();
            let ctx = SiteRenderContext {
                name: &site.name,
                base_url: &base_url,
                locale: &snapshot.locale,
                theme: &theme,
                strings: strings_for(&snapshot.locale),
                images: ImageSources::PublicPaths,
            };
            pages.insert(
                path.clone(),
                render_localized_page(&ctx, &page, &alternates),
            );
            page_alternates.insert(path.clone(), translations.clone());
            page_ids.insert(path.clone(), snapshot.page_id.as_str().to_owned());
            page_locales.insert(path.clone(), snapshot.locale.clone());
            page_paths.push(path);
        }
        let default_strings = strings_for(&site.default_locale);
        let default_ctx = SiteRenderContext {
            name: &site.name,
            base_url: &base_url,
            locale: &site.default_locale,
            theme: &theme,
            strings: default_strings,
            images: ImageSources::PublicPaths,
        };
        Self {
            publish: site.publish.clone(),
            pages,
            page_paths,
            page_alternates,
            page_ids,
            page_locales,
            css: stylesheet::stylesheet(&theme),
            not_found: render::render_not_found(&default_ctx),
            images,
            variants: image_variants,
            name: site.name.clone(),
            base_url,
            theme,
        }
    }

    /// The rendered document at `path`, if the publish has a page there.
    #[must_use]
    pub fn page(&self, path: &str) -> Option<&str> {
        self.pages.get(path).map(String::as_str)
    }

    /// Canonical page paths in the frozen publish's navigation order.
    pub fn page_paths(&self) -> &[String] {
        &self.page_paths
    }

    /// Exact translations of `path`, in the site's frozen language order.
    pub fn page_alternates(&self, path: &str) -> &[(String, String)] {
        self.page_alternates.get(path).map_or(&[], Vec::as_slice)
    }

    /// The page identity served at `path`, if the publish has a page there.
    /// Every language of a page answers the same id.
    #[must_use]
    pub fn page_id(&self, path: &str) -> Option<&str> {
        self.page_ids.get(path).map(String::as_str)
    }

    /// The unlock screen for the page at `path`, in that page's own language.
    /// Rendered per request rather than frozen with the publish because
    /// protection is live state: a password can be set, changed, or lifted
    /// between two requests of the same publish.
    #[must_use]
    pub fn challenge(&self, path: &str, notice: UnlockNotice) -> String {
        let locale = self.page_locales.get(path).map_or("", String::as_str);
        let strings = strings_for(locale);
        let ctx = SiteRenderContext {
            name: &self.name,
            base_url: &self.base_url,
            locale,
            theme: &self.theme,
            strings,
            images: ImageSources::PublicPaths,
        };
        render::render_password_challenge(&ctx, path, notice.text(strings))
    }

    /// Whether this publish references `blob_id` — the gate on the public
    /// image path: a live site serves exactly the images its published
    /// content shows, nothing else in the tenant.
    #[must_use]
    pub fn serves_image(&self, blob_id: &str) -> bool {
        self.images.contains(blob_id)
    }

    /// Whether this publish's own documents reference the derivative `key`
    /// (the part after `/assets/img/`) — the gate on the resize pipeline. It
    /// is checked before anything is read or decoded, so a width, a crop or a
    /// blob the published pages do not show costs one hash lookup.
    #[must_use]
    pub fn serves_variant(&self, key: &str) -> bool {
        self.variants.contains(key)
    }
}

fn localized_path(default_locale: &str, locale: &str, is_home: bool, slug: &str) -> String {
    match (locale == default_locale, is_home) {
        (true, true) => "/".to_owned(),
        (true, false) => format!("/{slug}"),
        (false, true) => format!("/{locale}"),
        (false, false) => format!("/{locale}/{slug}"),
    }
}

/// The generic not-found document for hosts that resolve to no live site.
/// One body for every miss — unknown subdomain, never-published site,
/// unpublished site, foreign domain — so the response can not leak whether
/// a tenant or site exists. Self-contained (no stylesheet path resolves
/// here), and built once at startup.
#[must_use]
pub fn unknown_host_not_found(strings: &UiStrings) -> String {
    minimal_document(
        strings.lang,
        strings.not_found_title,
        strings.not_found_text,
    )
}

/// A complete self-contained little document — the shell of the unknown-host
/// 404 and of the form-result pages a no-script submission lands on. Marked
/// `noindex` (none of these are content), no external stylesheet (no site
/// scope resolves here). Inputs are our own [`UiStrings`] constants or
/// store validation messages, never visitor text — nothing needs escaping.
#[must_use]
pub fn minimal_document(lang: &str, title: &str, text: &str) -> String {
    format!(
        "<!doctype html>\n<html lang=\"{lang}\">\n<head>\n<meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
         <title>{title}</title>\n<meta name=\"robots\" content=\"noindex\">\n\
         <style>body{{font-family:system-ui,sans-serif;display:grid;min-height:100vh;\
         margin:0;place-items:center;background:#fafafa;color:#1a1a1a}}\
         main{{text-align:center;padding:2rem}}h1{{font-size:1.5rem}}</style>\n\
         </head>\n<body>\n<main>\n<h1>{title}</h1>\n<p>{text}</p>\n</main>\n</body>\n</html>\n",
    )
}
