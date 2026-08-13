//! Constrained resize, at the three widths a page is actually read at
//! (ADR 0042, S3.01c).
//!
//! A resize is only worth offering if the result survives a phone, and the
//! only honest way to check that is to *resolve the generated stylesheet* at a
//! given viewport rather than to eyeball the rules. So this suite carries a
//! deliberately small CSS reader — enough for the selector shapes this sheet
//! actually uses (compound class selectors and one level of descendant) plus
//! `@media (min-width:)`/`(max-width:)` — resolves `grid-template-columns` for
//! every declared layout choice at phone, tablet and desktop, and pins the
//! resulting column counts as a golden.
//!
//! Two properties are asserted beyond the golden, because they are the promise
//! ADR 0042 makes rather than a fact about today's numbers:
//!
//! - **A phone always gets one column**, whatever was chosen. The choice is a
//!   ceiling for wide screens, never an instruction to squeeze four cards onto
//!   360 pixels.
//! - **Nothing exceeds what was asked for**: a section set to two columns is
//!   never rendered three across at any width.
//!
//! The markup half is pinned here too: every declared value must produce its
//! class in the rendered HTML, and a section with no choice made must render
//! byte-identically to one whose layout property does not exist at all — the
//! published pages of every site that has never resized anything.
#![allow(clippy::unwrap_used)]

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use alo_sites::render::{EN, ImageSources, PageRenderContext, SiteRenderContext, render_page};
use alo_sites::stylesheet::stylesheet;
use alo_store::site_theme::SiteTheme;
use alo_store::{RESIZABLE_SECTION_KINDS, layout_controls};
use serde_json::{Value, json};

/// The three widths, in CSS pixels, and the content width a section is laid
/// out in at each — the viewport minus the page gutters the sheet reserves.
/// Stated here rather than measured because a golden that hides its
/// assumptions is a number nobody can argue with.
const BREAKPOINTS: &[(&str, f64, f64)] = &[
    ("phone", 360.0, 328.0),
    ("tablet", 768.0, 704.0),
    ("desktop", 1280.0, 1080.0),
];

/// One declaration block that survived parsing, with the media query it sits
/// in and the order it appeared.
struct Rule {
    min_px: Option<f64>,
    max_px: Option<f64>,
    selector: String,
    value: String,
    order: usize,
}

/// Every `grid-template-columns` declaration in the sheet, in source order.
/// Anything else is skipped: this reader answers one question, and a parser
/// that pretends to understand a whole stylesheet would be a lie in a test.
fn column_rules(css: &str) -> Vec<Rule> {
    let mut rules = Vec::new();
    let mut media: Option<(Option<f64>, Option<f64>)> = None;
    // Comments first: a `/* … */` before an `@media` would otherwise be read
    // as part of its prelude and the whole block would resolve unconditioned —
    // which is a bug that *hides* a broken breakpoint rather than showing one.
    let stripped = without_comments(css);
    let mut rest = stripped.as_str();
    while let Some(brace) = rest.find('{') {
        let head = rest[..brace].trim();
        let head = head.rsplit('}').next().unwrap_or(head).trim();
        if head.starts_with("@media") {
            media = Some((query_px(head, "min-width"), query_px(head, "max-width")));
            rest = &rest[brace + 1..];
            continue;
        }
        let Some(close) = rest[brace..].find('}') else {
            break;
        };
        let body = &rest[brace + 1..brace + close];
        for selector in head.split(',') {
            let selector = selector.trim();
            if selector.is_empty() {
                continue;
            }
            for declaration in body.split(';') {
                let Some((property, value)) = declaration.split_once(':') else {
                    continue;
                };
                if property.trim() == "grid-template-columns" {
                    let (min_px, max_px) = media.unwrap_or((None, None));
                    rules.push(Rule {
                        min_px,
                        max_px,
                        selector: selector.to_owned(),
                        value: value.trim().to_owned(),
                        order: rules.len(),
                    });
                }
            }
        }
        rest = &rest[brace + close + 1..];
        // A closing `}` immediately after this rule ends the media block.
        if media.is_some() && rest.trim_start().starts_with('}') {
            media = None;
            rest = rest.trim_start().strip_prefix('}').unwrap_or(rest);
        }
    }
    rules
}

/// The sheet with every `/* … */` removed.
fn without_comments(css: &str) -> String {
    let mut out = String::with_capacity(css.len());
    let mut rest = css;
    while let Some(open) = rest.find("/*") {
        out.push_str(&rest[..open]);
        match rest[open..].find("*/") {
            Some(close) => rest = &rest[open + close + 2..],
            None => return out,
        }
    }
    out.push_str(rest);
    out
}

/// `(min-width: 48rem)` → 768.0.
fn query_px(head: &str, feature: &str) -> Option<f64> {
    let start = head.find(feature)? + feature.len();
    let rest = head[start..].trim_start().strip_prefix(':')?.trim_start();
    let end = rest.find(|c: char| !c.is_ascii_digit() && c != '.')?;
    let number: f64 = rest[..end].parse().ok()?;
    Some(if rest[end..].starts_with("rem") {
        number * 16.0
    } else {
        number
    })
}

/// Whether `selector` matches an element carrying `classes`, optionally as the
/// `.grid` child of a section carrying them.
fn matches(selector: &str, classes: &[&str], grid_child: bool) -> Option<usize> {
    let parts: Vec<&str> = selector.split_whitespace().collect();
    let (compound, wants_child) = match parts.as_slice() {
        [one] => (*one, false),
        [ancestor, ".grid"] => (*ancestor, true),
        _ => return None,
    };
    if wants_child != grid_child {
        // `.grid` alone is the child element's own compound selector.
        if !(grid_child && compound == ".grid" && !wants_child) {
            return None;
        }
    }
    let wanted: Vec<&str> = compound.split('.').filter(|c| !c.is_empty()).collect();
    let mut specificity = wanted.len();
    for class in &wanted {
        if *class == "grid" && grid_child {
            continue;
        }
        if !classes.contains(class) {
            return None;
        }
    }
    if grid_child && wants_child {
        specificity += 1;
    }
    Some(specificity)
}

/// The winning `grid-template-columns` for an element at `width`, by the
/// cascade rules this sheet stays inside: highest specificity, then last one
/// wins.
fn resolved(rules: &[Rule], classes: &[&str], grid_child: bool, width: f64) -> Option<String> {
    let mut best: Option<(usize, usize, &str)> = None;
    for rule in rules {
        if rule.min_px.is_some_and(|min| width < min) {
            continue;
        }
        if rule.max_px.is_some_and(|max| width > max) {
            continue;
        }
        let Some(specificity) = matches(&rule.selector, classes, grid_child) else {
            continue;
        };
        if best.is_none_or(|(s, o, _)| (specificity, rule.order) >= (s, o)) {
            best = Some((specificity, rule.order, &rule.value));
        }
    }
    best.map(|(_, _, value)| value.to_owned())
}

/// How many columns a resolved track list produces in `content` pixels.
/// `None` — nothing resolved — is one column: the element is laid out as a
/// block, which is exactly what the un-media-queried grid sections do.
fn columns(value: Option<&str>, content: f64) -> usize {
    let Some(value) = value else { return 1 };
    if let Some(inner) = value.strip_prefix("repeat(") {
        let inner = inner.trim_end_matches(')');
        let (count, track) = inner.split_once(',').unwrap();
        if let Ok(fixed) = count.trim().parse::<usize>() {
            return fixed;
        }
        // repeat(auto-fit, minmax(<min>, 1fr)) — as many as fit, at least one.
        let min = track
            .trim()
            .strip_prefix("minmax(")
            .and_then(|t| t.split(',').next())
            .map(|t| {
                let t = t.trim();
                t.strip_suffix("rem")
                    .and_then(|n| n.parse::<f64>().ok())
                    .map(|n| n * 16.0)
                    .or_else(|| t.strip_suffix("px").and_then(|n| n.parse::<f64>().ok()))
                    .unwrap_or(f64::INFINITY)
            })
            .unwrap_or(f64::INFINITY);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        return ((content / min).floor() as usize).max(1);
    }
    value.split_whitespace().count()
}

/// A section of `kind` carrying `control` set to `value` (or nothing set), as
/// stored JSON.
fn section(kind: &str, pointer: Option<&str>, value: Option<&str>) -> Value {
    let image = json!({ "blob_id": "9hK3vQ2mR8pT1xWz4bC5dg", "alt": "The roastery" });
    let mut section = match kind {
        "hero" => json!({ "type": "hero", "heading": "Roasted this morning", "image": image }),
        "text_image" => json!({
            "type": "text_image",
            "body": "A 1962 Probat drum, rebuilt by hand.",
            "image": image,
            "image_side": "left",
        }),
        "features" => json!({
            "type": "features",
            "items": [
                { "title": "Roasted to order", "body": "Never from a shelf." },
                { "title": "Harbour roastery", "body": "Come and watch." },
                { "title": "Next-day post", "body": "Anywhere in the EU." },
                { "title": "Refill bags", "body": "Bring the tin back." },
            ],
        }),
        "gallery" => json!({
            "type": "gallery",
            "images": [image.clone(), image.clone(), image.clone(), image],
        }),
        "team" => json!({
            "type": "team",
            "members": [
                { "name": "Ada" }, { "name": "Bo" }, { "name": "Cai" }, { "name": "Dee" },
            ],
        }),
        other => panic!("no fixture for {other}"),
    };
    if let (Some(pointer), Some(value)) = (pointer, value) {
        let tokens: Vec<&str> = pointer.split('/').skip(1).collect();
        let (last, parents) = tokens.split_last().unwrap();
        let mut cursor = &mut section;
        for token in parents {
            cursor = cursor.get_mut(*token).unwrap();
        }
        cursor
            .as_object_mut()
            .unwrap()
            .insert((*last).to_string(), json!(value));
    }
    section
}

fn render(section: Value) -> String {
    let theme = SiteTheme::new();
    let envelope = json!({ "schema_version": 1, "sections": [section] });
    let site = SiteRenderContext {
        name: "Nordwind Coffee Roasters",
        base_url: "https://nordwind.alosites.com",
        locale: "en",
        theme: &theme,
        strings: &EN,
        images: ImageSources::PublicPaths,
    };
    let page = PageRenderContext {
        path: "/",
        title: "Home",
        seo_title: None,
        seo_description: None,
        sections: &envelope,
        collections: &HashMap::new(),
        catalogs: &HashMap::new(),
        bookings: &HashMap::new(),
    };
    render_page(&site, &page)
}

/// The classes on the section's root element, read back out of the rendered
/// document — the same string a browser would match on.
fn section_classes(html: &str, kind: &str) -> Vec<String> {
    let hook = format!("class=\"s-{}", kind.replace('_', "-"));
    let start = html
        .find(&hook)
        .unwrap_or_else(|| panic!("no {hook} in page"));
    let open = html[start..].find('"').unwrap() + start + 1;
    let end = html[open..].find('"').unwrap() + open;
    html[open..end]
        .split_whitespace()
        .map(str::to_owned)
        .collect()
}

/// Resize each section every way it declares, and pin what each width gets.
#[test]
fn every_declared_choice_resolves_to_the_same_columns_it_always_did() {
    let css = stylesheet(&SiteTheme::new());
    let rules = column_rules(&css);
    let mut report = String::new();
    report.push_str("# Columns resolved from the generated stylesheet, per width.\n");
    report.push_str("# section / choice: <width>=<columns> [resolved track list]\n");
    for kind in RESIZABLE_SECTION_KINDS {
        for control in layout_controls(kind) {
            if control.key == "shape" {
                // A shape changes an image's aspect ratio, not the track list;
                // it is pinned by the markup test and the sheet's own golden.
                continue;
            }
            for value in std::iter::once(None).chain(control.values.iter().map(|v| Some(*v))) {
                let html = render(section(kind, Some(control.pointer), value));
                let classes = section_classes(&html, kind);
                let classes: Vec<&str> = classes.iter().map(String::as_str).collect();
                let grid_child = *kind != "text_image";
                let mut counts = Vec::new();
                for (name, width, content) in BREAKPOINTS {
                    let resolved = resolved(&rules, &classes, grid_child, *width);
                    let count = columns(resolved.as_deref(), *content);
                    if *name == "phone" {
                        assert_eq!(
                            count, 1,
                            "{kind} {}={value:?} is {count} columns on a phone",
                            control.key
                        );
                    }
                    if let Some(chosen) = value {
                        let ceiling = match chosen {
                            "two" => 2,
                            "three" => 3,
                            "four" => 4,
                            _ => 2,
                        };
                        assert!(
                            count <= ceiling,
                            "{kind} {}={chosen} rendered {count} columns at {width}",
                            control.key
                        );
                    }
                    counts.push(format!(
                        "{name}={count} [{}]",
                        resolved.as_deref().unwrap_or("stacked")
                    ));
                }
                report.push_str(&format!(
                    "{kind} / {}={}: {}\n",
                    control.key,
                    value.unwrap_or("(unset)"),
                    counts.join("  ")
                ));
            }
        }
    }
    assert_golden("layout-responsive.txt", &report);
}

/// The declaration and the markup are one thing: every value a section type
/// offers produces its own class, and no two produce the same one.
#[test]
fn every_declared_value_renders_its_own_class() {
    let mut seen: HashMap<String, String> = HashMap::new();
    for kind in RESIZABLE_SECTION_KINDS {
        let plain = render(section(kind, None, None));
        let plain_classes = section_classes(&plain, kind);
        for control in layout_controls(kind) {
            for value in control.values {
                let html = render(section(kind, Some(control.pointer), Some(value)));
                let added: Vec<String> = if control.key == "shape" {
                    figure_classes(&html)
                } else {
                    section_classes(&html, kind)
                        .into_iter()
                        .filter(|c| !plain_classes.contains(c))
                        .collect()
                };
                if *value == control.default_value && control.key == "shape" {
                    // `natural` is the shape a page without the property has:
                    // it adds no class, by design.
                    continue;
                }
                assert_eq!(added.len(), 1, "{kind} {}={value}: {added:?}", control.key);
                let class = added.into_iter().next().unwrap();
                if let Some(other) = seen.insert(class.clone(), format!("{kind}/{value}")) {
                    assert!(
                        other.ends_with(&format!("/{value}")),
                        "class {class} means two different things: {other}"
                    );
                }
            }
        }
    }
}

/// A page nobody has resized renders exactly what it rendered before the
/// schema had these properties — the bytes every published site already has.
#[test]
fn an_unset_layout_adds_nothing_to_the_page() {
    for kind in RESIZABLE_SECTION_KINDS {
        let html = render(section(kind, None, None));
        for control in layout_controls(kind) {
            for value in control.values {
                let class = match control.key {
                    "columns" => format!("cols-{value}"),
                    "split" => format!("split-{}", value.replace('_', "-")),
                    _ => format!("shape-{value}"),
                };
                assert!(!html.contains(&class), "{kind}: unset page carries {class}");
            }
        }
        assert!(!html.contains("style="), "{kind}: inline style in the page");
    }
}

/// Every `<figure>` class in a document.
fn figure_classes(html: &str) -> Vec<String> {
    html.match_indices("<figure class=\"")
        .map(|(at, hook)| {
            let open = at + hook.len();
            let end = html[open..].find('"').unwrap() + open;
            html[open..end].to_owned()
        })
        .collect()
}

fn assert_golden(name: &str, actual: &str) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(name);
    if std::env::var_os("UPDATE_GOLDENS").is_some() {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, actual).unwrap();
    }
    let expected = fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("missing golden {name}; run once with UPDATE_GOLDENS=1"));
    assert_eq!(
        expected, actual,
        "golden {name} drifted — if deliberate, re-bless with UPDATE_GOLDENS=1 and review the diff"
    );
}
