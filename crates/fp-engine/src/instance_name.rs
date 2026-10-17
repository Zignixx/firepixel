//! Names of symbol instances (re 30 §1, Q-23).
//!
//! - Converting to a symbol gives the new instance **no** object name; only the symbol is named.
//! - The convert dialog proposes the default name, made unique in the document: "Symbol", then
//!   "Symbol 1", "Symbol 2", … ([`unique_symbol_name`]). The entered text becomes the symbol name
//!   unchanged.
//! - Side effect of the Properties inspector: whenever its header refreshes while exactly one
//!   object is selected, that object is an instance without a name and its symbol exists, the
//!   instance gets the symbol name – written directly, **without** a history step
//!   ([`name_sole_unnamed_instance`]). A hidden inspector names nothing.
//! - The header exists only in the **object layout** of the inspector (re 35 §1.2.2): while a tool
//!   with its own inspector layout is active (eraser, brush, marquee, … – [`OWN_LAYOUT_TOOLS`]),
//!   that layout wins and nothing is named ([`apply_header_rule`]). The rule depends on the UI
//!   state after a command, not on the way it was called (menu, shortcut, script, `run_command`);
//!   without a visible inspector (headless, CLI, MCP without window) no name is written.
//! - Animation instances stay unnamed (re 35 §1.2.5: 6 of 6 runs, cause open – safe choice).

use crate::ToolId;
use crate::docops;
use crate::session::DocumentState;
use crate::session::Session;
use fp_model::{Document, ElementKind, SymbolKind};

/// Tools whose own inspector layout comes before the object layouts (re 35 §1.1, table slots
/// 1–23, plus the selection tools of the own AI functions, which sit in the same block).
pub const OWN_LAYOUT_TOOLS: [ToolId; 26] = [
    ToolId::Brush,
    ToolId::Eraser,
    ToolId::Eyedropper,
    ToolId::Lasso,
    ToolId::PolygonLasso,
    ToolId::MagicWand,
    ToolId::AiObject,
    ToolId::AiSubject,
    ToolId::AiRefineEdge,
    ToolId::Marquee,
    ToolId::OvalMarquee,
    ToolId::PaintBucket,
    ToolId::Gradient,
    ToolId::RubberStamp,
    ToolId::Pencil,
    ToolId::Blur,
    ToolId::Sharpen,
    ToolId::Dodge,
    ToolId::Burn,
    ToolId::Smudge,
    ToolId::RedEyeRemoval,
    ToolId::ReplaceColor,
    ToolId::Freeform,
    ToolId::Reshape,
    ToolId::PathScrubAdd,
    ToolId::PathScrubSubtract,
];

/// Header rule of the Properties inspector (re 35 §1.2.2), called by the UI on every refresh
/// (after selection, tool, document or command changes, also from scripts): names a sole
/// unnamed instance only when the inspector is visible and shows the object layout, i.e. no tool
/// of [`OWN_LAYOUT_TOOLS`] is active. Returns `true` when a name was written.
pub fn apply_header_rule(s: &mut Session, inspector_visible: bool) -> bool {
    if !inspector_visible || OWN_LAYOUT_TOOLS.contains(&s.tools.active) {
        return false;
    }
    s.active_mut().is_some_and(name_sole_unnamed_instance)
}

/// Proposed symbol name: `base` when free, else `base 1`, `base 2`, … (counter from 1).
pub fn unique_symbol_name(doc: &Document, base: &str) -> String {
    let taken = |n: &str| doc.symbols.iter().any(|s| s.name == n);
    if !taken(base) {
        return base.to_string();
    }
    (1u32..)
        .map(|i| format!("{base} {i}"))
        .find(|n| !taken(n))
        .unwrap_or_else(|| base.to_string())
}

/// Inspector refresh rule: exactly one selected element, an instance, empty name, symbol found ⇒
/// name := symbol name (no history step). Returns `true` when a name was written.
pub fn name_sole_unnamed_instance(d: &mut DocumentState) -> bool {
    if d.selection.elements.len() != 1 || d.selection.has_pixels() {
        return false;
    }
    let id = d.selection.elements[0];
    let page = d.page;
    let Some(path) = docops::find_on_page(&d.doc, page, id) else {
        return false;
    };
    let Some(e) = d.doc.element_at(&path) else {
        return false;
    };
    let ElementKind::Instance(i) = &e.kind else {
        return false;
    };
    if e.common.name.as_deref().is_some_and(|n| !n.is_empty()) {
        return false;
    }
    let Some((_, sym)) = d.doc.symbol_by_id(i.symbol_id) else {
        return false;
    };
    if sym.kind() == SymbolKind::Animation {
        return false; // re 35 §1.2.5: "animate selection" leaves the name empty
    }
    let name = sym.name.clone();
    docops::edit_by_id(d.doc_mut(), page, id, |e| e.common.name = Some(name)).is_some()
}
