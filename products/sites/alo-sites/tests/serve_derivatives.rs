//! The responsive-image half of the public serving contract, in process:
//! real photos through the real store into the compose Postgres, real
//! requests through the real router.
//!
//! What it pins: a published page's `srcset` paths are exactly the ones the
//! service will serve, they come back at the width they claim, they carry the
//! cache contract, and **nothing else does** — not another width, not another
//! frame, not another site's photo.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::io::Cursor;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use axum::response::Response;
use bytes::Bytes;
use image::{DynamicImage, ImageFormat, RgbImage};
use serde_json::json;
use tower::ServiceExt;

use alo_sites::serve::{AppState, app};
use alo_store::{AccountStore, BlobId, BlobStore, SiteId, SitePublicStore, Store};

const APEX: &str = "sites.test";

/// The database this suite runs against.
///
/// Delegates to `alo_test_db`, which refuses the database the product
/// runs on: suites create and drop their own, they never write into `alo`.
fn database_url() -> String {
    alo_test_db::url()
}

async fn harness() -> (Store, Arc<AppState>) {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(6)
        .connect(&database_url())
        .await
        .expect("connect to test postgres (is compose up? is DATABASE_URL set?)");
    let blobs = BlobStore::in_memory(25 * 1024 * 1024);
    let store = Store::new(pool.clone(), blobs.clone());
    store.migrate().await.expect("run migrations");
    let state = AppState::new(
        SitePublicStore::new(pool, blobs),
        APEX.to_owned(),
        b"serve-derivative-tests-secret",
    );
    (store, state)
}

async fn fresh_account(store: &Store, tag: &str) -> AccountStore {
    let tenant = store.create_tenant(&format!("t-{tag}")).await.unwrap();
    let user = store
        .for_tenant(tenant.clone())
        .create_user(&format!("{tag}@derivatives.test"))
        .await
        .unwrap();
    store.for_account(tenant, user)
}

fn unique(tag: &str) -> String {
    format!(
        "{tag}-{}x",
        SiteId::generate().as_str().to_lowercase().replace('_', "-")
    )
}

async fn send(state: &Arc<AppState>, host: &str, path: &str) -> Response {
    let request = Request::builder()
        .uri(path)
        .header(header::HOST, host)
        .body(Body::empty())
        .unwrap();
    app(Arc::clone(state)).oneshot(request).await.unwrap()
}

async fn body_bytes(response: Response) -> Bytes {
    axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap()
}

fn header_str<'a>(response: &'a Response, name: &header::HeaderName) -> &'a str {
    response
        .headers()
        .get(name)
        .map(|value| value.to_str().unwrap())
        .unwrap_or("")
}

/// A photo with a hard vertical split: red on the left, blue on the right, so
/// a served crop can be told from a served whole.
fn photo_bytes(width: u32, height: u32) -> Bytes {
    let mut buffer = RgbImage::new(width, height);
    for (x, y, pixel) in buffer.enumerate_pixels_mut() {
        let shade = u8::try_from((x * 255 / width + y * 96 / height) % 256).unwrap();
        *pixel = if x * 2 < width {
            image::Rgb([shade.max(120), 10, 10])
        } else {
            image::Rgb([10, 10, shade.max(120)])
        };
    }
    let mut out = Vec::new();
    DynamicImage::ImageRgb8(buffer)
        .write_to(&mut Cursor::new(&mut out), ImageFormat::Jpeg)
        .unwrap();
    Bytes::from(out)
}

fn decode(bytes: &[u8]) -> DynamicImage {
    image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .unwrap()
        .decode()
        .unwrap()
}

/// The crop the fixture site puts on its gallery photo: the right half.
const RIGHT_HALF: &str = "c5000-0-5000-10000";

/// Publishes a site whose home page shows `photo` twice: whole in the hero,
/// right-half-cropped in the gallery.
async fn publish_framed_site(acc: &AccountStore, name: &str, sub: &str, photo: &BlobId) -> SiteId {
    let site = acc.create_site(name, sub).await.unwrap();
    let home = acc.create_site_page(&site, "Home", "", true).await.unwrap();
    acc.set_page_sections(
        &site,
        &home,
        json!({
            "schema_version": 1,
            "sections": [
                {"type": "hero", "heading": "Roasted on the harbour",
                 "image": {"blob_id": photo.as_str(), "alt": "The drum"}},
                {"type": "gallery", "images": [{
                    "blob_id": photo.as_str(),
                    "alt": "The cupping table",
                    "crop": {"x_bp": 5000, "y_bp": 0, "width_bp": 5000, "height_bp": 10000}
                }]}
            ]
        }),
    )
    .await
    .unwrap();
    acc.publish_site(&site).await.unwrap();
    site
}

#[tokio::test]
async fn the_ladder_a_page_offers_is_exactly_what_the_service_serves() {
    let (store, state) = harness().await;
    let acc = fresh_account(&store, "deriv").await;
    let original = photo_bytes(2400, 1600);
    let photo = acc
        .put_blob(original.clone(), Some("image/jpeg"))
        .await
        .unwrap();
    let sub = unique("deriv");
    let host = format!("{sub}.{APEX}");
    publish_framed_site(&acc, "Framed Co", &sub, &photo).await;

    let html =
        String::from_utf8(body_bytes(send(&state, &host, "/").await).await.to_vec()).unwrap();
    for width in [480, 960, 1440] {
        assert!(
            html.contains(&format!("/assets/img/{}/w{width} {width}w", photo.as_str())),
            "the page offers every rung: {html}"
        );
    }

    // Every rung the page offers comes back at exactly that width, smaller
    // than the source, with the image cache contract.
    for width in [480u32, 960, 1440] {
        let path = format!("/assets/img/{}/w{width}", photo.as_str());
        let response = send(&state, &host, &path).await;
        assert_eq!(response.status(), StatusCode::OK, "{path}");
        assert_eq!(header_str(&response, &header::CONTENT_TYPE), "image/jpeg");
        assert_eq!(
            header_str(&response, &header::CACHE_CONTROL),
            "public, max-age=3600"
        );
        assert_eq!(
            header_str(&response, &header::ETAG),
            format!("\"img:{}/w{width}\"", photo.as_str())
        );
        assert_eq!(
            header_str(&response, &header::X_CONTENT_TYPE_OPTIONS),
            "nosniff"
        );
        assert_eq!(
            header_str(&response, &header::CONTENT_SECURITY_POLICY),
            "default-src 'none'; style-src 'unsafe-inline'"
        );
        let bytes = body_bytes(response).await;
        assert_eq!(decode(&bytes).width(), width, "{path}");
        assert!(
            bytes.len() < original.len(),
            "{path}: {} bytes from a {}-byte photo",
            bytes.len(),
            original.len()
        );
    }

    // The original path still serves the uploaded bytes untouched — the
    // `srcset` fallback, and what `og:image` points crawlers at.
    let full = send(&state, &host, &format!("/assets/img/{}", photo.as_str())).await;
    assert_eq!(full.status(), StatusCode::OK);
    assert_eq!(body_bytes(full).await, original);
}

#[tokio::test]
async fn a_derivative_is_cached_and_revalidates_without_a_body() {
    let (store, state) = harness().await;
    let acc = fresh_account(&store, "cache").await;
    let photo = acc
        .put_blob(photo_bytes(1800, 1200), Some("image/jpeg"))
        .await
        .unwrap();
    let sub = unique("cache");
    let host = format!("{sub}.{APEX}");
    publish_framed_site(&acc, "Cache Co", &sub, &photo).await;
    let path = format!("/assets/img/{}/w960", photo.as_str());

    let first = send(&state, &host, &path).await;
    let etag = header_str(&first, &header::ETAG).to_owned();
    let first_bytes = body_bytes(first).await;

    // The second read comes out of the derivative cache: same bytes, byte for
    // byte, without re-deciding anything.
    let second = send(&state, &host, &path).await;
    assert_eq!(second.status(), StatusCode::OK);
    assert_eq!(body_bytes(second).await, first_bytes);

    // The path is its own validator: a matching If-None-Match is a 304 with
    // no body and no decode.
    let request = Request::builder()
        .uri(&path)
        .header(header::HOST, &host)
        .header(header::IF_NONE_MATCH, &etag)
        .body(Body::empty())
        .unwrap();
    let revalidated = app(Arc::clone(&state)).oneshot(request).await.unwrap();
    assert_eq!(revalidated.status(), StatusCode::NOT_MODIFIED);
    assert_eq!(
        header_str(&revalidated, &header::CACHE_CONTROL),
        "public, max-age=3600"
    );
    assert!(body_bytes(revalidated).await.is_empty());
}

#[tokio::test]
async fn the_served_crop_is_the_rectangle_the_page_published() {
    let (store, state) = harness().await;
    let acc = fresh_account(&store, "crop").await;
    let photo = acc
        .put_blob(photo_bytes(2000, 1000), Some("image/jpeg"))
        .await
        .unwrap();
    let sub = unique("crop");
    let host = format!("{sub}.{APEX}");
    publish_framed_site(&acc, "Crop Co", &sub, &photo).await;

    let html =
        String::from_utf8(body_bytes(send(&state, &host, "/").await).await.to_vec()).unwrap();
    assert!(
        html.contains(&format!(
            "src=\"/assets/img/{}/{RIGHT_HALF}/w1440\"",
            photo.as_str()
        )),
        "a cropped image falls back to its own frame: {html}"
    );

    let path = format!("/assets/img/{}/{RIGHT_HALF}/w960", photo.as_str());
    let response = send(&state, &host, &path).await;
    assert_eq!(response.status(), StatusCode::OK);
    let image = decode(&body_bytes(response).await).to_rgb8();
    assert_eq!(image.width(), 960);
    assert_eq!(image.height(), 960, "the right half of 2000×1000 is square");
    for x in [10, 480, 940] {
        let pixel = image.get_pixel(x, 500);
        assert!(
            pixel[2] > pixel[0],
            "the frame is the blue right half; pixel at {x} is {pixel:?}"
        );
    }

    // The whole photo, at the same rung, still has its red left half.
    let whole = send(
        &state,
        &host,
        &format!("/assets/img/{}/w960", photo.as_str()),
    )
    .await;
    let whole_image = decode(&body_bytes(whole).await).to_rgb8();
    let left = whole_image.get_pixel(10, 240);
    assert!(left[0] > left[2], "the unframed rung is not the crop");
}

#[tokio::test]
async fn only_the_derivatives_the_publish_references_exist() {
    let (store, state) = harness().await;
    let acc = fresh_account(&store, "gate").await;
    let other = fresh_account(&store, "gate-other").await;
    let photo = acc
        .put_blob(photo_bytes(1600, 1200), Some("image/jpeg"))
        .await
        .unwrap();
    let unreferenced = acc
        .put_blob(photo_bytes(800, 600), Some("image/jpeg"))
        .await
        .unwrap();
    let foreign = other
        .put_blob(photo_bytes(800, 600), Some("image/jpeg"))
        .await
        .unwrap();
    let sub = unique("gate");
    let host = format!("{sub}.{APEX}");
    publish_framed_site(&acc, "Gate Co", &sub, &photo).await;

    let blob = photo.as_str();
    for path in [
        // A width off the ladder: the pipeline is not a resize service.
        format!("/assets/img/{blob}/w800"),
        format!("/assets/img/{blob}/w4000"),
        format!("/assets/img/{blob}/w0480"),
        // A frame this publish never showed.
        format!("/assets/img/{blob}/c0-0-2000-2000/w480"),
        // The hero's own image at a crop only the gallery uses is fine —
        // but a crop nobody uses is not.
        format!("/assets/img/{blob}/c1-1-9000-9000/w960"),
        // Nonsense in the grammar.
        format!("/assets/img/{blob}/x480"),
        format!("/assets/img/{blob}/w480/w480"),
        format!("/assets/img/{blob}//w480"),
        // Blobs the page does not show, and another tenant's blob entirely.
        format!("/assets/img/{}/w480", unreferenced.as_str()),
        format!("/assets/img/{}/w480", foreign.as_str()),
        format!("/assets/img/{}/{RIGHT_HALF}/w480", foreign.as_str()),
        "/assets/img/no-such-blob/w480".to_owned(),
        "/assets/img/../../etc/passwd/w480".to_owned(),
    ] {
        let response = send(&state, &host, &path).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path}");
        let body = String::from_utf8(body_bytes(response).await.to_vec()).unwrap();
        assert!(body.contains("Gate Co"), "stays in the brand: {path}");
    }
}

#[tokio::test]
async fn one_hosts_derivatives_are_never_reachable_from_another() {
    let (store, state) = harness().await;
    let a = fresh_account(&store, "iso-a").await;
    let b = fresh_account(&store, "iso-b").await;
    let photo_a = a
        .put_blob(photo_bytes(1600, 1200), Some("image/jpeg"))
        .await
        .unwrap();
    let photo_b = b
        .put_blob(photo_bytes(1600, 1200), Some("image/jpeg"))
        .await
        .unwrap();
    let sub_a = unique("iso-a");
    let sub_b = unique("iso-b");
    publish_framed_site(&a, "Alpha", &sub_a, &photo_a).await;
    publish_framed_site(&b, "Beta", &sub_b, &photo_b).await;
    let host_a = format!("{sub_a}.{APEX}");
    let host_b = format!("{sub_b}.{APEX}");

    // Each host serves its own photo's derivative...
    for (host, photo) in [(&host_a, &photo_a), (&host_b, &photo_b)] {
        let path = format!("/assets/img/{}/w480", photo.as_str());
        assert_eq!(send(&state, host, &path).await.status(), StatusCode::OK);
    }
    // ...and neither can name the other's, at any rung or frame, even though
    // both pages reference a derivative of the same shape.
    for (host, photo) in [(&host_a, &photo_b), (&host_b, &photo_a)] {
        for suffix in ["w480", "w960", "w1440", "c5000-0-5000-10000/w480"] {
            let path = format!("/assets/img/{}/{suffix}", photo.as_str());
            let response = send(&state, host, &path).await;
            assert_eq!(response.status(), StatusCode::NOT_FOUND, "{host} {path}");
        }
    }
}

#[tokio::test]
async fn an_image_that_cannot_be_resized_serves_its_original_bytes_under_the_derivative_path() {
    let (store, state) = harness().await;
    let acc = fresh_account(&store, "vector").await;
    // An SVG has no raster derivative; the ladder still renders (the document
    // cannot know a blob's format), so every rung must answer with the file.
    let svg = Bytes::from_static(
        b"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 10 10\"><rect width=\"10\" height=\"10\"/></svg>",
    );
    let logo = acc
        .put_blob(svg.clone(), Some("image/svg+xml"))
        .await
        .unwrap();
    let sub = unique("vector");
    let host = format!("{sub}.{APEX}");
    publish_framed_site(&acc, "Vector Co", &sub, &logo).await;

    let response = send(
        &state,
        &host,
        &format!("/assets/img/{}/w480", logo.as_str()),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        header_str(&response, &header::CONTENT_TYPE),
        "image/svg+xml"
    );
    // The CSP that keeps an SVG document inert applies to the derivative path
    // exactly as it does to the original.
    assert_eq!(
        header_str(&response, &header::CONTENT_SECURITY_POLICY),
        "default-src 'none'; style-src 'unsafe-inline'"
    );
    assert_eq!(body_bytes(response).await, svg);
}
