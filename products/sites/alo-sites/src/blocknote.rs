//! alo Docs (BlockNote JSON) to semantic blog-body HTML.
//!
//! The document editor persists BlockNote's public block JSON directly in
//! Drive. This renderer deliberately consumes that interchange shape rather
//! than BlockNote's browser-only editor runtime, keeping public Sites pages
//! small and deterministic. User text and links pass through the same
//! render-side safety primitives as ordinary Sites sections.

use serde_json::{Map, Value};
use thiserror::Error;

use crate::render::html::{esc, safe_href};

/// A stored alo Doc could not be read as a BlockNote block array.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum BlockNoteRenderError {
    /// The blob is not valid JSON.
    #[error("the document is not valid JSON")]
    InvalidJson,
    /// BlockNote documents are stored as a top-level array of blocks.
    #[error("the document root must be a block array")]
    InvalidRoot,
}

/// Renders a stored alo Doc blob into a semantic HTML fragment.
///
/// Unknown block kinds are omitted, but their child blocks are still walked.
/// That makes an older public renderer forward-compatible without ever
/// treating unknown stored markup as trusted HTML.
pub fn render_blocknote(bytes: &[u8]) -> Result<String, BlockNoteRenderError> {
    let value: Value =
        serde_json::from_slice(bytes).map_err(|_| BlockNoteRenderError::InvalidJson)?;
    let blocks = value.as_array().ok_or(BlockNoteRenderError::InvalidRoot)?;
    let mut out = String::new();
    render_blocks(blocks, &mut out);
    Ok(out)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ListKind {
    Bullet,
    Numbered,
    Check,
}

impl ListKind {
    fn from_block(block: &Value) -> Option<Self> {
        match block_type(block)? {
            "bulletListItem" => Some(Self::Bullet),
            "numberedListItem" => Some(Self::Numbered),
            "checkListItem" => Some(Self::Check),
            _ => None,
        }
    }

    fn open(self, first: &Value, out: &mut String) {
        match self {
            Self::Bullet => out.push_str("<ul>\n"),
            Self::Numbered => {
                let start = first
                    .get("props")
                    .and_then(|props| props.get("start"))
                    .and_then(Value::as_u64)
                    .unwrap_or(1);
                if start > 1 {
                    out.push_str("<ol start=\"");
                    out.push_str(&start.to_string());
                    out.push_str("\">\n");
                } else {
                    out.push_str("<ol>\n");
                }
            }
            Self::Check => out.push_str("<ul class=\"check-list\">\n"),
        }
    }

    fn close(self) -> &'static str {
        match self {
            Self::Numbered => "</ol>\n",
            Self::Bullet | Self::Check => "</ul>\n",
        }
    }
}

fn render_blocks(blocks: &[Value], out: &mut String) {
    let mut index = 0;
    while index < blocks.len() {
        if let Some(kind) = ListKind::from_block(&blocks[index]) {
            kind.open(&blocks[index], out);
            while index < blocks.len() && ListKind::from_block(&blocks[index]) == Some(kind) {
                render_list_item(&blocks[index], kind, out);
                index += 1;
            }
            out.push_str(kind.close());
            continue;
        }

        render_block(&blocks[index], out);
        index += 1;
    }
}

fn render_block(block: &Value, out: &mut String) {
    match block_type(block) {
        Some("paragraph") => render_text_block("p", block, out),
        Some("heading") => {
            let level = block
                .get("props")
                .and_then(|props| props.get("level"))
                .and_then(Value::as_u64)
                .filter(|level| (1..=6).contains(level))
                .unwrap_or(2);
            render_text_block(&format!("h{level}"), block, out);
        }
        Some("quote") => render_text_block("blockquote", block, out),
        Some("image") => render_image(block, out),
        Some("codeBlock") => {
            let language = block
                .get("props")
                .and_then(|props| props.get("language"))
                .and_then(Value::as_str)
                .unwrap_or("text");
            let mut code = String::new();
            collect_plain_inline(block.get("content"), &mut code);
            render_code(language, &code, out);
            render_children(block, out);
        }
        Some("aloCode") => {
            let props = block.get("props");
            let language = props
                .and_then(|props| props.get("language"))
                .and_then(Value::as_str)
                .unwrap_or("text");
            let code = props
                .and_then(|props| props.get("code"))
                .and_then(Value::as_str)
                .unwrap_or_default();
            render_code(language, code, out);
            render_children(block, out);
        }
        Some("equation") => {
            let latex = block
                .get("props")
                .and_then(|props| props.get("latex"))
                .and_then(Value::as_str)
                .unwrap_or_default();
            out.push_str("<div class=\"equation\" role=\"math\"><code>");
            out.push_str(&esc(latex));
            out.push_str("</code></div>\n");
            render_children(block, out);
        }
        _ => render_children(block, out),
    }
}

fn render_image(block: &Value, out: &mut String) {
    let props = block.get("props");
    let url = props
        .and_then(|props| props.get("url"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    let Some(src) = safe_image_src(url) else {
        if !url.is_empty() {
            tracing::warn!("stored BlockNote image failed the render-side allowlist; omitting it");
        }
        render_children(block, out);
        return;
    };
    let name = props
        .and_then(|props| props.get("name"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    let caption = props
        .and_then(|props| props.get("caption"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    let width = props
        .and_then(|props| props.get("previewWidth"))
        .and_then(Value::as_u64)
        .filter(|width| (48..=1600).contains(width));

    out.push_str("<figure class=\"doc-image\"><img src=\"");
    out.push_str(&src);
    out.push_str("\" alt=\"");
    out.push_str(&esc(name));
    out.push('"');
    if let Some(width) = width {
        out.push_str(" width=\"");
        out.push_str(&width.to_string());
        out.push('"');
    }
    out.push_str(" loading=\"lazy\" decoding=\"async\">");
    if !caption.is_empty() {
        out.push_str("<figcaption>");
        out.push_str(&esc(caption));
        out.push_str("</figcaption>");
    }
    out.push_str("</figure>\n");
    render_children(block, out);
}

fn render_code(language: &str, code: &str, out: &mut String) {
    let language = language_token(language);
    out.push_str("<pre><code class=\"language-");
    out.push_str(&language);
    out.push_str("\">");
    out.push_str(&esc(code));
    out.push_str("</code></pre>\n");
}

fn language_token(language: &str) -> String {
    let token: String = language
        .chars()
        .filter(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '+' | '#')
        })
        .take(32)
        .collect();
    if token.is_empty() {
        "text".to_owned()
    } else {
        token
    }
}

fn safe_image_src(url: &str) -> Option<String> {
    if url.starts_with("/assets/img/") {
        return Some(esc(url));
    }

    const RASTER_PREFIXES: [&str; 5] = [
        "data:image/png;base64,",
        "data:image/jpeg;base64,",
        "data:image/gif;base64,",
        "data:image/webp;base64,",
        "data:image/avif;base64,",
    ];
    let prefix = RASTER_PREFIXES
        .iter()
        .find(|prefix| url.starts_with(**prefix))?;
    let payload = &url[prefix.len()..];
    if payload.is_empty()
        || !payload.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'/' | b'=' | b'\r' | b'\n')
        })
    {
        return None;
    }
    Some(esc(url))
}

fn collect_plain_inline(content: Option<&Value>, out: &mut String) {
    let Some(content) = content.and_then(Value::as_array) else {
        return;
    };
    for inline in content {
        match inline.get("type").and_then(Value::as_str) {
            Some("text") => out.push_str(
                inline
                    .get("text")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
            ),
            Some("link") => collect_plain_inline(inline.get("content"), out),
            _ => {}
        }
    }
}

fn render_text_block(tag: &str, block: &Value, out: &mut String) {
    out.push('<');
    out.push_str(tag);
    out.push('>');
    render_inline_array(block.get("content"), out);
    out.push_str("</");
    out.push_str(tag);
    out.push_str(">\n");
    render_children(block, out);
}

fn render_list_item(block: &Value, kind: ListKind, out: &mut String) {
    match kind {
        ListKind::Check => {
            let checked = block
                .get("props")
                .and_then(|props| props.get("checked"))
                .and_then(Value::as_bool)
                .unwrap_or(false);
            out.push_str(if checked {
                "<li data-checked=\"true\">"
            } else {
                "<li data-checked=\"false\">"
            });
        }
        ListKind::Bullet | ListKind::Numbered => out.push_str("<li>"),
    }
    render_inline_array(block.get("content"), out);
    if let Some(children) = block.get("children").and_then(Value::as_array)
        && !children.is_empty()
    {
        out.push('\n');
        render_blocks(children, out);
    }
    out.push_str("</li>\n");
}

fn render_children(block: &Value, out: &mut String) {
    if let Some(children) = block.get("children").and_then(Value::as_array) {
        render_blocks(children, out);
    }
}

fn render_inline_array(content: Option<&Value>, out: &mut String) {
    let Some(content) = content.and_then(Value::as_array) else {
        return;
    };
    for inline in content {
        render_inline(inline, out);
    }
}

fn render_inline(inline: &Value, out: &mut String) {
    match inline.get("type").and_then(Value::as_str) {
        Some("text") => {
            let text = inline
                .get("text")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let styles = inline.get("styles").and_then(Value::as_object);
            render_styled_text(text, styles, out);
        }
        Some("link") => {
            let href = inline
                .get("href")
                .and_then(Value::as_str)
                .unwrap_or_default();
            out.push_str("<a href=\"");
            out.push_str(&safe_href(href));
            out.push_str("\">");
            render_inline_array(inline.get("content"), out);
            out.push_str("</a>");
        }
        _ => {}
    }
}

fn render_styled_text(text: &str, styles: Option<&Map<String, Value>>, out: &mut String) {
    const TAGS: [(&str, &str); 4] = [
        ("bold", "strong"),
        ("italic", "em"),
        ("underline", "u"),
        ("strike", "s"),
    ];
    let enabled = |name: &str| {
        styles
            .and_then(|styles| styles.get(name))
            .and_then(Value::as_bool)
            .unwrap_or(false)
    };

    for (style, tag) in TAGS {
        if enabled(style) {
            out.push('<');
            out.push_str(tag);
            out.push('>');
        }
    }
    out.push_str(&esc(text));
    for (style, tag) in TAGS.into_iter().rev() {
        if enabled(style) {
            out.push_str("</");
            out.push_str(tag);
            out.push('>');
        }
    }
}

fn block_type(block: &Value) -> Option<&str> {
    block.get("type").and_then(Value::as_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_documents_have_a_typed_error() {
        assert_eq!(
            render_blocknote(b"not json"),
            Err(BlockNoteRenderError::InvalidJson)
        );
        assert_eq!(
            render_blocknote(br#"{"type":"paragraph"}"#),
            Err(BlockNoteRenderError::InvalidRoot)
        );
    }

    #[test]
    fn scriptable_links_are_inert() -> Result<(), BlockNoteRenderError> {
        let doc = br#"[{"type":"paragraph","content":[{"type":"link","href":"javascript:alert(1)","content":[{"type":"text","text":"unsafe","styles":{}}]}]}]"#;
        assert_eq!(render_blocknote(doc)?, "<p><a href=\"#\">unsafe</a></p>\n");
        Ok(())
    }
}
