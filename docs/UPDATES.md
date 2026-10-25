# Update notes

User-visible changes on `main` after release v0.2.14. Release notes for a tagged build are still produced by the pre-release workflow ([RELEASE.md](RELEASE.md)).

## 24 Oct 2026 — Live font preview

While the font list in Properties (or the text editor dialog) is open, the highlighted family is shown on the selected text. Moving the highlight with the pointer, the arrow keys, or the scroll wheel updates the canvas immediately.

- The document is not changed and no History row is written until Enter or a click.
- Esc, or highlighting the font the text already uses, restores the previous look.
- With the on-canvas text editor open, the preview is skipped (the editor owns the glyphs).

## 24 Oct 2026 — Properties panel can be resized

The Properties inspector at the bottom of the window can be dragged from its top edge. Before this, that edge was only a drawn line, so the panel stayed at one row, two rows, or the tab row.

- Hover the line between the canvas and Properties. The pointer becomes a vertical resize arrow.
- Drag up to make the panel taller, including past the usual two rows. Drag down to shrink it. Dragging it down to the tab row collapses it.
- The second row of controls stays hidden until the panel is tall enough for it. The Symbol Properties tab uses the extra height for its table.
- The diamond on the Properties tab still switches between one row and two. Double-clicking the tab still cycles full, half, and collapsed. Either of those clears a dragged height and returns to those three sizes.
- The dragged height is saved with the workspace. Workspaces saved before this change still open.

At least 120 pixels of the document area stay visible. The three discrete heights of the reference application are unchanged when the edge is not dragged.
