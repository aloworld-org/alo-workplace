//! Golden contract for real BlockNote 0.52 document JSON as persisted by alo
//! Docs. The fixture deliberately retains generated block ids and every
//! default prop so schema drift is visible here before it reaches a blog.

use alo_sites::blocknote::{BlockNoteRenderError, render_blocknote};

#[test]
fn core_doc_blocks_render_as_semantic_html() -> Result<(), BlockNoteRenderError> {
    let actual = render_blocknote(include_bytes!("fixtures/blocknote/core_document.json"))?;
    assert_eq!(actual, include_str!("golden/blocknote_core.html"));
    Ok(())
}

#[test]
fn rich_doc_blocks_have_a_readable_runtime_free_fallback() -> Result<(), BlockNoteRenderError> {
    let actual = render_blocknote(include_bytes!("fixtures/blocknote/rich_document.json"))?;
    assert_eq!(actual, include_str!("golden/blocknote_rich.html"));
    Ok(())
}

#[test]
fn hostile_rich_blocks_never_render_live_markup() -> Result<(), BlockNoteRenderError> {
    let actual = render_blocknote(include_bytes!("fixtures/blocknote/hostile_document.json"))?;

    assert!(!actual.contains("<script"));
    assert!(!actual.contains("javascript:"));
    assert!(!actual.contains("data:image/svg"));
    assert!(!actual.contains(" data-pwn="));
    assert!(!actual.contains("<img"));
    assert!(actual.contains("&lt;script&gt;alert(&#39;text&#39;)&lt;/script&gt;"));
    assert!(actual.contains("&lt;/code&gt;&lt;script&gt;alert(&#39;code&#39;)&lt;/script&gt;"));
    assert!(actual.contains("Safe child &lt;b&gt;text&lt;/b&gt;"));
    Ok(())
}
