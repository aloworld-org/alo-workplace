//! The responsive-image contract: which derivatives a published page offers,
//! what their URLs spell, and how one of those URLs is read back.
//!
//! Both directions live in one file on purpose. The renderer writes these
//! paths into `srcset`, [`crate::serve::rendered::RenderedSite`] collects the
//! same paths into the set the public service will serve, and the service
//! parses an incoming path back into the frame and width to produce — three
//! readers of one grammar, which must never drift.
//!
//! ## The grammar
//!
//! ```text
//! /assets/img/<blob_id>                       the original bytes, unframed
//! /assets/img/<blob_id>/w960                  the whole image at 960px wide
//! /assets/img/<blob_id>/c1000-0-8000-10000/w960
//!                                             a crop of it at 960px wide
//! ```
//!
//! The crop segment carries the stored [`ImageCrop`] verbatim in basis points
//! (`c<x>-<y>-<width>-<height>`), so a derivative URL is a pure function of
//! the published section — and therefore stable across republishes that did
//! not touch the photo, and cacheable forever by any proxy in front of us.
//!
//! ## Why a fixed ladder, and why membership
//!
//! Nothing in a derivative URL is chosen by the visitor. The width must be one
//! of [`DERIVATIVE_WIDTHS`], and the whole path must be one the served publish
//! itself references — the service checks membership before it decodes a
//! single byte. An image pipeline that resizes to whatever a query string asks
//! for is a CPU amplifier pointed at its own origin; this one can only ever be
//! asked for the handful of derivatives the site's own HTML names.

use alo_store::site_model::{IMAGE_GEOMETRY_FULL_BP, ImageCrop, MIN_CROP_EXTENT_BP, SiteImage};

/// The widths a published page offers, narrowest first. Three rungs cover a
/// phone, a tablet or half-width column, and a full-width banner on a
/// desktop (or a phone at 2× device-pixel-ratio); a longer ladder multiplies
/// the derivatives to generate and cache for a difference no one sees.
pub const DERIVATIVE_WIDTHS: [u32; 3] = [480, 960, 1440];

/// The path prefix every image reference of a published page starts with.
pub const IMAGE_PATH_PREFIX: &str = "/assets/img/";

/// Where an image sits in the layout, which is the only honest source of a
/// `sizes` attribute: the browser picks a candidate before it has any CSS, so
/// the document has to tell it how wide the slot will be. The values mirror
/// `crate::stylesheet` (content column 70rem, two-column split at 48rem,
/// grids of ~16rem cards).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageSlot {
    /// A full-content-width image: the hero's artwork.
    Banner,
    /// One column of the two-column `text_image` split.
    Half,
    /// One card of a responsive grid: a gallery tile, a team portrait.
    Card,
}

impl ImageSlot {
    /// The `sizes` attribute for this slot.
    #[must_use]
    pub const fn sizes(self) -> &'static str {
        match self {
            // `main > section` is 70rem wide with 1.25rem of padding a side.
            ImageSlot::Banner => "(min-width: 70rem) 67.5rem, 100vw",
            // Two equal columns from 48rem up, inside that same 67.5rem.
            ImageSlot::Half => "(min-width: 70rem) 33rem, (min-width: 48rem) 50vw, 100vw",
            // `repeat(auto-fit, minmax(15rem, 1fr))`: at most four across.
            ImageSlot::Card => "(min-width: 70rem) 17rem, (min-width: 48rem) 33vw, 100vw",
        }
    }

    /// Whether an image in this slot may be deferred until it is near the
    /// viewport. Grid cards are below the fold by construction; a hero or a
    /// `text_image` illustration can be the largest element painted, and
    /// deferring that would make the page feel slower, not faster.
    #[must_use]
    pub const fn lazy(self) -> bool {
        matches!(self, ImageSlot::Card)
    }
}

/// One entry of an image's `srcset`: the path to fetch and the width
/// descriptor that path claims.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    /// Site-relative path, unescaped (the renderer escapes it).
    pub path: String,
    /// The `w` descriptor, in CSS pixels.
    pub width: u32,
}

/// The derivative candidates of one published image, narrowest first.
///
/// Every image gets the full ladder: the renderer cannot know how many pixels
/// the source actually has (dimensions are not part of the stored model, and
/// rendering never touches blob bytes), so the *service* is what refuses to
/// upscale — a rung wider than the source serves the source. The cost of the
/// widest rung on a small photo is therefore a slightly optimistic descriptor,
/// never a blurry enlargement.
#[must_use]
pub fn candidates(image: &SiteImage) -> Vec<Candidate> {
    let crop = image.crop_or_full();
    DERIVATIVE_WIDTHS
        .iter()
        .map(|&width| Candidate {
            path: variant_path(image.blob_id.as_str(), crop, width),
            width,
        })
        .collect()
}

/// The `src` an `<img>` falls back to when `srcset` is not honored.
///
/// An uncropped image keeps the original path — the bytes the tenant
/// uploaded, and the same URL `og:image` and the theme use. A **cropped**
/// image cannot: the original is the unframed photo, and a fallback that
/// ignores the frame would show exactly what the owner cropped away. It falls
/// back to the widest derivative instead, which is always framed.
#[must_use]
pub fn fallback_path(image: &SiteImage) -> String {
    match image.crop {
        None => format!("{IMAGE_PATH_PREFIX}{}", image.blob_id.as_str()),
        Some(crop) => {
            let widest = *DERIVATIVE_WIDTHS.last().unwrap_or(&960);
            variant_path(image.blob_id.as_str(), crop, widest)
        }
    }
}

/// The variant keys of one image — every derivative path it may be asked for,
/// **relative to [`IMAGE_PATH_PREFIX`]**. This is what the servable set holds
/// and what an incoming request is matched against.
#[must_use]
pub fn variant_keys(image: &SiteImage) -> Vec<String> {
    let crop = image.crop_or_full();
    DERIVATIVE_WIDTHS
        .iter()
        .map(|&width| variant_key(image.blob_id.as_str(), crop, width))
        .collect()
}

/// One derivative's site-relative path.
#[must_use]
pub fn variant_path(blob_id: &str, crop: ImageCrop, width: u32) -> String {
    format!("{IMAGE_PATH_PREFIX}{}", variant_key(blob_id, crop, width))
}

/// One derivative's key: the part after [`IMAGE_PATH_PREFIX`].
fn variant_key(blob_id: &str, crop: ImageCrop, width: u32) -> String {
    if crop == ImageCrop::full() {
        format!("{blob_id}/w{width}")
    } else {
        format!(
            "{blob_id}/c{}-{}-{}-{}/w{width}",
            crop.x_bp, crop.y_bp, crop.width_bp, crop.height_bp
        )
    }
}

/// What a derivative request asks for, once read back out of its path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DerivativeRequest {
    /// The tenant blob the pixels come from.
    pub blob_id: String,
    /// The rectangle of it to show.
    pub crop: ImageCrop,
    /// The width to render it at, always one of [`DERIVATIVE_WIDTHS`].
    pub width: u32,
}

/// Reads a variant key (the part after [`IMAGE_PATH_PREFIX`]) back into what
/// it asks for, or `None` when it is not one of ours.
///
/// The service checks membership in the publish's own set first, so this
/// parse can never see an attacker-chosen string in practice — it stays
/// strict anyway (fixed segment count, no empty blob id, no path traversal,
/// digits only, a width from the ladder, a rectangle inside the image), so
/// that the two gates are independent.
#[must_use]
pub fn parse_variant(key: &str) -> Option<DerivativeRequest> {
    let mut segments = key.split('/');
    let blob_id = segments.next()?;
    let second = segments.next()?;
    let (crop, width_segment) = match segments.next() {
        Some(third) => (parse_crop(second)?, third),
        None => (ImageCrop::full(), second),
    };
    if segments.next().is_some() {
        return None;
    }
    if blob_id.is_empty() || !blob_id.bytes().all(is_token_byte) {
        return None;
    }
    // Digits only, and no leading zero: one derivative has exactly one
    // spelling, so it has exactly one cache entry.
    let digits = width_segment.strip_prefix('w')?;
    if digits.starts_with('0') || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let width: u32 = digits.parse().ok()?;
    if !DERIVATIVE_WIDTHS.contains(&width) {
        return None;
    }
    Some(DerivativeRequest {
        blob_id: blob_id.to_owned(),
        crop,
        width,
    })
}

/// `c<x>-<y>-<width>-<height>`, in basis points, inside the image.
fn parse_crop(segment: &str) -> Option<ImageCrop> {
    let mut parts = segment.strip_prefix('c')?.split('-');
    let mut next = || -> Option<u16> {
        let part = parts.next()?;
        // Digits only (`parse` would accept `+1`), and canonically spelled:
        // no leading zero, so `c0-0-…` is the only way to write that edge.
        if part.is_empty()
            || !part.bytes().all(|b| b.is_ascii_digit())
            || (part.len() > 1 && part.starts_with('0'))
        {
            return None;
        }
        part.parse().ok()
    };
    let crop = ImageCrop {
        x_bp: next()?,
        y_bp: next()?,
        width_bp: next()?,
        height_bp: next()?,
    };
    if parts.next().is_some() {
        return None;
    }
    let full = u32::from(IMAGE_GEOMETRY_FULL_BP);
    // The same rectangle rules the write gate applies (`site_model`): inside
    // the image, and never the degenerate sliver that would ask the pipeline
    // to blow a handful of pixels up to a full-width photo.
    if u32::from(crop.x_bp) + u32::from(crop.width_bp) > full
        || u32::from(crop.y_bp) + u32::from(crop.height_bp) > full
        || crop.width_bp < MIN_CROP_EXTENT_BP
        || crop.height_bp < MIN_CROP_EXTENT_BP
    {
        return None;
    }
    Some(crop)
}

/// The blob-id charset (`alo_store`'s id tokens): letters, digits, `-`, `_`.
const fn is_token_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'-' || b == b'_'
}

#[cfg(test)]
mod tests {
    use super::*;
    use alo_store::id::BlobId;

    fn image(crop: Option<ImageCrop>) -> SiteImage {
        SiteImage {
            crop,
            ..SiteImage::new(BlobId::new("9hK3vQ2mR8pT1xWz4bC5dg"), "A drum roaster")
        }
    }

    #[test]
    fn every_rendered_candidate_parses_back_to_what_it_asked_for() {
        let crop = ImageCrop {
            x_bp: 1000,
            y_bp: 0,
            width_bp: 8000,
            height_bp: 10000,
        };
        for framed in [image(None), image(Some(crop))] {
            let expected = framed.crop_or_full();
            for (candidate, key) in candidates(&framed).into_iter().zip(variant_keys(&framed)) {
                assert_eq!(candidate.path, format!("{IMAGE_PATH_PREFIX}{key}"));
                let Some(parsed) = parse_variant(&key) else {
                    panic!("a key this crate renders must parse back: {key}");
                };
                assert_eq!(parsed.blob_id, framed.blob_id.as_str());
                assert_eq!(parsed.crop, expected);
                assert_eq!(parsed.width, candidate.width);
            }
        }
    }

    #[test]
    fn the_fallback_is_the_original_only_while_the_whole_photo_is_shown() {
        assert_eq!(
            fallback_path(&image(None)),
            "/assets/img/9hK3vQ2mR8pT1xWz4bC5dg"
        );
        assert_eq!(
            fallback_path(&image(Some(ImageCrop {
                x_bp: 1000,
                y_bp: 0,
                width_bp: 8000,
                height_bp: 10000,
            }))),
            "/assets/img/9hK3vQ2mR8pT1xWz4bC5dg/c1000-0-8000-10000/w1440"
        );
    }

    #[test]
    fn a_full_crop_spells_the_same_key_as_no_crop_at_all() {
        assert_eq!(
            variant_keys(&image(Some(ImageCrop::full()))),
            variant_keys(&image(None))
        );
    }

    #[test]
    fn nothing_outside_the_grammar_parses() {
        for key in [
            "",
            "blob",
            "blob/w481",                    // not a rung of the ladder
            "blob/w0480",                   // not the canonical spelling
            "blob/W960",                    // case matters
            "blob/w-960",                   // no negatives
            "blob/w960/w960",               // one width
            "blob/c0-0-10000-10000",        // a crop is not a derivative
            "blob/x0-0-1-1/w960",           // unknown segment
            "blob/c0-0-10000/w960",         // three parts
            "blob/c0-0-10000-10000-1/w960", // five parts
            "blob/c0-0-0-10000/w960",       // zero extent
            "blob/c0-0-99-10000/w960",      // below the schema's 1% floor
            "blob/c5000-0-6000-10000/w960", // leaves the image
            "blob/c-1-0-100-100/w960",      // no negatives
            "blob/c00-0-10000-10000/w960",  // one spelling per rectangle
            "blob/c+1-0-100-100/w960",      // digits only
            "../../etc/passwd/w960",
            "blob/../w960",
            "bl ob/w960",
            "blob%2fw960",
        ] {
            assert!(parse_variant(key).is_none(), "parsed: {key}");
        }
    }

    #[test]
    fn a_crop_that_is_not_the_whole_image_survives_the_round_trip_exactly() {
        let crop = ImageCrop {
            x_bp: 1,
            y_bp: 9900,
            width_bp: 100,
            height_bp: 100,
        };
        let key = variant_key("blob", crop, 480);
        assert_eq!(key, "blob/c1-9900-100-100/w480");
        assert_eq!(parse_variant(&key).map(|parsed| parsed.crop), Some(crop));
    }
}
