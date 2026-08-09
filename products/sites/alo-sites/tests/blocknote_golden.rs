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
