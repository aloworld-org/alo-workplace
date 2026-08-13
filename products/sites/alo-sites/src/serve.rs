//! The public HTTP surface (ADR 0036, `docs/design/sites.md`): resolve the
//! Host header to one tenant's live site, serve its published snapshots, and
//! accept contact-form submissions (`POST /f/{form_id}`, the [`forms`]
//! module), catalog orders (`POST /o/{catalog_id}`, the [`orders`] module)
//! and page-beacon reports (`POST /_alo/collect`, the [`beacon`],
//! [`heatmap`] and [`conversion`] modules) — nothing else. The service holds no session — its whole tenant
//! scope is the [`host`] lookup's result (or, for a submission, the posted
//! form id's own resolution) — and it is deliberately terse on the wire:
//! misses are one uniform not-found, errors carry no internals.
//!
//! Response semantics:
//! - Pages and the stylesheet are immutable per publish, so 200s carry a
//!   strong `ETag` built from the publish id and honor `If-None-Match`
//!   with `304`. `Cache-Control: public, max-age=60` bounds client
//!   staleness; the service itself is never stale (the per-request resolver
//!   read is what flips content on republish).
//! - `/assets/img/<blob_id>` serves image bytes for exactly the blob ids
//!   the served publish references (tenant-scoped through the resolved
//!   site); immutable per id, so `max-age=3600` with the id as `ETag`.
//! - `/assets/img/<blob_id>/[c<crop>/]w<width>` serves one derivative of such
//!   an image — the frame and the width the publish's own `srcset` names, and
//!   only those ([`crate::images`], [`derivative`]). Immutable per path, so
//!   the same validator and lifetime as the original.
//! - Unknown host → the generic not-found (identical for unknown and
//!   unpublished — no existence leak). Unknown path on a live site → the
//!   site's themed not-found. Both `404`, `no-cache`.
//! - A page carrying a password ([`unlock`]) is answered by the gate instead:
//!   `401` with the site's unlock screen until a signed session opens it, then
//!   the page itself as `private, no-store` with `Vary: Cookie` and no `ETag`.
//!   Protection is live state, read per request, so a password set or lifted a
//!   moment ago holds now rather than at the next publish — and the one `POST`
//!   this surface accepts is that page's own unlock form.
//! - Database trouble → `503` with a static line, `Retry-After: 10`.

mod analytics;
mod beacon;
mod blog;
mod bookings;
mod cache;
pub mod config;
mod conversion;
pub mod derivative;
mod forms;
mod heatmap;
mod host;
mod orders;
/// The public surface's anti-abuse budgets. Public because they are an
/// operational contract — an operator sizing a proxy, and the tests that pin
/// each budget, both need the numbers.
pub mod rate;
mod rendered;
mod unlock;

use std::sync::Arc;

use axum::Router;
use axum::extract::{DefaultBodyLimit, Query, Request, State};
use axum::http::{HeaderValue, Method, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};

use alo_store::{PublishedSite, SitePublicStore, StoreError};
use serde::Deserialize;

use crate::render::EN;
use crate::seo::{SITEMAP_URL_LIMIT, SitemapAlternate, SitemapUrl, render_robots, render_sitemap};
pub use config::{ConfigError, ServeConfig};
use rendered::RenderedSite;

/// Shared state of the public service.
pub struct AppState {
    store: SitePublicStore,
    sites_domain: String,
    cache: cache::SiteCache,
    /// The one body every host-level miss serves (built once).
    unknown_host: String,
    /// Resized images by tenant and derivative path, so a photo is decoded
    /// once per width rather than once per visitor.
    derivatives: cache::DerivativeCache,
    /// Per-client budget on the form-submit path (in-memory, transient).
    rate: rate::RateLimiter,
    /// The beacon's own budget ([`beacon`]), separate so a page's analytics
    /// traffic can never spend a visitor's form or unlock budget.
    beacon_rate: rate::RateLimiter,
    /// The separate, tighter budget on password guesses at protected pages.
    unlock_rate: rate::RateLimiter,
    /// Signs and checks visitor sessions on protected pages. Holds a derived
    /// key only — no session is ever stored.
    unlock: unlock::UnlockSessions,
    /// Secret-keyed visitor hashing. Raw identifiers never cross storage.
    analytics: analytics::VisitorHasher,
}

impl AppState {
    /// Wires the service state: the public store door and the apex domain
    /// (already lowercase, from [`ServeConfig`]).
    ///
    /// `secret` is the deployment's sites secret. Two independent keys are
    /// derived from it — daily visitor hashes for analytics and unlock-session
    /// signatures — so neither can be produced from the other.
    #[must_use]
    pub fn new(
        store: SitePublicStore,
        sites_domain: String,
        secret: impl AsRef<[u8]>,
    ) -> Arc<Self> {
        Arc::new(Self {
            store,
            sites_domain,
            cache: cache::SiteCache::default(),
            derivatives: cache::DerivativeCache::default(),
            unknown_host: rendered::unknown_host_not_found(&EN),
            rate: rate::RateLimiter::default(),
            beacon_rate: rate::RateLimiter::with_budget(
                rate::BEACON_MAX_PER_WINDOW,
                rate::BEACON_WINDOW,
            ),
            unlock_rate: rate::RateLimiter::with_budget(
                rate::UNLOCK_MAX_PER_WINDOW,
                rate::UNLOCK_WINDOW,
            ),
            unlock: unlock::UnlockSessions::new(&secret),
            analytics: analytics::VisitorHasher::new(secret),
        })
    }
}

/// The service router: `/healthz`, the three POSTs an anonymous visitor may
/// make off a page path — a form submission, a catalog order, and a page
/// beacon, each with its own tight body cap — and the catch-all site path.
/// Every POST path carries a shape no page slug or locale tag can (`/f/…`,
/// `/o/…`, `/_alo/…`), so none can ever shadow a page a tenant published.
pub fn app(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/internal/tls/ask", get(tls_ask))
        .route(
            "/f/{form_id}",
            post(forms::submit).layer(DefaultBodyLimit::max(forms::FORM_BODY_MAX_BYTES)),
        )
        .route(
            "/o/{catalog_id}",
            post(orders::place).layer(DefaultBodyLimit::max(orders::ORDER_BODY_MAX_BYTES)),
        )
        .route(
            "/b/{booking_id}",
            get(bookings::offer)
                .post(bookings::book)
                .layer(DefaultBodyLimit::max(bookings::BOOKING_BODY_MAX_BYTES)),
        )
        .route(
            "/_alo/collect",
            post(beacon::collect).layer(DefaultBodyLimit::max(beacon::BEACON_BODY_MAX_BYTES)),
        )
        .fallback(serve_site)
        .with_state(state)
}

/// Liveness: the process is up and routing. Deliberately does not touch the
/// database — a Postgres blip must not make the proxy mark every site dead.
async fn healthz() -> &'static str {
    "ok\n"
}

#[derive(Deserialize)]
struct TlsAsk {
    domain: String,
}

/// Caddy on-demand-TLS authorization. A 200 means the exact hostname is
/// already able to serve a live publish; every other outcome denies issuance.
/// This endpoint intentionally reveals no site or tenant metadata.
async fn tls_ask(State(state): State<Arc<AppState>>, Query(query): Query<TlsAsk>) -> Response {
    let Some(scope) = host::scope(&query.domain, &state.sites_domain) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    match resolve_scope(&state, &scope).await {
        Ok(Some(_)) => (
            StatusCode::OK,
            [(header::CACHE_CONTROL, HeaderValue::from_static("no-store"))],
        )
            .into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!(host = scope.host(), %error, "TLS authorization read failed");
            StatusCode::SERVICE_UNAVAILABLE.into_response()
        }
    }
}

async fn resolve_scope(
    state: &AppState,
    scope: &host::Scope,
) -> Result<Option<PublishedSite>, StoreError> {
    match scope {
        host::Scope::Subdomain { label, .. } => state.store.resolve_published(label).await,
        host::Scope::Custom { host } => state.store.resolve_custom_published(host).await,
    }
}

/// Serves one public request: Host → current publish → bytes.
///
/// `POST` is accepted on exactly one kind of path — a protected page, posting
/// its own unlock form ([`unlock`]). Everywhere else it is the same
/// method-not-allowed as any other verb.
async fn serve_site(State(state): State<Arc<AppState>>, req: Request) -> Response {
    if req.method() != Method::GET && req.method() != Method::HEAD && req.method() != Method::POST {
        return method_not_allowed();
    }

    let Some(scope) = req
        .headers()
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| host::scope(value, &state.sites_domain))
    else {
        return not_found(state.unknown_host.clone());
    };
    let public_host = scope.host().to_owned();

    let resolved = match resolve_scope(&state, &scope).await {
        Ok(Some(site)) => site,
        Ok(None) => return not_found(state.unknown_host.clone()),
        Err(error) => {
            tracing::error!(host = %public_host, %error, "resolver read failed");
            return unavailable();
        }
    };

    let site = match state.cache.get(&public_host, &resolved.publish) {
        Some(site) => site,
        None => {
            let snapshots = match state.store.published_pages(&resolved).await {
                Ok(snapshots) => snapshots,
                Err(error) => {
                    tracing::error!(host = %public_host, %error, "snapshot read failed");
                    return unavailable();
                }
            };
            let collections = match state.store.published_collections(&resolved).await {
                Ok(collections) => collections,
                Err(error) => {
                    tracing::error!(host = %public_host, %error, "collection snapshot read failed");
                    return unavailable();
                }
            };
            let catalogs = match state.store.published_catalogs(&resolved).await {
                Ok(catalogs) => catalogs,
                Err(error) => {
                    tracing::error!(host = %public_host, %error, "catalog snapshot read failed");
                    return unavailable();
                }
            };
            let bookings = match state.store.published_bookings(&resolved).await {
                Ok(bookings) => bookings,
                Err(error) => {
                    tracing::error!(host = %public_host, %error, "booking snapshot read failed");
                    return unavailable();
                }
            };
            let built = Arc::new(RenderedSite::build(
                &public_host,
                &resolved,
                &snapshots,
                &collections,
                &catalogs,
                &bookings,
            ));
            tracing::info!(
                host = %public_host,
                site = %resolved.site,
                publish = %resolved.publish,
                pages = snapshots.len(),
                collections = collections.len(),
                catalogs = catalogs.len(),
                bookings = bookings.len(),
                "rendered publish into cache"
            );
            state.cache.put(&public_host, Arc::clone(&built));
            built
        }
    };

    // `/about/` serves `/about` (the canonical URL in the document keeps
    // search engines on the slash-less form); everything else is exact.
    let raw = req.uri().path().to_owned();
    let trimmed = raw.trim_end_matches('/');
    let path = if trimmed.is_empty() { "/" } else { trimmed };

    // The one write an anonymous visitor may make on a page path: trying the
    // password of a protected page. Anything else posted here is a 405.
    if req.method() == Method::POST {
        let Some(page_id) = site.page_id(path).map(str::to_owned) else {
            return method_not_allowed();
        };
        let protections = match state.store.published_page_protections(&resolved).await {
            Ok(protections) => protections,
            Err(error) => {
                tracing::error!(host = %public_host, %error, "page protection read failed");
                return unavailable();
            }
        };
        if !protections
            .iter()
            .any(|protection| protection.page.as_str() == page_id)
        {
            return method_not_allowed();
        }
        return unlock::attempt(&state, &resolved, &site, &public_host, path, &page_id, req).await;
    }

    let analytics_visit = analytics::capture(&state, &req);

    let base_url = format!("https://{public_host}");
    if path == "/robots.txt" {
        return dynamic_text(render_robots(&base_url));
    }
    if path == "/sitemap.xml" {
        // A protected page is not a page search engines may be pointed at:
        // the sitemap is the site telling the internet what to come and read.
        let protected = match state.store.published_page_protections(&resolved).await {
            Ok(protections) => protections
                .into_iter()
                .map(|protection| protection.page.as_str().to_owned())
                .collect::<std::collections::HashSet<_>>(),
            Err(error) => {
                tracing::error!(host = %public_host, %error, "page protection read failed");
                return unavailable();
            }
        };
        let public_paths: Vec<&String> = site
            .page_paths()
            .iter()
            .filter(|path| {
                site.page_id(path)
                    .is_none_or(|page_id| !protected.contains(page_id))
            })
            .collect();
        let page_count = public_paths.len();
        let post_limit = SITEMAP_URL_LIMIT.saturating_sub(page_count + 1);
        let posts = if post_limit == 0 {
            Vec::new()
        } else {
            let limit = u32::try_from(post_limit).unwrap_or(u32::MAX);
            match state.store.published_posts_page(&resolved, 0, limit).await {
                Ok(page) => page.posts,
                Err(error) => {
                    tracing::error!(host = %public_host, %error, "sitemap post read failed");
                    return unavailable();
                }
            }
        };
        let mut urls: Vec<SitemapUrl> =
            Vec::with_capacity(page_count + usize::from(!posts.is_empty()) + posts.len());
        urls.extend(public_paths.into_iter().map(|path| {
            SitemapUrl {
                location: format!("{base_url}{path}"),
                alternates: site
                    .page_alternates(path)
                    .iter()
                    .map(|(locale, alternate_path)| SitemapAlternate {
                        is_default: locale == &resolved.default_locale,
                        locale: locale.clone(),
                        location: format!("{base_url}{alternate_path}"),
                    })
                    .collect(),
            }
        }));
        if !posts.is_empty() {
            urls.push(SitemapUrl::plain(format!("{base_url}/blog")));
            urls.extend(
                posts
                    .iter()
                    .map(|post| SitemapUrl::plain(format!("{base_url}/blog/{}", post.slug))),
            );
        }
        return dynamic_xml(render_sitemap(&urls));
    }

    // The image path of the public contract: bytes for exactly the blob ids
    // this publish references (`RenderedSite::serves_image`), read
    // tenant-scoped through the resolved site. Anything else — foreign,
    // unreferenced, or non-image — is the site's themed 404.
    if let Some(blob_id) = path.strip_prefix(crate::images::IMAGE_PATH_PREFIX) {
        // A derivative asks for a frame and a width on top of a blob id. Only
        // the exact derivatives this publish's own pages reference exist: the
        // membership check comes before any read, let alone any decode.
        if blob_id.contains('/') {
            if !site.serves_variant(blob_id) {
                tracing::debug!(host = %public_host, "image derivative not referenced by the served publish");
                return not_found(site.not_found.clone());
            }
            let Some(request) = crate::images::parse_variant(blob_id) else {
                tracing::warn!(host = %public_host, "a referenced derivative path did not parse");
                return not_found(site.not_found.clone());
            };
            return serve_derivative(
                &state,
                &resolved,
                blob_id,
                &request,
                req.headers(),
                site.not_found.clone(),
            )
            .await;
        }
        let referenced = if site.serves_image(blob_id) {
            true
        } else {
            match state
                .store
                .published_post_uses_cover(&resolved, blob_id)
                .await
            {
                Ok(referenced) => referenced,
                Err(error) => {
                    tracing::error!(host = %public_host, %error, "blog cover reference read failed");
                    return unavailable();
                }
            }
        };
        if !referenced {
            tracing::debug!(host = %public_host, "image not referenced by the served publish");
            return not_found(site.not_found.clone());
        }
        return serve_image(
            &state,
            &resolved,
            blob_id,
            req.headers(),
            site.not_found.clone(),
        )
        .await;
    }

    if path == "/blog" || path.starts_with("/blog/") {
        let response = blog::serve(
            &state,
            &resolved,
            &public_host,
            path,
            req.uri().query(),
            site.not_found.clone(),
        )
        .await;
        return analytics::record_html_view(&state, &resolved, path, analytics_visit, response)
            .await;
    }

    // A page carrying a password is answered by the gate, not by the cache:
    // protection is live state (`alo_store::site_page_protection`), so it is
    // read per request rather than frozen with the publish, and the bytes
    // behind it never get a cacheable answer.
    if let Some(page_id) = site.page_id(path) {
        let protections = match state.store.published_page_protections(&resolved).await {
            Ok(protections) => protections,
            Err(error) => {
                tracing::error!(host = %public_host, %error, "page protection read failed");
                return unavailable();
            }
        };
        if let Some(protection) = protections
            .iter()
            .find(|protection| protection.page.as_str() == page_id)
        {
            let cookies = req
                .headers()
                .get(header::COOKIE)
                .and_then(|value| value.to_str().ok());
            let now = time::OffsetDateTime::now_utc().unix_timestamp();
            if !state
                .unlock
                .opens(cookies, &public_host, page_id, &protection.version, now)
            {
                // A session that no longer opens the page is cleared, so a
                // password change does not leave a dead cookie in flight.
                let stale = unlock::carries_session(cookies, page_id).then_some(page_id);
                return unlock::challenge(&site, path, unlock::UnlockNotice::None, stale);
            }
            let response = unlock::unlocked(site.page(path).unwrap_or_default().to_owned());
            return analytics::record_html_view(&state, &resolved, path, analytics_visit, response)
                .await;
        }
    }

    let (content_type, body) = if path == "/assets/site.css" {
        ("text/css; charset=utf-8", site.css.clone())
    } else if let Some(page) = site.page(path) {
        ("text/html; charset=utf-8", page.to_owned())
    } else {
        tracing::debug!(host = %public_host, "no page at requested path");
        return not_found(site.not_found.clone());
    };

    // Strong ETag: bytes are a pure function of (publish, path).
    let etag = format!("\"{}:{path}\"", site.publish.as_str());
    if if_none_match_hits(req.headers().get(header::IF_NONE_MATCH), &etag) {
        let response = cacheable(StatusCode::NOT_MODIFIED, content_type, &etag, String::new());
        return analytics::record_html_view(&state, &resolved, path, analytics_visit, response)
            .await;
    }
    let response = cacheable(StatusCode::OK, content_type, &etag, body);
    analytics::record_html_view(&state, &resolved, path, analytics_visit, response).await
}

/// Serves one referenced image blob. Bytes are immutable per blob id
/// (content-addressed underneath), so the ETag is the id and clients may
/// cache for an hour; the CSP defangs SVG opened as a top-level document
/// (no scripts can run on the site's origin).
async fn serve_image(
    state: &Arc<AppState>,
    resolved: &alo_store::PublishedSite,
    blob_id: &str,
    req_headers: &axum::http::HeaderMap,
    site_not_found: String,
) -> Response {
    let etag = format!("\"img:{blob_id}\"");
    if if_none_match_hits(req_headers.get(header::IF_NONE_MATCH), &etag) {
        // The bytes are immutable per id — a matching validator never needs
        // the row read at all.
        return image_response(StatusCode::NOT_MODIFIED, None, &etag, Vec::new());
    }
    match state.store.published_image(resolved, blob_id).await {
        Ok(Some(image)) => image_response(
            StatusCode::OK,
            Some(image.content_type),
            &etag,
            image.bytes.to_vec(),
        ),
        Ok(None) => {
            // Referenced by the publish but not servable (blob gone, or a
            // non-image content type): the same themed 404 as a missing page.
            tracing::warn!(site = %resolved.site, "referenced image blob is not servable");
            not_found(site_not_found)
        }
        Err(error) => {
            tracing::error!(site = %resolved.site, %error, "image read failed");
            unavailable()
        }
    }
}

/// Serves one derivative of a referenced image: the frame and width the
/// publish's own `srcset` asked for.
///
/// The bytes are a pure function of (blob, crop, width) — all three in the
/// URL — so the path is its own validator and the same cache contract as the
/// original applies. The resize runs on the blocking pool: it is CPU-bound on
/// untrusted input, and a decoder that panics or hangs must cost one request,
/// not the runtime. Anything the pipeline declines (a vector, an animation, a
/// photo already narrower than the rung) serves the original bytes under the
/// derivative's own path, which is what the `srcset` fallback expects.
async fn serve_derivative(
    state: &Arc<AppState>,
    resolved: &alo_store::PublishedSite,
    key: &str,
    request: &crate::images::DerivativeRequest,
    req_headers: &axum::http::HeaderMap,
    site_not_found: String,
) -> Response {
    let etag = format!("\"img:{key}\"");
    if if_none_match_hits(req_headers.get(header::IF_NONE_MATCH), &etag) {
        return image_response(StatusCode::NOT_MODIFIED, None, &etag, Vec::new());
    }
    // The cache is keyed by the resolved site, not by the blob id alone: a
    // blob id is only unique inside its tenant, and a site belongs to exactly
    // one — so no host can ever read another tenant's pixels out of the map.
    let scope = resolved.site.as_str();
    if let Some(cached) = state.derivatives.get(scope, key) {
        return image_response(
            StatusCode::OK,
            Some(cached.content_type),
            &etag,
            cached.bytes.to_vec(),
        );
    }
    let source = match state
        .store
        .published_image(resolved, &request.blob_id)
        .await
    {
        Ok(Some(source)) => source,
        Ok(None) => {
            tracing::warn!(site = %resolved.site, "referenced image blob is not servable");
            return not_found(site_not_found);
        }
        Err(error) => {
            tracing::error!(site = %resolved.site, %error, "image read failed");
            return unavailable();
        }
    };
    let crop = request.crop;
    let width = request.width;
    let produced = {
        let source = source.clone();
        tokio::task::spawn_blocking(move || derivative::derive(&source, crop, width)).await
    };
    let served = match produced {
        Ok(Some(derived)) => cache::CachedImage {
            content_type: derived.content_type,
            bytes: derived.bytes,
        },
        Ok(None) => cache::CachedImage {
            content_type: source.content_type,
            bytes: source.bytes,
        },
        Err(error) => {
            // A panicking decoder is one bad image, not an outage: the
            // original bytes are still a correct answer for this path.
            tracing::error!(site = %resolved.site, %error, "image derivation task failed");
            cache::CachedImage {
                content_type: source.content_type,
                bytes: source.bytes,
            }
        }
    };
    let served = Arc::new(served);
    state.derivatives.put(scope, key, Arc::clone(&served));
    image_response(
        StatusCode::OK,
        Some(served.content_type),
        &etag,
        served.bytes.to_vec(),
    )
}

/// A 200/304 for an image: revalidatable for an hour, `nosniff`, and a CSP
/// that keeps an SVG document inert on the site's origin.
fn image_response(
    status: StatusCode,
    content_type: Option<&'static str>,
    etag: &str,
    body: Vec<u8>,
) -> Response {
    let mut response = (status, body).into_response();
    let headers = response.headers_mut();
    if let Some(value) = content_type.and_then(|ct| HeaderValue::from_str(ct).ok()) {
        headers.insert(header::CONTENT_TYPE, value);
    }
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=3600"),
    );
    if let Ok(value) = HeaderValue::from_str(etag) {
        headers.insert(header::ETAG, value);
    }
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static("default-src 'none'; style-src 'unsafe-inline'"),
    );
    response
}

/// Whether an `If-None-Match` value matches `etag` (list form and `*`).
fn if_none_match_hits(value: Option<&HeaderValue>, etag: &str) -> bool {
    value
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.split(',').any(|c| c.trim() == etag || c.trim() == "*"))
}

/// A 200/304 with the revalidation headers shared by pages and the stylesheet.
fn cacheable(status: StatusCode, content_type: &'static str, etag: &str, body: String) -> Response {
    let mut response = (status, body).into_response();
    let headers = response.headers_mut();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=60"),
    );
    if let Ok(value) = HeaderValue::from_str(etag) {
        headers.insert(header::ETAG, value);
    }
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    response
}

/// The method policy of every public path: read it, or — on a protected page
/// only — post its unlock form. `Allow` names the verbs that work everywhere,
/// so it says nothing about which pages carry a password.
fn method_not_allowed() -> Response {
    (
        StatusCode::METHOD_NOT_ALLOWED,
        [(header::ALLOW, HeaderValue::from_static("GET, HEAD"))],
        "method not allowed\n",
    )
        .into_response()
}

/// A 404 carrying the given document (generic or site-themed).
fn not_found(body: String) -> Response {
    (
        StatusCode::NOT_FOUND,
        [
            (
                header::CONTENT_TYPE,
                HeaderValue::from_static("text/html; charset=utf-8"),
            ),
            (header::CACHE_CONTROL, HeaderValue::from_static("no-cache")),
            (
                header::X_CONTENT_TYPE_OPTIONS,
                HeaderValue::from_static("nosniff"),
            ),
        ],
        body,
    )
        .into_response()
}

/// Dynamic public HTML (blog publication can change independently of the
/// site's page-snapshot publish id): always revalidate instead of handing a
/// stale entity tag to a browser.
fn dynamic_html(body: String) -> Response {
    dynamic(body, "text/html; charset=utf-8")
}

/// Dynamic RSS derived from independently changing published posts.
fn dynamic_rss(body: String) -> Response {
    dynamic(body, "application/rss+xml; charset=utf-8")
}

fn dynamic_xml(body: String) -> Response {
    dynamic(body, "application/xml; charset=utf-8")
}

fn dynamic_text(body: String) -> Response {
    dynamic(body, "text/plain; charset=utf-8")
}

fn dynamic(body: String, content_type: &'static str) -> Response {
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, HeaderValue::from_static(content_type)),
            (header::CACHE_CONTROL, HeaderValue::from_static("no-cache")),
            (
                header::X_CONTENT_TYPE_OPTIONS,
                HeaderValue::from_static("nosniff"),
            ),
        ],
        body,
    )
        .into_response()
}

/// The terse 503 for database trouble — nothing internal on the wire.
fn unavailable() -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        [
            (
                header::CONTENT_TYPE,
                HeaderValue::from_static("text/plain; charset=utf-8"),
            ),
            (header::CACHE_CONTROL, HeaderValue::from_static("no-store")),
            (header::RETRY_AFTER, HeaderValue::from_static("10")),
        ],
        "temporarily unavailable\n",
    )
        .into_response()
}
