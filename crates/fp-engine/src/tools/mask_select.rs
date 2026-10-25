//! Mask Element Selection (Firepixel extension FB-80, in the Subselection flyout, keys A / 1).
//!
//! Since FB-80 the Subselection reaches the **content** of a masked object (also a single masked
//! path or bitmap). This tool reaches its **mask**: the mask path or bitmap of the clicked masked
//! object, for a group mask the mask group (a further click then picks one of its children,
//! extension E-1) — the measured Subselection target of the reference (RE spec 68 §3.2), the same
//! as clicking the mask thumbnail in the Layers panel. A bitmap mask starts bitmap editing of the
//! mask like the panel's Edit Mask. Ordinary objects behave exactly as with the Subselection
//! (outline, hollow anchors, point dragging, marquee; RE spec 54).
//!
//! The machine is the Subselection's ([`PointerTool::mask_elements`]); only the tool id and the
//! cursor (Subselection arrow with a mask badge, [`Cursor::MaskSelect`]) differ.

use super::pointer::PointerTool;
use super::{Cursor, Overlay, ToolContext, ToolEvent, ToolId, ToolMachine, ToolResponse};

/// The Mask Element Selection tool.
#[derive(Clone, Debug)]
pub struct MaskElementTool {
    inner: PointerTool,
}

impl Default for MaskElementTool {
    fn default() -> Self {
        MaskElementTool {
            inner: PointerTool::mask_elements(),
        }
    }
}

/// The plain arrow of the Subselection (idle, over objects and empty space) becomes the arrow with
/// the mask badge; point, move and copy cursors stay those of the Subselection.
fn cursor(c: Cursor) -> Cursor {
    match c {
        Cursor::Arrow | Cursor::Subselect => Cursor::MaskSelect,
        c => c,
    }
}

impl ToolMachine for MaskElementTool {
    fn id(&self) -> ToolId {
        ToolId::MaskElementSelect
    }

    fn handle(&mut self, ev: &ToolEvent, ctx: &ToolContext) -> ToolResponse {
        let mut r = self.inner.handle(ev, ctx);
        r.cursor = cursor(r.cursor);
        r
    }

    fn overlay(&self) -> Vec<Overlay> {
        self.inner.overlay()
    }

    fn feedback(&self, ctx: &ToolContext) -> Vec<Overlay> {
        self.inner.feedback(ctx)
    }

    fn inspector_hold(&self) -> bool {
        self.inner.inspector_hold()
    }

    fn press_hit(&self) -> Option<&'static str> {
        self.inner.press_hit()
    }
}
