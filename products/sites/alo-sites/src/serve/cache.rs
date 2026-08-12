//! The public service's in-memory caches: rendered sites, and the image
//! derivatives their pages reference.
//!
//! Sites are keyed by canonical public host and validated by publish id.
//! Every request still performs the one indexed resolver read — that is what
//! makes a republish (or unpublish) visible immediately — but rendering is
//! done once per publish, not per request: a cache entry is a hit only while
//! its publish id matches what the resolver just returned, so serving stale
//! content is impossible by construction.
//!
//! Derivatives are keyed by resolved site **and** derivative path, and need
//! no validity check at all: both halves of the key are immutable (a blob id
//! names fixed bytes, the path names the frame and the width), so a hit is
//! always the right answer. The site is in the key because a blob id is only
//! unique inside its tenant, and a site belongs to exactly one — two tenants
//! must never be able to read each other's pixels out of a shared map.

use std::collections::HashMap;
use std::sync::{Arc, PoisonError, RwLock};

use alo_store::SitePublishId;
use bytes::Bytes;

use super::rendered::RenderedSite;

/// Upper bound on cached sites; above it an arbitrary entry is evicted.
/// Rendering is cheap (a few ms) so eviction quality is not worth an LRU
/// until real traffic says otherwise.
const MAX_SITES: usize = 512;

/// The publish-keyed render cache. Lock poisoning is deliberately ignored:
/// a rendered site is plain data, valid regardless of another thread's
/// panic, and the public service must keep serving.
#[derive(Default)]
pub struct SiteCache {
    inner: RwLock<HashMap<String, Arc<RenderedSite>>>,
}

impl SiteCache {
    /// The cached render of `host`, only if it is exactly `publish`.
    pub fn get(&self, host: &str, publish: &SitePublishId) -> Option<Arc<RenderedSite>> {
        let map = self.inner.read().unwrap_or_else(PoisonError::into_inner);
        map.get(host)
            .filter(|site| site.publish == *publish)
            .cloned()
    }

    /// Stores a fresh render, replacing any earlier publish of the same
    /// host and evicting an arbitrary entry at the bound.
    pub fn put(&self, host: &str, site: Arc<RenderedSite>) {
        let mut map = self.inner.write().unwrap_or_else(PoisonError::into_inner);
        if !map.contains_key(host)
            && map.len() >= MAX_SITES
            && let Some(evict) = map.keys().next().cloned()
        {
            map.remove(&evict);
        }
        map.insert(host.to_owned(), site);
    }
}

/// Byte ceiling of the derivative cache. Resizing a photo costs tens of
/// milliseconds of CPU on untrusted input, so the answer is kept; a bound in
/// bytes (rather than entries) is what keeps a site of large images from
/// crowding out every other site on the process.
const MAX_DERIVATIVE_BYTES: usize = 64 * 1024 * 1024;

/// One servable answer for a derivative path: either the resized image or —
/// when the pipeline declined ([`super::derivative::derive`]) — the original
/// bytes under that path. Both are cached, so a source that cannot be resized
/// is not re-examined on every request.
#[derive(Debug, Clone)]
pub struct CachedImage {
    /// Content type of these bytes.
    pub content_type: &'static str,
    /// The encoded image.
    pub bytes: Bytes,
}

#[derive(Default)]
struct DerivativeEntries {
    by_key: HashMap<String, Arc<CachedImage>>,
    bytes: usize,
}

/// Derivatives by site + derivative path. Lock poisoning is ignored for the
/// same reason as above: the map is plain data, and the public service keeps
/// serving.
#[derive(Default)]
pub struct DerivativeCache {
    inner: RwLock<DerivativeEntries>,
}

impl DerivativeCache {
    /// The cached answer for one site's derivative path.
    pub fn get(&self, site: &str, key: &str) -> Option<Arc<CachedImage>> {
        let entries = self.inner.read().unwrap_or_else(PoisonError::into_inner);
        entries.by_key.get(&Self::index(site, key)).cloned()
    }

    /// Stores an answer, evicting arbitrary entries until it fits. Eviction
    /// quality is not worth an LRU here: everything in this map can be
    /// recomputed from bytes we still hold, and a wrong eviction costs one
    /// resize.
    pub fn put(&self, site: &str, key: &str, image: Arc<CachedImage>) {
        let index = Self::index(site, key);
        let size = image.bytes.len();
        if size > MAX_DERIVATIVE_BYTES {
            return;
        }
        let mut entries = self.inner.write().unwrap_or_else(PoisonError::into_inner);
        if let Some(previous) = entries.by_key.remove(&index) {
            entries.bytes = entries.bytes.saturating_sub(previous.bytes.len());
        }
        while entries.bytes + size > MAX_DERIVATIVE_BYTES {
            let Some(evict) = entries.by_key.keys().next().cloned() else {
                break;
            };
            if let Some(removed) = entries.by_key.remove(&evict) {
                entries.bytes = entries.bytes.saturating_sub(removed.bytes.len());
            }
        }
        entries.bytes += size;
        entries.by_key.insert(index, image);
    }

    /// Site-scoped map key. The separator is `\u{0}`, which appears in neither
    /// a site id nor a derivative path, so no pair of (site, path) values can
    /// ever collide into one entry.
    fn index(site: &str, key: &str) -> String {
        format!("{site}\u{0}{key}")
    }
}
