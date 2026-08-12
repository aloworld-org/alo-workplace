//! The derivative pipeline's rules, exercised on real encoded images.
//!
//! Every source here is generated in-process (no binary fixtures to trust or
//! to review), decoded back after the fact, and checked for the properties the
//! module promises: the frame is applied, nothing is ever enlarged, nothing
//! served is larger than what it came from, and no hostile or undecodable
//! input produces anything but a clean "serve the original".
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::io::Cursor;

use alo_sites::images::DERIVATIVE_WIDTHS;
use alo_sites::serve::derivative::{MAX_SOURCE_BYTES, derive};
use alo_store::SiteImageData;
use alo_store::site_model::ImageCrop;
use bytes::Bytes;
use image::{DynamicImage, ImageFormat, RgbImage, RgbaImage};

/// A photo-like source: a smooth gradient with a hard vertical split, so a
/// crop can be told apart from the whole image by reading one pixel.
fn photo(width: u32, height: u32) -> DynamicImage {
    let mut buffer = RgbImage::new(width, height);
    for (x, y, pixel) in buffer.enumerate_pixels_mut() {
        let left = x * 2 < width;
        let shade = u8::try_from((x * 255 / width.max(1) + y * 96 / height.max(1)) % 256).unwrap();
        *pixel = if left {
            image::Rgb([shade, 20, 20])
        } else {
            image::Rgb([20, 20, shade])
        };
    }
    DynamicImage::ImageRgb8(buffer)
}

fn encode(image: &DynamicImage, format: ImageFormat, content_type: &'static str) -> SiteImageData {
    let mut out = Vec::new();
    image.write_to(&mut Cursor::new(&mut out), format).unwrap();
    SiteImageData {
        content_type,
        bytes: Bytes::from(out),
    }
}

fn jpeg(width: u32, height: u32) -> SiteImageData {
    encode(&photo(width, height), ImageFormat::Jpeg, "image/jpeg")
}

fn png(width: u32, height: u32) -> SiteImageData {
    encode(&photo(width, height), ImageFormat::Png, "image/png")
}

fn decode(data: &[u8]) -> DynamicImage {
    image::ImageReader::new(Cursor::new(data))
        .with_guessed_format()
        .unwrap()
        .decode()
        .unwrap()
}

const FULL: ImageCrop = ImageCrop {
    x_bp: 0,
    y_bp: 0,
    width_bp: 10_000,
    height_bp: 10_000,
};

/// The right half of a source.
const RIGHT_HALF: ImageCrop = ImageCrop {
    x_bp: 5_000,
    y_bp: 0,
    width_bp: 5_000,
    height_bp: 10_000,
};

#[test]
fn every_rung_comes_back_at_exactly_that_width_and_keeps_the_aspect_ratio() {
    let source = jpeg(3000, 2000);
    for width in DERIVATIVE_WIDTHS {
        let derived = derive(&source, FULL, width).expect("a 3000px photo shrinks to every rung");
        let image = decode(&derived.bytes);
        assert_eq!(image.width(), width);
        assert_eq!(image.height(), width * 2 / 3, "3:2 stays 3:2 at {width}px");
        assert_eq!(derived.content_type, "image/jpeg");
    }
}

#[test]
fn no_derivative_is_ever_larger_than_the_photo_it_came_from() {
    for source in [jpeg(3000, 2000), png(1800, 1200), jpeg(1000, 1000)] {
        for width in DERIVATIVE_WIDTHS {
            let Some(derived) = derive(&source, FULL, width) else {
                continue; // declined: the original is served, which is smaller by definition
            };
            assert!(
                derived.bytes.len() < source.bytes.len(),
                "{width}px derivative of a {}-byte source was {} bytes",
                source.bytes.len(),
                derived.bytes.len()
            );
        }
    }
}

#[test]
fn a_photo_narrower_than_the_rung_is_never_enlarged() {
    let source = jpeg(300, 200);
    for width in DERIVATIVE_WIDTHS {
        assert!(
            derive(&source, FULL, width).is_none(),
            "a 300px photo must not be blown up to {width}px"
        );
    }
}

#[test]
fn the_crop_is_what_gets_served_and_it_is_not_enlarged_either() {
    let source = jpeg(1000, 1000);
    // The rung is wider than the crop: the frame still has to be applied, at
    // the crop's own size rather than stretched up to 1440.
    let derived = derive(&source, RIGHT_HALF, 1440).expect("a crop is always rendered");
    let image = decode(&derived.bytes);
    assert_eq!(image.width(), 500, "the right half of 1000px, not upscaled");
    assert_eq!(image.height(), 1000);

    // The source is red on its left half and blue on its right; the crop must
    // be blue throughout, which the unframed original never is.
    let rgb = image.to_rgb8();
    for x in [5, 250, 495] {
        let pixel = rgb.get_pixel(x, 500);
        assert!(
            pixel[2] > pixel[0],
            "pixel at {x} came from the left half: {pixel:?}"
        );
    }
    let whole = derive(&source, FULL, 480).expect("the whole photo shrinks to 480");
    let whole_rgb = decode(&whole.bytes).to_rgb8();
    let left = whole_rgb.get_pixel(5, 240);
    assert!(
        left[0] > left[2],
        "the unframed original keeps its red half"
    );
}

#[test]
fn a_crop_of_a_small_photo_is_still_rendered_even_if_it_costs_bytes() {
    // The byte rule ("never grow the payload") must not silently drop the
    // frame: a cropped image has no correct fallback but the derivative.
    let source = png(200, 200);
    let derived = derive(&source, RIGHT_HALF, 480).expect("a frame always renders");
    assert_eq!(decode(&derived.bytes).width(), 100);
}

#[test]
fn transparency_survives_as_png_rather_than_being_flattened_onto_black() {
    let mut buffer = RgbaImage::new(1200, 800);
    for (x, _y, pixel) in buffer.enumerate_pixels_mut() {
        *pixel = image::Rgba([200, 40, 40, u8::try_from(x % 256).unwrap()]);
    }
    let source = encode(
        &DynamicImage::ImageRgba8(buffer),
        ImageFormat::Png,
        "image/png",
    );
    let derived = derive(&source, FULL, 480).expect("a transparent PNG resizes");
    assert_eq!(derived.content_type, "image/png");
    assert!(decode(&derived.bytes).color().has_alpha());
}

#[test]
fn an_opaque_png_photo_is_re_encoded_as_a_smaller_jpeg() {
    let source = png(2000, 1400);
    let derived = derive(&source, FULL, 960).expect("a PNG photo resizes");
    assert_eq!(
        derived.content_type, "image/jpeg",
        "the served type is what came out, not what went in"
    );
    assert!(
        derived.bytes.len() < source.bytes.len(),
        "{} bytes from a {}-byte source",
        derived.bytes.len(),
        source.bytes.len()
    );
}

#[test]
fn sources_this_build_does_not_resize_fall_back_to_the_original() {
    let svg = SiteImageData {
        content_type: "image/svg+xml",
        bytes: Bytes::from_static(b"<svg xmlns=\"http://www.w3.org/2000/svg\"></svg>"),
    };
    let gif = SiteImageData {
        content_type: "image/gif",
        bytes: Bytes::from_static(b"GIF89a"),
    };
    let icon = SiteImageData {
        content_type: "image/x-icon",
        bytes: Bytes::from_static(b"\x00\x00\x01\x00"),
    };
    for source in [svg, gif, icon] {
        assert!(
            derive(&source, FULL, 480).is_none(),
            "{} must be served untouched",
            source.content_type
        );
    }
}

#[test]
fn hostile_and_broken_bytes_produce_nothing_instead_of_panicking() {
    let truncated = jpeg(900, 600);
    let half = truncated.bytes.slice(..truncated.bytes.len() / 2);
    for bytes in [
        Bytes::from_static(b""),
        Bytes::from_static(b"not an image at all"),
        half,
    ] {
        let source = SiteImageData {
            content_type: "image/jpeg",
            bytes,
        };
        // A truncated JPEG may decode to a partial image; what matters is that
        // it never panics and never produces something larger than it was.
        if let Some(derived) = derive(&source, FULL, 480) {
            assert!(derived.bytes.len() < source.bytes.len());
        }
    }
}

#[test]
fn an_image_that_claims_a_gigapixel_canvas_is_never_decoded() {
    // 40 000 × 40 000 in a 70-byte PNG header: 6.4 GB of RGBA if believed.
    let bomb = SiteImageData {
        content_type: "image/png",
        bytes: Bytes::from(png_header_claiming(40_000, 40_000)),
    };
    assert!(derive(&bomb, FULL, 480).is_none());
}

#[test]
fn a_source_over_the_byte_ceiling_is_not_decoded_at_all() {
    let mut oversized = Vec::from(jpeg(600, 400).bytes.as_ref());
    oversized.resize(MAX_SOURCE_BYTES + 1, 0);
    let source = SiteImageData {
        content_type: "image/jpeg",
        bytes: Bytes::from(oversized),
    };
    assert!(derive(&source, FULL, 480).is_none());
}

/// A structurally valid PNG whose `IHDR` claims `width` × `height` and whose
/// pixel data is a stub — the shape of a decompression bomb.
fn png_header_claiming(width: u32, height: u32) -> Vec<u8> {
    let mut out = Vec::from(b"\x89PNG\r\n\x1a\n".as_slice());
    let mut ihdr = Vec::from(b"IHDR".as_slice());
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]); // 8-bit RGBA, no interlace
    push_chunk(&mut out, &ihdr);
    push_chunk(&mut out, b"IDAT\x78\x9c\x63\x00\x00\x00\x01\x00\x01");
    push_chunk(&mut out, b"IEND");
    out
}

fn push_chunk(out: &mut Vec<u8>, chunk: &[u8]) {
    let length = u32::try_from(chunk.len() - 4).unwrap();
    out.extend_from_slice(&length.to_be_bytes());
    out.extend_from_slice(chunk);
    out.extend_from_slice(&crc32(chunk).to_be_bytes());
}

/// CRC-32 (the PNG polynomial), computed the slow way — clearer than a table
/// in a test, and fast enough for three chunks.
fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}
