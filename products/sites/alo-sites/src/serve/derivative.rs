//! Turning one stored photo into the width a visitor's screen actually needs:
//! decode → crop → resize → re-encode, with every step bounded.
//!
//! This is the only place in the sites family that looks at image *pixels*.
//! Everything above it deals in blob ids and basis points; the crop rectangle
//! S2.07a stores becomes visible here, and nowhere else.
//!
//! ## The safety rules, and why each one exists
//!
//! - **Nothing is decoded that the publish did not ask for.** The caller
//!   checks the requested derivative against the served publish's own set
//!   ([`crate::images`]) first, so the work is bounded by what the site's HTML
//!   already promises — not by what a request can spell.
//! - **Decoding is bounded** ([`Limits`]): a source over
//!   [`MAX_SOURCE_BYTES`] is not decoded at all, and the decoder refuses an
//!   image over [`MAX_SOURCE_PIXELS`] or [`MAX_ALLOC_BYTES`]. A 40-kilobyte
//!   PNG can declare a 40 000 × 40 000 canvas; without a limit that is six
//!   gigabytes of allocation from one GET.
//! - **Only raster sources this build can decode produce a derivative.** SVG,
//!   AVIF, ICO and GIF (animation would be flattened to its first frame)
//!   return [`None`], and the caller serves the original bytes — a smaller
//!   answer than a wrong one.
//! - **Never upscale.** A rung wider than the source serves the source. An
//!   enlarged photo is bytes spent making an image worse.
//! - **Never grow the payload.** A derivative that is not smaller than the
//!   source is dropped — unless the frame differs from the source, where the
//!   point was the crop and correctness outranks bytes.
//! - **Encoding is JPEG or PNG, chosen by transparency**, not by what arrived:
//!   the format of the source is not a promise, and the served content type is
//!   whatever came out.
//!
//! Decoding is CPU-bound and takes untrusted input, so the caller runs
//! [`derive`] on the blocking pool: a slow decode cannot stall the runtime,
//! and a decoder panic lands as a join error the request turns into the
//! original bytes rather than a dead process.

use std::io::Cursor;

use alo_store::SiteImageData;
use alo_store::site_model::{IMAGE_GEOMETRY_FULL_BP, ImageCrop};
use bytes::Bytes;
use image::{DynamicImage, ImageEncoder, ImageFormat, ImageReader, Limits};

/// The largest source this pipeline will decode. Above it the original is
/// served as-is: a photo that big is already the tenant's own problem, and
/// making it the CPU's problem on every request would be ours.
pub const MAX_SOURCE_BYTES: usize = 16 * 1024 * 1024;
/// The largest source canvas, in pixels. Comfortably above a 100-megapixel
/// camera; far below what a crafted header can claim.
pub const MAX_SOURCE_PIXELS: u64 = 120_000_000;
/// The decoder's allocation ceiling for one image.
pub const MAX_ALLOC_BYTES: u64 = 512 * 1024 * 1024;
/// JPEG quality of a derivative. 82 is the usual knee of the curve: visually
/// indistinguishable from 95 at a third of the bytes.
const JPEG_QUALITY: u8 = 82;

/// One rendered derivative, ready to serve.
#[derive(Debug, Clone)]
pub struct Derivative {
    /// The content type of these bytes (which is **not** necessarily the
    /// source's — see the module note).
    pub content_type: &'static str,
    /// The encoded image.
    pub bytes: Bytes,
}

/// Renders `source` framed to `crop` at `width` pixels wide.
///
/// `None` means "serve the original bytes instead", and is the answer for
/// every case where a derivative would be absent, larger, or worse: a source
/// this build cannot decode, one too large to decode safely, a decode
/// failure, an unframed photo already narrower than the rung, or an encode
/// that came out no smaller than what it started from.
///
/// Blocking and CPU-bound: call it on the blocking pool.
#[must_use]
pub fn derive(source: &SiteImageData, crop: ImageCrop, width: u32) -> Option<Derivative> {
    if source.bytes.len() > MAX_SOURCE_BYTES {
        tracing::debug!(
            bytes = source.bytes.len(),
            "source too large to derive from"
        );
        return None;
    }
    let format = decodable_format(source.content_type)?;
    let decoded = decode(&source.bytes, format)?;

    let full_frame = crop == ImageCrop::full();
    let framed = apply_crop(decoded, crop);
    // Never upscale: the rung is a ceiling, not a target.
    let target = width.min(framed.width());
    if target == 0 {
        return None;
    }
    if full_frame && framed.width() <= width {
        // Nothing to crop and nothing to shrink — the original is the answer,
        // and re-encoding it would only lose a generation of quality.
        return None;
    }
    let height = scaled_height(framed.width(), framed.height(), target)?;
    let resized = framed.resize_exact(target, height, image::imageops::FilterType::Lanczos3);
    let derivative = encode(&resized)?;

    // A derivative that is not smaller has no reason to exist — except when
    // it is the only thing carrying the owner's frame.
    if full_frame && derivative.bytes.len() >= source.bytes.len() {
        tracing::debug!("derivative was not smaller than its source; serving the original");
        return None;
    }
    Some(derivative)
}

/// The decoder for a stored content type, or `None` when this build does not
/// resize that kind of image at all.
fn decodable_format(content_type: &str) -> Option<ImageFormat> {
    match content_type {
        "image/jpeg" => Some(ImageFormat::Jpeg),
        "image/png" => Some(ImageFormat::Png),
        "image/webp" => Some(ImageFormat::WebP),
        // Vector (SVG), animated (GIF), and formats no codec is compiled in
        // for (AVIF, ICO) are served as they were uploaded.
        _ => None,
    }
}

/// Decodes under [`Limits`]. A malformed or hostile image is a `None`, never
/// a panic path the caller has to reason about.
fn decode(bytes: &[u8], format: ImageFormat) -> Option<DynamicImage> {
    // Header first: dimensions are what a decompression bomb lies with, and
    // reading them costs nothing.
    let (width, height) = ImageReader::with_format(Cursor::new(bytes), format)
        .into_dimensions()
        .ok()?;
    if u64::from(width) * u64::from(height) > MAX_SOURCE_PIXELS {
        tracing::warn!(width, height, "refusing to decode an oversized image");
        return None;
    }
    let mut reader = ImageReader::with_format(Cursor::new(bytes), format);
    let mut limits = Limits::default();
    limits.max_alloc = Some(MAX_ALLOC_BYTES);
    limits.max_image_width = Some(width);
    limits.max_image_height = Some(height);
    reader.limits(limits);
    match reader.decode() {
        Ok(image) => Some(image),
        Err(error) => {
            tracing::warn!(%error, "stored image did not decode");
            None
        }
    }
}

/// The crop rectangle in pixels, clamped to the decoded image. Basis points
/// are ten-thousandths of the source dimension, so the arithmetic is exact in
/// `u64` and the result is at least one pixel on each axis.
fn apply_crop(image: DynamicImage, crop: ImageCrop) -> DynamicImage {
    if crop == ImageCrop::full() {
        return image;
    }
    let (source_width, source_height) = (image.width(), image.height());
    let scale = |value: u16, extent: u32| -> u32 {
        let full = u64::from(IMAGE_GEOMETRY_FULL_BP);
        u32::try_from(u64::from(value) * u64::from(extent) / full).unwrap_or(u32::MAX)
    };
    let x = scale(crop.x_bp, source_width).min(source_width.saturating_sub(1));
    let y = scale(crop.y_bp, source_height).min(source_height.saturating_sub(1));
    let width = scale(crop.width_bp, source_width)
        .max(1)
        .min(source_width - x);
    let height = scale(crop.height_bp, source_height)
        .max(1)
        .min(source_height - y);
    image.crop_imm(x, y, width, height)
}

/// The height that keeps the aspect ratio at `target` width, at least 1.
fn scaled_height(width: u32, height: u32, target: u32) -> Option<u32> {
    if width == 0 || height == 0 {
        return None;
    }
    let scaled = u64::from(height) * u64::from(target) / u64::from(width);
    Some(u32::try_from(scaled).unwrap_or(u32::MAX).max(1))
}

/// Encodes a derivative: PNG when the image carries transparency (JPEG would
/// flatten it onto black), JPEG otherwise — a photograph as a PNG is many
/// times the bytes for no visible gain.
fn encode(image: &DynamicImage) -> Option<Derivative> {
    let mut out = Vec::new();
    let content_type = if image.color().has_alpha() {
        image
            .write_to(&mut Cursor::new(&mut out), ImageFormat::Png)
            .ok()?;
        "image/png"
    } else {
        let rgb = image.to_rgb8();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, JPEG_QUALITY)
            .write_image(
                rgb.as_raw(),
                rgb.width(),
                rgb.height(),
                image::ExtendedColorType::Rgb8,
            )
            .ok()?;
        "image/jpeg"
    };
    Some(Derivative {
        content_type,
        bytes: Bytes::from(out),
    })
}
