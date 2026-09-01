//! Theme tokens → the one stylesheet of a published site (pure, infallible).
//!
//! [`stylesheet`] turns a site's theme into the complete CSS document served
//! at `/assets/site.css` (the crate-level path contract). Only the custom
//! properties in `:root` vary by preset; every rule below them is the same
//! static sheet, so a theme change is a token swap, never a layout change.
//!
//! Invariants the tests pin:
//!
//! - **Self-contained**: no `@import`, no `url()`, no `@font-face` — a
//!   published page performs zero requests beyond the three contract paths,
//!   and fonts are the preset's system stacks. This is the privacy promise,
//!   not an optimization.
//! - **Responsive**: one mobile-first sheet, a single `48rem` breakpoint;
//!   images never overflow their box.
//! - **Scriptless by default**: the collapsed mobile menu only exists under
//!   the `js` class the behavior script adds to `<html>` — without
//!   JavaScript the menu simply renders expanded, never unreachable.
//! - **Budget**: the whole sheet stays under 50 KB (byte-budget test).
//!
//! Color use sticks to the pairings the store's WCAG AA test proves for
//! every shipped palette: `text`/`muted_text` on `background`/`surface`,
//! `on_primary` on `primary`, and `primary` (links, secondary buttons) on
//! `background`/`surface`. A pairing outside that list is a bug even if it
//! happens to look fine in one preset.

use alo_store::site_theme::SiteTheme;

/// Renders the complete CSS document for a site's theme.
pub fn stylesheet(theme: &SiteTheme) -> String {
    let preset = theme.resolved_preset();
    let palette = preset.palette;
    let custom = theme.colors.as_ref();
    let typography = preset.typography;
    let background = custom.map_or(palette.background, |colors| colors.background.as_str());
    let surface = custom.map_or(palette.surface, |colors| colors.background.as_str());
    let text = custom.map_or(palette.text, |colors| colors.text.as_str());
    let muted = custom.map_or(palette.muted_text, |colors| colors.text.as_str());
    let border = custom.map_or(palette.border, |colors| colors.border.as_str());
    let accent_1 = custom.map_or(palette.primary, |colors| colors.accent_1.as_str());
    let accent_2 = custom.map_or(palette.text, |colors| colors.accent_2.as_str());
    let accent_3 = custom.map_or(palette.muted_text, |colors| colors.accent_3.as_str());
    let accent_4 = custom.map_or(palette.surface, |colors| colors.accent_4.as_str());
    let accent_5 = custom.map_or(palette.background, |colors| colors.accent_5.as_str());
    let on_primary = if custom.is_some() {
        readable_on(accent_1)
    } else {
        palette.on_primary
    };
    let on_bg = readable_on(background);
    let on_text = readable_on(text);
    let on_border = readable_on(border);
    let on_accent_1 = readable_on(accent_1);
    let on_accent_2 = readable_on(accent_2);
    let on_accent_3 = readable_on(accent_3);
    let on_accent_4 = readable_on(accent_4);
    let on_accent_5 = readable_on(accent_5);
    format!(
        "/* alo Sites stylesheet — theme preset \"{id}\" */\n\
         :root {{\n\
         --bg: {bg};\n\
         --surface: {surface};\n\
         --text: {text};\n\
         --muted: {muted};\n\
         --primary: {primary};\n\
         --on-primary: {on_primary};\n\
         --border: {border};\n\
         --accent-1: {accent_1};\n\
         --accent-2: {accent_2};\n\
         --accent-3: {accent_3};\n\
         --accent-4: {accent_4};\n\
         --accent-5: {accent_5};\n\
         --on-bg: {on_bg};\n\
         --on-text: {on_text};\n\
         --on-border: {on_border};\n\
         --on-accent-1: {on_accent_1};\n\
         --on-accent-2: {on_accent_2};\n\
         --on-accent-3: {on_accent_3};\n\
         --on-accent-4: {on_accent_4};\n\
         --on-accent-5: {on_accent_5};\n\
         --font-heading: {heading_family};\n\
         --font-body: {body_family};\n\
         --weight-heading: {heading_weight};\n\
         }}\n{BASE_RULES}",
        id = preset.id,
        bg = background,
        surface = surface,
        text = text,
        muted = muted,
        primary = accent_1,
        on_primary = on_primary,
        border = border,
        accent_1 = accent_1,
        accent_2 = accent_2,
        accent_3 = accent_3,
        accent_4 = accent_4,
        accent_5 = accent_5,
        on_bg = on_bg,
        on_text = on_text,
        on_border = on_border,
        on_accent_1 = on_accent_1,
        on_accent_2 = on_accent_2,
        on_accent_3 = on_accent_3,
        on_accent_4 = on_accent_4,
        on_accent_5 = on_accent_5,
        heading_family = typography.heading_family,
        body_family = typography.body_family,
        heading_weight = typography.heading_weight,
    )
}

/// Black or white, whichever gives the stronger contrast on a custom accent.
fn readable_on(hex: &str) -> &'static str {
    let channel = |at: usize| {
        let raw = u8::from_str_radix(&hex[at..at + 2], 16).unwrap_or_default() as f64 / 255.0;
        if raw <= 0.04045 {
            raw / 12.92
        } else {
            ((raw + 0.055) / 1.055).powf(2.4)
        }
    };
    let luminance = 0.2126 * channel(1) + 0.7152 * channel(3) + 0.0722 * channel(5);
    if (luminance + 0.05) / 0.05 >= 1.05 / (luminance + 0.05) {
        "#000000"
    } else {
        "#ffffff"
    }
}

/// Everything below `:root`: identical for every preset, tokens only via
/// `var()`. Kept as one readable constant — this *is* the design; a rule
/// added here shows up on every published site.
const BASE_RULES: &str = "\
*, *::before, *::after { box-sizing: border-box; }

body {
  margin: 0;
  background: var(--bg);
  color: var(--text);
  font-family: var(--font-body);
  font-size: 1.0625rem;
  line-height: 1.6;
}

h1, h2, h3 {
  font-family: var(--font-heading);
  font-weight: var(--weight-heading);
  line-height: 1.2;
  margin: 0 0 0.5em;
}
h1 { font-size: clamp(2rem, 5vw, 3rem); }
h2 { font-size: clamp(1.5rem, 3.5vw, 2.125rem); }
h3 { font-size: 1.125rem; }
p { margin: 0 0 1em; }
img { max-width: 100%; height: auto; display: block; }
figure { margin: 0; }
a { color: var(--primary); }
:focus-visible { outline: 2px solid var(--primary); outline-offset: 2px; }

/* Skip link: first focusable element, visible only on focus. */
.skip-link {
  position: absolute;
  left: -999rem;
  top: 0;
  z-index: 10;
  background: var(--surface);
  color: var(--text);
  padding: 0.5rem 1rem;
}
.skip-link:focus { left: 0; }

/* Direct, one-click language choices. The current language stays visible;
   no translation is hidden behind a menu. */
.language-switcher {
  max-width: 70rem;
  margin: 0 auto;
  padding: 0.5rem 1.25rem;
  display: flex;
  justify-content: flex-end;
  gap: 0.5rem;
}
.language-switcher a {
  min-width: 2.5rem;
  min-height: 2.5rem;
  display: inline-grid;
  place-items: center;
  padding: 0.25rem 0.5rem;
  border: 1px solid var(--border);
  border-radius: 0.5rem;
  color: var(--text);
  text-decoration: none;
  font-size: 0.875rem;
  font-weight: 600;
}
.language-switcher a:hover { border-color: var(--primary); }
.language-switcher a[aria-current=\"page\"] {
  background: var(--primary);
  border-color: var(--primary);
  color: var(--on-primary);
}

/* Honeypot: hidden from humans (and layout), still in the posted form. */
.hp {
  position: absolute !important;
  width: 1px;
  height: 1px;
  overflow: hidden;
  clip-path: inset(50%);
  white-space: nowrap;
}

/* Sections are centered content boxes. */
main > section { max-width: 70rem; margin: 0 auto; padding: 3rem 1.25rem; }

.button {
  display: inline-block;
  background: var(--primary);
  color: var(--on-primary);
  border: 1px solid var(--primary);
  border-radius: 0.5rem;
  padding: 0.6rem 1.2rem;
  font-weight: 600;
  text-decoration: none;
}
.button:hover { text-decoration: none; }
.button.secondary { background: transparent; color: var(--primary); }
.actions { display: flex; gap: 0.75rem; flex-wrap: wrap; }
.intro { color: var(--muted); max-width: 45rem; }

/* Navigation. The toggle only exists for the behavior script: without
   JavaScript (no .js on <html>) the menu is always expanded. */
.s-nav {
  border-bottom: 1px solid var(--border);
  background: var(--nav-bg, var(--bg));
}
.s-nav nav {
  max-width: 70rem;
  margin: 0 auto;
  padding: 0.75rem 1.25rem;
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 1rem;
}
.s-nav .brand {
  margin-right: auto;
  color: var(--nav-text, var(--text));
  font-family: var(--font-heading);
  font-weight: var(--weight-heading);
  font-size: 1.25rem;
  text-decoration: none;
}
.s-nav .logo { max-height: 2.5rem; width: auto; }
.s-nav ul {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 1rem;
}
.s-nav a {
  min-height: 2.75rem;
  display: inline-flex;
  align-items: center;
  color: var(--nav-text, var(--text));
  text-decoration: none;
}
.s-nav a:not(.button):hover,
.s-nav a:not(.button):focus-visible { color: var(--nav-hover, var(--primary)); text-decoration: underline; }
.s-nav a[aria-current=\"page\"] {
  color: var(--nav-hover, var(--primary));
  font-weight: 600;
  text-decoration: underline;
  text-decoration-thickness: 0.125rem;
  text-underline-offset: 0.35rem;
}
.s-nav :focus-visible { outline-color: var(--nav-hover, var(--primary)); }
.s-nav a.button { color: var(--on-primary); }
.nav-toggle {
  min-height: 2.75rem;
  border: 1px solid var(--border);
  border-radius: 0.5rem;
  background: var(--nav-bg, var(--surface));
  color: var(--nav-text, var(--text));
  padding: 0.4rem 0.8rem;
  font: inherit;
  cursor: pointer;
  display: none;
}

/* Blog index and article pages. */
.sr-only {
  position: absolute;
  width: 1px;
  height: 1px;
  padding: 0;
  margin: -1px;
  overflow: hidden;
  clip-path: inset(50%);
  white-space: nowrap;
  border: 0;
}
.blog-nav { border-bottom: 1px solid var(--border); }
.blog-nav nav {
  max-width: 70rem;
  margin: 0 auto;
  padding: 1rem 1.25rem;
  display: flex;
  align-items: center;
  gap: 1.25rem;
}
.blog-nav a { color: var(--text); text-decoration: none; }
.blog-nav a:hover { text-decoration: underline; }
.blog-brand {
  margin-right: auto;
  display: inline-flex;
  align-items: center;
  gap: 0.75rem;
  font-family: var(--font-heading);
  font-size: 1.25rem;
  font-weight: var(--weight-heading);
}
.blog-brand img { max-height: 2.5rem; width: auto; }
.blog-main { max-width: 70rem; margin: 0 auto; padding: 3rem 1.25rem 5rem; }
.blog-heading { margin-bottom: 2rem; }
.blog-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(min(100%, 18rem), 1fr));
  gap: 1.5rem;
}
.blog-card {
  overflow: hidden;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 0.75rem;
}
.blog-card-cover { display: block; }
.blog-card-cover img { width: 100%; aspect-ratio: 16 / 9; object-fit: cover; }
.blog-card-copy { padding: 1.5rem; }
.blog-card h2 { margin-bottom: 0.5rem; font-size: 1.5rem; }
.blog-card h2 a { color: var(--text); text-decoration: none; }
.blog-card h2 a:hover { text-decoration: underline; }
.blog-date { color: var(--muted); font-size: 0.9375rem; }
.blog-read { font-weight: 600; }
.blog-empty {
  max-width: 38rem;
  padding: 1.5rem;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 0.75rem;
  color: var(--muted);
}
.blog-pages {
  margin-top: 2.5rem;
  display: grid;
  grid-template-columns: 1fr auto 1fr;
  align-items: center;
  gap: 1rem;
  color: var(--muted);
  text-align: center;
}
.blog-pages a {
  min-height: 2.5rem;
  padding: 0.5rem 0.875rem;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border: 1px solid var(--border);
  border-radius: 0.5rem;
  background: var(--surface);
  color: var(--text);
  text-decoration: none;
}
.blog-pages a:first-child { justify-self: start; }
.blog-pages a:last-child { justify-self: end; }
.blog-pages a:hover { border-color: var(--primary); color: var(--primary); }
.blog-post { max-width: 48rem; margin: 0 auto; }
.blog-post-header { margin-bottom: 2rem; }
.blog-kicker { margin-bottom: 0.5rem; font-weight: 600; }
.blog-cover { margin-bottom: 2.5rem; }
.blog-cover img { width: 100%; border-radius: 0.75rem; }
.blog-body > * { max-width: 100%; }
.blog-body pre {
  overflow-x: auto;
  padding: 1.25rem;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 0.5rem;
}
.blog-body code { font-size: 0.9375em; }
.blog-body .doc-image { margin: 2rem 0; }
.blog-body figcaption { margin-top: 0.5rem; color: var(--muted); font-size: 0.9375rem; }
.blog-body .equation {
  margin: 1.5rem 0;
  overflow-x: auto;
  padding: 1rem 1.25rem;
  background: var(--surface);
  border-left: 0.25rem solid var(--primary);
}


@media (max-width: 47.99rem) {
  .js .nav-toggle { display: inline-block; }
  .js .s-nav ul { display: none; width: 100%; flex-direction: column; align-items: flex-start; }
  .js .s-nav .nav-toggle[aria-expanded=\"true\"] + ul { display: flex; }
}

/* Hero. */
.s-hero { text-align: center; padding-top: 4rem; }
.s-hero .subheading { font-size: 1.25rem; color: var(--muted); }
.s-hero .actions { justify-content: center; }
.s-hero figure { margin-top: 2rem; }
.s-hero img { margin: 0 auto; border-radius: 0.75rem; }
.s-hero.hero-height-compact { padding-top: 2.5rem; padding-bottom: 2.5rem; }
.s-hero.hero-height-standard { padding-top: 4rem; padding-bottom: 4rem; }
.s-hero.hero-height-tall { min-height: 70vh; padding-top: 7rem; padding-bottom: 7rem; display: grid; align-content: center; }
.s-hero.hero-width-narrow > h1, .s-hero.hero-width-narrow > .subheading { max-width: 34rem; }
.s-hero.hero-width-balanced > h1, .s-hero.hero-width-balanced > .subheading { max-width: 48rem; }
.s-hero.hero-width-wide > h1, .s-hero.hero-width-wide > .subheading { max-width: 64rem; }
.s-hero.hero-align-left { text-align: left; }
.s-hero.hero-align-center { text-align: center; }
.s-hero.hero-align-right { text-align: right; }
.s-hero.hero-align-left > h1, .s-hero.hero-align-left > .subheading { margin-left: 0; margin-right: auto; }
.s-hero.hero-align-center > h1, .s-hero.hero-align-center > .subheading { margin-left: auto; margin-right: auto; }
.s-hero.hero-align-right > h1, .s-hero.hero-align-right > .subheading { margin-left: auto; margin-right: 0; }
.s-hero.hero-align-left .actions { justify-content: flex-start; }
.s-hero.hero-align-center .actions { justify-content: center; }
.s-hero.hero-align-right .actions { justify-content: flex-end; }
.s-hero.hero-split-right.hero-has-image,
.s-hero.hero-split-left.hero-has-image {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(18rem, 1fr);
  column-gap: 3rem;
  align-items: center;
}
.s-hero.hero-split-right.hero-has-image > :not(figure) { grid-column: 1; }
.s-hero.hero-split-right.hero-has-image > figure { grid-column: 2; }
.s-hero.hero-split-left.hero-has-image > :not(figure) { grid-column: 2; }
.s-hero.hero-split-left.hero-has-image > figure { grid-column: 1; }
.s-hero.hero-split-right.hero-has-image > figure,
.s-hero.hero-split-left.hero-has-image > figure { grid-row: 1 / span 4; margin-top: 0; }
.s-hero.hero-split-right.hero-has-image img,
.s-hero.hero-split-left.hero-has-image img { width: 100%; max-height: 38rem; object-fit: cover; }
.s-hero.hero-background,
.s-hero.hero-video-background {
  position: relative;
  isolation: isolate;
  overflow: hidden;
  display: grid;
  align-content: center;
  color: var(--on-primary);
  background: var(--primary);
}
.s-hero.hero-background::before,
.s-hero.hero-video-background::before {
  position: absolute;
  z-index: -1;
  inset: 0;
  background: color-mix(in srgb, var(--text) 58%, transparent);
  content: \"\";
}
.s-hero.hero-background > figure { position: absolute; z-index: -2; inset: 0; margin: 0; }
.s-hero.hero-background > figure img { width: 100%; height: 100%; object-fit: cover; border-radius: 0; }
.s-hero.hero-video-background > figure { position: absolute; z-index: -3; inset: 0; margin: 0; }
.s-hero.hero-video-background > figure img { width: 100%; height: 100%; object-fit: cover; border-radius: 0; }
.s-hero.hero-video-background > .hero-video { position: absolute; z-index: -2; inset: 0; width: 100%; height: 100%; object-fit: cover; }
.s-hero.hero-background .subheading,
.s-hero.hero-video-background .subheading { color: inherit; }
.s-hero.hero-background a:not(.button),
.s-hero.hero-video-background a:not(.button) { color: inherit; }
.s-hero.hero-custom-appearance { background: var(--hero-bg); color: var(--hero-text); }
.s-hero.hero-custom-appearance .subheading { color: inherit; }
.s-hero.hero-custom-appearance .button {
  background: var(--hero-primary);
  border-color: var(--hero-primary);
  color: var(--hero-primary-text);
}
.s-hero.hero-custom-appearance .button:hover,
.s-hero.hero-custom-appearance .button:focus-visible {
  background: var(--hero-primary-hover);
  border-color: var(--hero-primary-hover);
  color: var(--hero-primary-hover-text);
}
.s-hero.hero-custom-appearance .button.secondary {
  background: var(--hero-secondary);
  border-color: var(--hero-secondary);
  color: var(--hero-secondary-text);
}
.s-hero.hero-custom-appearance .button.secondary:hover,
.s-hero.hero-custom-appearance .button.secondary:focus-visible {
  background: var(--hero-secondary-hover);
  border-color: var(--hero-secondary-hover);
  color: var(--hero-secondary-hover-text);
}
.s-hero.hero-editorial { border-left: 0.35rem solid var(--primary); padding-left: clamp(1.5rem, 5vw, 5rem); }
.s-hero.hero-editorial > h1 { font-size: clamp(2.75rem, 7vw, 5.5rem); max-width: 14ch; }
.s-hero.hero-editorial > .subheading { max-width: 42rem; }
.s-hero.hero-editorial figure { max-width: 70%; margin-left: auto; }

/* Hero motion: authored presets with one shared pace. Motion is enhancement,
   never meaning, and the reduced-motion branch below removes every effect. */
.s-hero.hero-motion-quick { --hero-motion-duration: 450ms; --hero-zoom-duration: 8s; }
.s-hero.hero-motion-smooth { --hero-motion-duration: 700ms; --hero-zoom-duration: 12s; }
.s-hero.hero-motion-relaxed { --hero-motion-duration: 1000ms; --hero-zoom-duration: 18s; }
.s-hero.hero-text-fade-up > h1,
.s-hero.hero-text-fade-up > .subheading,
.s-hero.hero-text-fade-up > .actions { opacity: 0; animation: hero-fade-up var(--hero-motion-duration) cubic-bezier(.22, 1, .36, 1) both; }
.s-hero.hero-text-slide-in > h1,
.s-hero.hero-text-slide-in > .subheading,
.s-hero.hero-text-slide-in > .actions { opacity: 0; animation: hero-slide-in var(--hero-motion-duration) cubic-bezier(.22, 1, .36, 1) both; }
.s-hero.hero-text-fade-up > .subheading,
.s-hero.hero-text-slide-in > .subheading { animation-delay: 120ms; }
.s-hero.hero-text-fade-up > .actions,
.s-hero.hero-text-slide-in > .actions { animation-delay: 220ms; }
.s-hero.hero-text-word-reveal .hero-word { display: inline-block; opacity: 0; animation: hero-word-reveal var(--hero-motion-duration) cubic-bezier(.22, 1, .36, 1) both; animation-delay: var(--hero-word-delay); }
.s-hero.hero-text-word-reveal > .subheading,
.s-hero.hero-text-word-reveal > .actions { opacity: 0; animation: hero-fade-up var(--hero-motion-duration) cubic-bezier(.22, 1, .36, 1) both; animation-delay: 240ms; }
.s-hero.hero-media-fade-in > figure,
.s-hero.hero-media-fade-in > .hero-video { opacity: 0; animation: hero-media-fade-in var(--hero-motion-duration) ease-out both; }
.s-hero.hero-media-slide-up > figure,
.s-hero.hero-media-slide-up > .hero-video { opacity: 0; animation: hero-media-slide-up var(--hero-motion-duration) cubic-bezier(.22, 1, .36, 1) both; }
.s-hero.hero-media-slow-zoom > figure img,
.s-hero.hero-media-slow-zoom > .hero-video { animation: hero-media-slow-zoom var(--hero-zoom-duration) ease-in-out infinite alternate; }
@keyframes hero-fade-up { from { opacity: 0; transform: translateY(1.25rem); } to { opacity: 1; transform: none; } }
@keyframes hero-slide-in { from { opacity: 0; transform: translateX(-1.75rem); } to { opacity: 1; transform: none; } }
@keyframes hero-word-reveal { from { opacity: 0; transform: translateY(.8em); } to { opacity: 1; transform: none; } }
@keyframes hero-media-fade-in { from { opacity: 0; } to { opacity: 1; } }
@keyframes hero-media-slide-up { from { opacity: 0; transform: translateY(2rem); } to { opacity: 1; transform: none; } }
@keyframes hero-media-slow-zoom { from { transform: scale(1); } to { transform: scale(1.06); } }

@media (max-width: 47.99rem) {
  .s-hero.hero-split-right.hero-has-image,
  .s-hero.hero-split-left.hero-has-image { display: block; }
  .s-hero.hero-split-right.hero-has-image > figure,
  .s-hero.hero-split-left.hero-has-image > figure { margin-top: 2rem; }
  .s-hero.hero-editorial figure { max-width: 100%; }
}
@media (prefers-reduced-motion: reduce) {
  .s-hero.hero-video-background > .hero-video { display: none; }
  .s-hero[class*=\"hero-text-\"] > h1,
  .s-hero[class*=\"hero-text-\"] > .subheading,
  .s-hero[class*=\"hero-text-\"] > .actions,
  .s-hero[class*=\"hero-media-\"] > figure,
  .s-hero[class*=\"hero-media-\"] > figure img,
  .s-hero[class*=\"hero-media-\"] > .hero-video,
  .s-hero.hero-text-word-reveal .hero-word { opacity: 1; transform: none; animation: none; }
}

/* A transition marker is authoring metadata; the following section is the visual. */
.section-presented {
  background: var(--section-bg);
  color: var(--section-text);
}
.section-presented.section-spacing-compact { padding-top: 2rem; padding-bottom: 2rem; }
.section-presented.section-spacing-standard { padding-top: 3rem; padding-bottom: 3rem; }
.section-presented.section-spacing-generous { padding-top: 5rem; padding-bottom: 5rem; }
.section-presented.section-width-narrow { max-width: 48rem; }
.section-presented.section-width-balanced { max-width: 70rem; }
.section-presented.section-width-wide { max-width: 90rem; }
.section-presented.section-align-center { text-align: center; }
.section-presented.section-align-center > .grid,
.section-presented.section-align-center > .tiers,
.section-presented.section-align-center > ul { text-align: left; }
.section-presented .button,
.section-presented button[type=\"submit\"] {
  background: var(--section-button);
  border-color: var(--section-button);
  color: var(--section-button-text);
}
.section-presented .button:hover,
.section-presented .button:focus-visible,
.section-presented button[type=\"submit\"]:hover,
.section-presented button[type=\"submit\"]:focus-visible {
  background: var(--section-button-hover);
  border-color: var(--section-button-hover);
  color: var(--section-button-hover-text);
}
.section-presented.section-cards > .grid > li,
.section-presented.section-cards > .tiers > li,
.section-presented.section-cards > ul > li,
.section-presented.section-cards > details {
  padding: 1.25rem;
  border: 1px solid color-mix(in srgb, var(--section-text) 14%, transparent);
  border-radius: 1rem;
  background: color-mix(in srgb, var(--section-bg) 92%, var(--section-text));
}
.section-presented.section-minimal > .grid > li,
.section-presented.section-minimal > .tiers > li,
.section-presented.section-minimal > ul > li { border: 0; box-shadow: none; background: transparent; }
.section-presented.section-editorial { border-left: .3rem solid var(--section-button); padding-left: clamp(1.5rem,5vw,4rem); }
.section-presented { --section-enter-duration: 700ms; }
.section-presented.section-speed-quick { --section-enter-duration: 420ms; }
.section-presented.section-speed-relaxed { --section-enter-duration: 1050ms; }
.js .section-motion { opacity: 0; transition: opacity var(--section-enter-duration) cubic-bezier(.22,1,.36,1), transform var(--section-enter-duration) cubic-bezier(.22,1,.36,1), clip-path var(--section-enter-duration) cubic-bezier(.22,1,.36,1), filter var(--section-enter-duration) ease-out; }
.js .section-enter-fade-up { transform: translateY(1.5rem); }
.js .section-enter-slide-in { transform: translateX(-2rem); }
.js .section-enter-scale-in { transform: scale(.965); }
.js .section-enter-reveal { clip-path: inset(0 0 25% 0); filter: blur(3px); }
.js .section-motion.is-visible { opacity: 1; transform: none; clip-path: inset(0); filter: none; }

.s-transition { display: none; }
.js .alo-transition {
  --alo-transition-duration: 700ms;
  opacity: 0;
  transition: opacity var(--alo-transition-duration) cubic-bezier(.22,1,.36,1), transform var(--alo-transition-duration) cubic-bezier(.22,1,.36,1), clip-path var(--alo-transition-duration) cubic-bezier(.22,1,.36,1), filter var(--alo-transition-duration) ease-out;
  will-change: opacity, transform;
}
.js .alo-transition.alo-speed-quick { --alo-transition-duration: 420ms; }
.js .alo-transition.alo-speed-relaxed { --alo-transition-duration: 1050ms; }
.js .alo-transition-slide.alo-from-up { transform: translateY(2.5rem); }
.js .alo-transition-slide.alo-from-down { transform: translateY(-2.5rem); }
.js .alo-transition-slide.alo-from-left { transform: translateX(2.5rem); }
.js .alo-transition-slide.alo-from-right { transform: translateX(-2.5rem); }
.js .alo-transition-scale { transform: scale(.965); }
.js .alo-transition-reveal { clip-path: inset(0 0 22% 0); filter: blur(3px); }
.js .alo-transition.is-visible { opacity: 1; transform: none; clip-path: inset(0); filter: none; }
@media (prefers-reduced-motion: reduce) {
  .section-presented[class*=\"section-enter-\"] { opacity: 1; transform: none; clip-path: none; filter: none; transition: none; }
  .js .alo-transition { opacity: 1; transform: none; clip-path: none; filter: none; transition: none; }
}

/* Shared card grid (features, gallery, team). */
.grid {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  gap: 1.5rem;
  grid-template-columns: repeat(auto-fit, minmax(15rem, 1fr));
}
.s-features .grid li {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 0.75rem;
  padding: 1.5rem;
}
.s-features.features-list .grid { grid-template-columns: 1fr; max-width: 52rem; }
.s-features.features-list .grid li { display: grid; grid-template-columns: minmax(10rem, .45fr) 1fr; gap: 1.5rem; align-items: baseline; }
.s-features.features-steps .grid { counter-reset: feature-step; }
.s-features.features-steps .grid li { counter-increment: feature-step; position: relative; padding-top: 3.25rem; }
.s-features.features-steps .grid li::before { content: counter(feature-step, decimal-leading-zero); position: absolute; top: 1rem; left: 1.5rem; color: var(--primary); font-weight: 700; letter-spacing: .08em; }
.s-features.features-bento .grid,
.s-features.features-spotlight .grid { grid-template-columns: repeat(2, minmax(0, 1fr)); }
.s-features.features-bento .grid li:first-child,
.s-features.features-spotlight .grid li:first-child { grid-column: 1 / -1; padding-block: 2.5rem; }
.s-features.features-bento .grid li:nth-child(4n+2) { grid-row: span 2; }
.s-features.features-spotlight .grid li:first-child h3 { font-size: clamp(1.5rem, 3vw, 2.25rem); }
@media (max-width: 42rem) {
  .s-features.features-bento .grid,
  .s-features.features-spotlight .grid { grid-template-columns: 1fr; }
  .s-features.features-list .grid li { grid-template-columns: 1fr; gap: .25rem; }
}
.s-gallery img { border-radius: 0.75rem; }

/* Base-backed collection cards. */
.collection-grid {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  gap: 1.5rem;
  grid-template-columns: repeat(auto-fit, minmax(15rem, 1fr));
}
.collection-card {
  overflow: hidden;
  padding: 1.5rem;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 0.75rem;
}
.collection-card img {
  width: 100%;
  margin-bottom: 1rem;
  border-radius: 0.5rem;
  aspect-ratio: 16 / 10;
  object-fit: cover;
}
.collection-card h3 { margin-bottom: 0.5rem; }
.collection-card h3 a { color: inherit; }
.collection-summary { color: var(--muted); }
.collection-card time { color: var(--muted); font-size: 0.9375rem; }
.collection-empty {
  max-width: 38rem;
  padding: 1.5rem;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 0.75rem;
  color: var(--muted);
}

/* Catalog: groupings of what the site offers, price aligned with the name. */
.catalog-group + .catalog-group { margin-top: 2.5rem; }
.catalog-group h3 { margin-bottom: 1rem; }
.catalog-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  gap: 1.5rem;
  grid-template-columns: repeat(auto-fit, minmax(16rem, 1fr));
}
.catalog-item {
  overflow: hidden;
  padding: 1.5rem;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 0.75rem;
}
.catalog-item img {
  width: 100%;
  margin-bottom: 1rem;
  border-radius: 0.5rem;
  aspect-ratio: 4 / 3;
  object-fit: cover;
}
.catalog-item h4 { margin-bottom: 0.25rem; font-size: 1.125rem; }
.catalog-price { font-weight: 600; }
.catalog-price .price-note { font-weight: 400; color: var(--muted); }
.catalog-unavailable {
  display: inline-block;
  margin-top: 0.25rem;
  padding: 0.125rem 0.5rem;
  border: 1px solid var(--border);
  border-radius: 999px;
  color: var(--muted);
  font-size: 0.9375rem;
}
.catalog-description { margin-top: 0.5rem; color: var(--muted); }
.catalog-qty { display: flex; align-items: center; gap: 0.5rem; margin-top: 0.75rem; }
.catalog-qty label { font-weight: 600; }
.catalog-qty input { width: 5rem; }
.catalog-order input,
.catalog-order textarea {
  background: var(--bg);
  color: var(--text);
  border: 1px solid var(--border);
  border-radius: 0.5rem;
  padding: 0.6rem 0.75rem;
  font: inherit;
}
.order-details {
  max-width: 36rem;
  margin-top: 2.5rem;
}
.order-details label { display: block; font-weight: 600; margin-bottom: 0.25rem; }
.order-details input,
.order-details textarea { width: 100%; }
.order-details textarea { min-height: 6rem; resize: vertical; }
.order-no-payment { color: var(--muted); }
.order-details button {
  background: var(--primary);
  color: var(--on-primary);
  border: 1px solid var(--primary);
  border-radius: 0.5rem;
  padding: 0.6rem 1.2rem;
  font: inherit;
  font-weight: 600;
  cursor: pointer;
}
.catalog-empty {
  max-width: 38rem;
  padding: 1.5rem;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 0.75rem;
  color: var(--muted);
}

/* What can be booked. The free times live one navigation away, so the section
   itself is a short offer and a day field. */
.s-booking {
  max-width: 38rem;
  padding: 1.5rem;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 0.75rem;
}
.booking-length { font-weight: 600; }
.booking-description,
.booking-where,
.booking-closed { color: var(--muted); }
.booking-day { margin-top: 1rem; }
.booking-day label { display: block; font-weight: 600; margin-bottom: 0.25rem; }
.booking-day input {
  width: 100%;
  padding: 0.6rem 0.75rem;
  font: inherit;
  color: inherit;
  background: var(--bg);
  border: 1px solid var(--border);
  border-radius: 0.5rem;
}
.booking-day button {
  background: var(--primary);
  color: var(--on-primary);
  border: 1px solid var(--primary);
  border-radius: 0.5rem;
  padding: 0.6rem 1.2rem;
  font: inherit;
  font-weight: 600;
  cursor: pointer;
}

/* The door to the ticket shop. The section is a short offer and one link;
   the shop itself (prices, seats) is live state on /tix, styled by the same
   minimal-page chrome as every other service document. */
.s-tickets {
  max-width: 38rem;
  padding: 1.5rem;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 0.75rem;
}
.s-tickets p { color: var(--muted); }
.s-tickets .actions { margin-top: 1rem; }

/* The door to the stock shop: the same short-offer card as the tickets door,
   for the same reason — goods, prices and shelf counts are live state on
   /shop, never bytes of this page. */
.s-shop {
  max-width: 38rem;
  padding: 1.5rem;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 0.75rem;
}
.s-shop p { color: var(--muted); }
.s-shop .actions { margin-top: 1rem; }

/* A custom-code block. The page styles the box around the frame and nothing
   inside it: the frame is another document, and the site's tokens deliberately
   do not reach in. Its height is authored (a sandboxed frame cannot be
   measured from here), so only the width is ours. */
.s-custom-code .custom-frame {
  display: block;
  width: 100%;
  border: 0;
  background: transparent;
}

/* Text beside image; sides swap by modifier, stack on small screens. */
.s-text-image { display: grid; gap: 2rem; align-items: center; }
.s-text-image img { border-radius: 0.75rem; }
@media (min-width: 48rem) {
  .s-text-image { grid-template-columns: 1fr 1fr; }
  .s-text-image.image-right figure { order: 2; }
}

/* Constrained resize (ADR 0042): the ratios and shapes a section declares in
   `alo_store::site_layout`, and nothing between them. Every rule here is a
   ceiling rather than a promise — a phone gets one column and a tablet at
   most two whatever was chosen, which is what keeps mobile good by
   construction on a page somebody has resized. */
@media (min-width: 48rem) {
  .s-text-image.split-wide-image.image-left,
  .s-text-image.split-wide-text.image-right { grid-template-columns: 3fr 2fr; }
  .s-text-image.split-wide-image.image-right,
  .s-text-image.split-wide-text.image-left { grid-template-columns: 2fr 3fr; }
  .s-text-image.split-half { grid-template-columns: 1fr 1fr; }
  .cols-2 .grid,
  .cols-3 .grid,
  .cols-4 .grid { grid-template-columns: repeat(2, minmax(0, 1fr)); }
}
@media (min-width: 70rem) {
  .cols-3 .grid { grid-template-columns: repeat(3, minmax(0, 1fr)); }
  .cols-4 .grid { grid-template-columns: repeat(4, minmax(0, 1fr)); }
}
figure.shape-wide img,
figure.shape-square img,
figure.shape-tall img { width: 100%; object-fit: cover; }
figure.shape-wide img { aspect-ratio: 16 / 9; }
figure.shape-square img { aspect-ratio: 1 / 1; }
figure.shape-tall img { aspect-ratio: 3 / 4; }

/* Testimonials. */
.s-testimonials ul {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  gap: 1.5rem;
  grid-template-columns: repeat(auto-fit, minmax(18rem, 1fr));
}
.testimonial {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 0.75rem;
  padding: 1.5rem;
  margin: 0;
}
.testimonial blockquote { margin: 0 0 1rem; font-style: italic; }
.testimonial figcaption { font-weight: 600; }
.role { color: var(--muted); font-weight: 400; }

/* Pricing. */
.tiers {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  gap: 1.5rem;
  grid-template-columns: repeat(auto-fit, minmax(16rem, 1fr));
}
.tier {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 0.75rem;
  padding: 1.5rem;
}
.tier.highlighted { border: 2px solid var(--primary); }
.price {
  font-family: var(--font-heading);
  font-weight: var(--weight-heading);
  font-size: 1.75rem;
}
.period { color: var(--muted); font-size: 1rem; font-weight: 400; }
.description { color: var(--muted); }
.tier-features { list-style: none; margin: 0 0 1.5rem; padding: 0; }
.tier-features li { padding: 0.35rem 0; border-bottom: 1px solid var(--border); }

/* Team. */
.s-team .grid li { text-align: center; }
.s-team img { border-radius: 0.75rem; margin: 0 auto 1rem; }
.s-team .role { display: block; margin-bottom: 0.5rem; }

/* FAQ: native <details> accordion. */
.s-faq details {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 0.5rem;
  padding: 0.75rem 1rem;
  margin-bottom: 0.75rem;
}
.s-faq summary { cursor: pointer; font-weight: 600; }
.s-faq details p { margin: 0.75rem 0 0; }

/* Call to action: the one primary-filled band. */
main > .s-cta {
  background: var(--primary);
  color: var(--on-primary);
  text-align: center;
  border-radius: 0.75rem;
  padding: 3rem 1.5rem;
}
.s-cta .actions { justify-content: center; }
.s-cta .button { background: var(--on-primary); color: var(--primary); border-color: var(--on-primary); }

/* Contact form. */
.s-contact-form form { max-width: 36rem; }
.s-contact-form label { display: block; font-weight: 600; margin-bottom: 0.25rem; }
.s-contact-form input,
.s-contact-form textarea {
  width: 100%;
  background: var(--bg);
  color: var(--text);
  border: 1px solid var(--border);
  border-radius: 0.5rem;
  padding: 0.6rem 0.75rem;
  font: inherit;
}
.s-contact-form textarea { min-height: 8rem; resize: vertical; }
.s-contact-form button {
  background: var(--primary);
  color: var(--on-primary);
  border: 1px solid var(--primary);
  border-radius: 0.5rem;
  padding: 0.6rem 1.2rem;
  font: inherit;
  font-weight: 600;
  cursor: pointer;
}
.form-success {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 0.5rem;
  padding: 1rem;
  font-weight: 600;
}

/* Footer. */
.s-footer {
  border-top: 1px solid var(--border);
  margin-top: 3rem;
  padding: 2rem 1.25rem;
  color: var(--muted);
}
.s-footer nav, .s-footer p { max-width: 70rem; margin: 0 auto; }
.s-footer nav + p { margin-top: 1rem; }
.s-footer ul {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-wrap: wrap;
  gap: 1rem;
}
.s-footer a { color: var(--muted); }
";
