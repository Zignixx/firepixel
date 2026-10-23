# Roadmap

Where Firepixel stands and what remains. Figures as of **23 Oct 2026**; details in
[docs/STATUS.md](docs/STATUS.md), [docs/PARITY.md](docs/PARITY.md) and [docs/FINDINGS.md](docs/FINDINGS.md).

## Stage

**Feature-complete preview, parity polishing.** All menus, tools, panels, the property inspector and the dialogs
exist and work; FW-PNG round-trips; CLI, JSF and MCP drive everything.

| Measure | Value |
|---|---|
| Parity units equal except for design | **49 / 49** (0 real deviations, 0 unclear, 0 under-measured; follow-up run 36 over 9 units on top of runs 35, 34 and 33, 23 Oct 2026: tester feedback batch 6 changed no class, no Firepixel-side step status and no Firepixel-side value, none of its interactions is reached by a step (runner item TW-210 for the Layers rows), three reference-side errors of run 33 are `ok` again; follow-up run 35 over 6 units, 22 Oct 2026: tester feedback batch 5 changed no class, no step status and no compared value; TW-202 (F65) shows in the Pointer shots of a selected rectangle, TW-200 (F64) in the slider pop-ups, TW-201 (F66) in the width field's tooltip; the runner item TW-197 did not recur; run 34 had confirmed TW-195, the refined follow rules of TW-192 and the runner items TW-196 and R-31) |
| Proven 1:1 | **100.0 %** (49 / 49; 100.0 % over the 48 units before run 24; run 35: 100.0 %; run 34: 100.0 %; run 33: 98.0 %; run 32: 100.0 %; run 31: 95.9 %; run 30: 98.0 %; runs 26 to 29: 100.0 %; run 25: 98.0 %; run 24: 93.9 %) |
| Measurement quota, all 49 units | **87.3 %** (1077 / 1234 steps; run 36: three reference-side steps `ok` again, `fx-bevel-emboss-fill/09` and `tool-text/11`, `13`; 87.2 % over the 48 old units; run 35: 87.0 %, 1074 / 1234, unchanged by run 35, its six units 159 / 176 as in their baselines; run 33: 87.1 %, 1075 / 1234, `tool-autoshapes/10` lost in run 34 by a reference-side focus loss; 86.9 % over the 48 old units; runs 26 to 29: 87.3 %; run 24: 87.0 %; run 17: 86.3 %) |
| Canvas image pairs within 1 % | 186 / 212 (run 36: `tool-text/13` 0.80 %, `tool-text/11` 2.35 % and `fx-bevel-emboss-fill/09` 21.8 % back with their reference shots, the first within 1 % although the reference's edit did not happen; 177 / 203 over the 48 old units; run 35: 185 / 209, unchanged by run 35: the six units 32 / 40, the same pairs; `inspector-fields/14` 2.03 → 1.58 % with the slider pop-up flush with its arrow, still above 1 % by the live preview of TW-171; run 34: `tool-autoshapes/05` back below 1 %, 0.97 %, with TW-195 fixed; `tool-text/14, 16` above 1 %, TW-165). Tiled canvas (BA-098): no sampling offset at fractional zoom |
| Errors | 34 reference-side, 21 Firepixel-side (run 36: three reference-side errors of run 33 `ok` again; 7 `runner error` lines in run 36, reference-side text selection in `tool-text`, TW-146 rest; run 35: 37 and 21, unchanged by run 35, no step status changed; run 34 added the reference-side `tool-autoshapes/10`: another application's window took the foreground); 0 `runner error` lines in runs 34 and 35 (over the newest run of every unit: 4, reference-side text entry and selection in `tool-text` from run 33, TW-146 rest) |
| Tests | Tree with the fixes after v0.2.9 (F73 to F78, `main` at `f5478fa0`): **3065 passed, 0 failed, 141 ignored** at `f5478fa0` (GT-1s, 23 Oct 2026, the code of preview `2026-10-23_11-02`); the first full run GT-1r on `3cf23641` had 1 failure (the CLI parameter-text check over four new parameter texts of F74 and F76) and `fp-ui --test all` ended early (BA-117), the F78 save retry fixed the `fp-script` f25 failures that a real-time scanner caused on this machine since 09:46 (os error 32 on the rename). Before: Tree with tester feedback batch 6 (F68 to F72): **3025 passed, 0 failed, 141 ignored** at `99114278` (GT-1q, 23 Oct 2026, the code of preview `2026-10-23_07-49` and of v0.2.9; the `fp-ui` lib process crashed once with an access violation, BA-117, and passed its 149 tests on the rerun). Before: Tree with the Dither fill edge (F67): **2986 passed, 1 failed, 141 ignored** at `80359c0b` (GT-1p, 22 Oct 2026, the code of preview `2026-10-22_21-03`); the one failure was the CLI parameter-text check over the two new `pi.fill` texts, translated in `873fa4bd` (CLI, `fp-i18n`, `fp-neutral`, `fp-engine` lib green again). Before: Tree with tester feedback batch 5 (F62 to F66): **2969 passed, 0 failed, 141 ignored** at `8c636395` (GT-1o, 22 Oct 2026, the code of preview `2026-10-22_14-27`); interim run on `8a681109` (F62 to F64) 2963 / 0 / 141. Before: tree with F61 (auto shapes, follow rules) and runners R-30/R-31: 2948 passed, 0 failed, 141 ignored at `84346fec` (GT-1n, 22 Oct 2026, the code of preview `2026-10-22_05-02`). Before: tree with F56 to F59 and runner R-29: **2929 passed, 2 failed, 141 ignored** at `5e9847e6` (GT-1m, 21 Oct 2026): the timing test `image_preview_p1::worker_cancels_outdated_jobs_and_ui_stays_fluid` (green on the rerun, BA-117) and `tw186_swap_colors::rectangle_swaps_with_one_row_repeat_and_undo`, where F57's restored toolbox colours and F58's tool-attribute follow met — reconciled in F60 `6ebdc374` (the follow wins when an object is selected; `fp-engine`/`fp-script` and the touched `fp-ui` modules green); interim run on `36874d34` 2918 / 0 / 141. Before: F55 (TW-189, the Q-60 UI test) on top of the F53 fixes and v0.2.6: 2899 / 0 / 141 at `f543d7b9` (GT-1l; the code of preview `2026-10-21_08-16` and of run 32); the F53 fixes: 2892 / 0 / 141 at `e1340a21` (GT-1k); tester feedback batch 4 (F54a to F54d): 2877 / 1 / 141 at `82b0b5b2` (GT-1j, the code of preview `2026-10-21_05-40` and of v0.2.6; one `fp-ui` lib process crash, BA-117, green on the rerun; the failure was an untranslated parameter text, fixed in `1b95dd37`); JPEG 2000 and the AI batch steps: 2852 / 1 / 141 at `3b7386fa` (GT-1i). Batch and export tests also run on macOS and Linux CI (BA-083) |
| Latest release | **v0.2.9** pre-release published on 23 Oct 2026 (from `62efd30b`): Windows x64 ZIP, macOS universal ZIP (arm64 + x86_64), Linux x86_64 tar.gz and AppImage, with tester feedback batch 6 (F68 to F72: pattern fills, paragraph indent, live previews, colour pop-up alpha, Layers selection); before: **v0.2.8** (22 Oct 2026); Linux tested in CI only, the Intel macOS slice verified by tools only; the repository is public since 20 Oct 2026 |
| Open findings | 59 fixable rows, 5 open: the new deviation TW-212 (a transform tool or numeric scale of a text: the reference changes the font size, Firepixel stores a matrix; RE spec 62 §2.4), the new runner items TW-210 (Firepixel's Layers panel in the runner is shorter than the reference's flyout, so the Layers object-row steps and TW-208 are not measured) and TW-211 (the reference's stored text alignment at unit start), the new question Q-65 (the current layer after opening T07: the reference "Ebene 1", Firepixel "Ebene 3") and the runner item TW-197, plus the unmeasured rests noted in TW-186, TW-193, TW-195 (gradient or pattern tool fills on auto shapes), TW-202, TW-207 (the Pointer's own scale handles, the numeric rotate mode, `setSelectionBounds` / `resizeSelection`, the auto-trim of menu rotations), Q-61 (the tag for further tools), Q-62 and Q-64 (RE spec 62 §5); fixed after follow-up run 36 after measurement RE-30 (RE spec 62) and pending the click-test A4-58 and A4-59: F76 `3cf23641` (Q-62 re-measured with real keys: transform mode Escape as a redo step, the Brush hands over to the Pointer at Escape and paints the whole stroke, the Marquee and the Lasso end bitmap mode at Escape while dragging, marquee nudges merge into one row; the other rules kept, none dropped) and F77 `2b702176` (Q-64: gradient and pattern fills on text from the fill handles, measured default handles, the numeric transform scales them); fixed after v0.2.9 and pending the click-test A4-56 and A4-57: F73 `2e0fe826` (TW-203 rest, Feather and Dither on text through `pi.fill` and scripting; the Alter Path item is dropped, the reference has none), F74 `b585f6b8` (TW-207 rest, the transform tools honour `XformToolXformsBFE`, the Numeric Transform dialog `NumericXformXformsBFE`) and F75 `2150bbf3` (TW-207 rest, the scripting option `transformAttributes`, `setQuadrangle` with four points); tester feedback batch 6 (23 Oct 2026, RE spec 61) fixed after v0.2.8 and pending the click-test A4-51 to A4-55 (not reached by follow-up run 36): TW-204 (F69, the paragraph indent on point text), TW-205 (F70, live Layers panel opacity, blank on mixed values), TW-206 (F70, live colour pop-up alpha, HEX and RGB keep the base colour), TW-207 (F68, patterns move with the object, measured default handles) and TW-208 (F72, Layers range selection, Ctrl/Cmd toggle); TW-209 (FB-27, blend mode hover preview) declined; status of 22 Oct 2026 before batch 6, then 4 open: TW-202 (FB-19: Pen and Pointer draw a box around a free path, the reference outline + anchors, RE spec 60 §1; fixed by F65 `8c636395`; run 35 shows the Pointer display of a selected rectangle, the Pen is reached by no step; pending the click-test A4-49), TW-203 (FB-23: the Dither fill edge, done by F67 `80359c0b`, pending the click-test A4-50), the runner item TW-197 (the reference's Properties inspector collapsed in run 34; inspector and overview pairs only; not recurred in run 35) and Q-62 (rules derived from plain-key runs before R-27, to re-measure), plus the unmeasured rests noted in TW-186, TW-193, TW-195 (gradient or pattern tool fills on auto shapes) and Q-61 (TW-185, the Tool Properties tab, declined on 21 Oct 2026); tester feedback batch 5 (22 Oct 2026, RE spec 60) fixed after v0.2.7 and pending the click-test A4-45 to A4-48: TW-198 (F62, built-in patterns, no stuck pointer after a failed command), TW-199 (F63, one cursor after focus return), TW-200 (F64, live canvas for the feather, softness, texture and text sliders, pop-up place) and TW-201 (F64, F66: text inspector tooltips); FB-17: View ▸ Tooltips off by default (F64); fixed in the night of 22 Oct 2026 after RE spec 59 (RE-27), confirmed by follow-up run 34 and pending the click-test A4-43 and A4-44: TW-195 (F61, auto shapes take the tool attributes, the Arrow Line and the Measure tool draw their own strokes and set the tool) and the refined follow rules of TW-192 (F61); the runner item TW-196 fixed by R-30 and confirmed by run 34; Q-63 closed (runner R-31 sets the tool stroke at unit start, cases c and d by F61); fixed on the evening of 21 Oct 2026 after RE spec 58, pending the click-test A4-38 to A4-42: TW-187 (F56), TW-186 and TW-194 (F57), TW-192 (F58, replaces Q-58) and TW-193 (F59, replaces Q-59), TW-192 (colours), TW-193 and TW-194 confirmed by full run 33; closed: Q-61 (measured, tag per tool in F57), the TW-175 rest (measured, equal) and the runner item TW-174 (R-29, confirmed by run 33); closed by run 32: TW-189 (Enter with a group; fixed in F55 and confirmed, pending the click-test), the runner items TW-190 and TW-191 (runner R-28) and Q-60 (runner matter); TW-188 closed by run 31 (runner matter); TW-179 fixed by runner R-27 (host bug: plain named keys were sent as their last letter); fixed on 21 Oct 2026 pending the click-test: batch 4 (TW-180 to TW-184, TW-175, in v0.2.6) and the RE-24 rules TW-173 (confirmed by run 31), TW-176 to TW-178 (F53a/F53b after v0.2.6); TW-151, TW-160 to TW-162 and TW-168 to TW-172 awaiting the click-test and the macOS re-test; TW-146 rest and TW-165 are click-test item A1-19 · 5 deferred rows, 1 open (BA-101, PSD stroke only) · 20 informational · 850+ closed |

Unit count: 49 since 19 Oct 2026; full run 24 classified all of them, including the measurement unit
`xs-select-behind` (9 steps); run 25 repeated 20 of them, run 26 five, run 27 three, run 28 two and run 29 three;
full run 30 classified all 49 again, follow-up run 31 eight of them, follow-up run 32 three, full run 33 all 49, follow-up run 34 eight, follow-up run 35 six and follow-up run 36 nine.

## Next (in order)

1. **Human click-test** of [docs/VERIFICATION.md](docs/VERIFICATION.md) (embedded panel dialogs, real keyboard
   paths, brush strokes with a real mouse, drag and drop between windows, the text selection edits of A1-19, the open
   checks in section A2), including the wave-3 Batch Studio UI, the PDF/PSD export options and section A4 (wave-4
   features, URL panel menu, typing after a double-click, the crop option, the Linux start, the pen anchor modes, the
   Select Behind hover frame and the History rows of key nudges, A4-15 to A4-18, the screen modes, A4-22, the
   Subselection display, A4-23, tester feedback batch 3, A4-24 to A4-27, a real layered PDF, A4-28, the text
   tool reset, A4-29, JPEG 2000 in PDF, A4-30, the AI batch steps upscale and face blur, A4-31 and A4-32, and the
   parity fixes of 21/22 Oct 2026, A4-38 to A4-44, tester feedback batch 5, A4-45 to A4-49, the Dither fill edge,
   A4-50, tester feedback batch 6, A4-51 to A4-55, the fixes after v0.2.9, A4-56 and A4-57, and the RE-30 fixes, A4-58
   (the Escape and arrow rules of Q-62) and A4-59 (gradient and pattern fills on text)).
2. **macOS re-test of v0.2.4 / v0.2.5 by the tester** (A4-19 to A4-21 on a real Mac; also TW-198, TW-199 and the Cmd
   and Ctrl clicks of TW-208, A4-45, A4-46, A4-55).
3. **Preview** `2026-10-21_05-40` (GT-1j) built with tester feedback batch 4 (FB-12 to FB-15 fixed, FB-13 done; FB-16 declined,
   TW-185) and released as pre-release v0.2.6 on 21 Oct 2026; before it preview `2026-10-20_19-06` (GT-1i)
   with JPEG 2000 and the AI batch steps, and v0.2.5 (20 Oct 2026) from the code of preview `2026-10-20_09-25`.
4. **Tester feedback batch 6** (23 Oct 2026, FB-24 to FB-30, measurement RE-29, RE spec 61) is on `main` after
   v0.2.8: F69 `c9c9a329` (**TW-204**, the paragraph indent on point text), F70 `4657c7cb` (**TW-205**, **TW-206**, live
   Layers opacity and colour pop-up alpha, the remaining live sliders), F72 `249f4569` (**TW-208**, Layers range
   selection) and F68 `99114278` (**TW-207**, patterns move with the object); **TW-209** (blend mode hover preview)
   declined; GT-1q 3025 passed; released as **v0.2.9**. **Follow-up run 36** (23 Oct 2026) over
   `panel-layers`, `panel-layers-menu`, `color-popup`, `color-popup-contexts`, `inspector-text`, `inspector-fields`, `fx-bevel-emboss-fill`, `tool-text` and `xs-select-behind` reached none of the batch 6
   interactions and changed no class (49 / 0 / 0 / 0, quota 87.3 %, canvas pairs 186 / 212). Then the open items:
   runners for **TW-210** (Firepixel's Layers panel height, so that the Layers selection steps and TW-208 are measured)
   and **TW-211** (the text alignment at unit start), the **Q-65** measurement (the current layer after opening T07),
   runner R-32 (**TW-197**), and the rests (the coordinate tag for the other tools, TW-195 gradient or pattern tool fills
   on auto shapes, the TW-207 rests: the Pointer's own scale handles, `setSelectionBounds` / `resizeSelection`). After
   v0.2.9: F73 `2e0fe826` (**TW-203** rest, Feather and Dither on text; no Alter Path item, the reference has none),
   F74 `b585f6b8` and F75 `2150bbf3` (**TW-207** rests, the scale-attributes option of the transform tools, the
   Numeric Transform dialog and the scripting calls), pending the click-test A4-56 and A4-57. **Measurement RE-30**
   (RE spec 62, 23 Oct 2026) is done: the **Q-62** rules re-measured with real keys and **Q-64** measured; F76
   `3cf23641` (Escape and arrow rules) and F77 `2b702176` (gradient and pattern fills on text) are on `main`, pending
   the click-test A4-58 and A4-59; new row **TW-212** (font size instead of a matrix when a text is scaled). Still open
   per the spec: Escape mid-drag after earlier drags of a transform session, other shapes than the rectangle, other
   paint tools, the oval Marquee and the Polygon Lasso mid-drag, Ctrl/Alt+arrow with a pixel selection, radial
   gradients, rotation and `XformToolXformsBFE=false` for text.
5. **After follow-up run 35** (22 Oct 2026, 14:31–15:15, 6 units: `tool-path-reshape`, `inspector-text`,
   `inspector-fields`, `xs-select-behind`, `toolbox`, `color-popup`; preview `2026-10-22_14-27` from `8c636395` with
   tester feedback batch 5, F62 `208d0564`, F63 `bd397c68`, F64 `8a681109`, F65 `8c636395`, F66 `08ef298d`; runner R-31;
   49 / 0 / 0 / 0, proven 1:1 100.0 %, quota 87.0 %, canvas pairs 185 / 209): no step status, no compared value and no
   class changed. **TW-202** shows in the Pointer shots of a selected rectangle (outline + corner squares, pairs
   unchanged), **TW-200** in the slider pop-ups (`inspector-fields/14` 2.03 → 1.58 %), **TW-201** in the width field's
   tooltip, and View ▸ Tooltips is off by default; the runner item **TW-197** did not recur. Next: the click-test
   A4-45 to A4-49 (item 1; the Pen display and the pen cursors of TW-202, the live sliders, the patterns and the focus
   return are reached by no runner step), the macOS re-test (item 2, also TW-198 and TW-199), the click-test A4-50 for the
   new Dither fill edge (**TW-203**, F67 `80359c0b`), the re-measurement of the **Q-62** rules with
   real keys and runner R-32 for **TW-197** (set the reference's Properties inspector at unit start). Before it:
   tester feedback batch 5 (22 Oct 2026, FB-17 to FB-23, measurement RE-28 / RE spec 60), GT-1o 2969 passed.
6. **After follow-up run 34** (22 Oct 2026, 8 units, preview `2026-10-22_05-02` from `84346fec` with F61, runner R-31
   with R-30; 49 / 0 / 0 / 0, proven 1:1 100.0 %, quota 87.0 %, canvas pairs 185 / 209): **TW-195** and the refined
   follow rules of **TW-192** (F61) and the runner items **TW-196** (R-30) and R-31 (the same tool stroke on both sides
   at unit start) are confirmed. Next: the click-test items A4-38 to A4-44 (item 1) and the macOS re-test (item 2),
   the re-measurement of the **Q-62** rules (from plain-key runs before R-27) with real keys, runner R-32 for the new
   runner item **TW-197** (the reference's Properties inspector collapsed; inspector and overview pairs only) and the
   unmeasured rests of TW-186, TW-193, TW-195 and Q-61; TW-176, TW-177, TW-186 and TW-187 are not reached by any step
   and stay with the click-test. Before it: in the night of 22 Oct 2026 measurement RE-27 (RE spec 59) led to F61
   (`132b4941`, `84346fec`) and the runner changes R-30 (`924fbfc7`) and R-31 (`63f1e059`), and **Q-63** was closed;
   full parity run 33 (21/22 Oct 2026, all 49 units,
   preview `2026-10-21_21-47` from `6ebdc374` with F53, F55 and F56 to F60, runner R-29 with real plain keys in every
   unit; 48 / 1 / 0 / 0, proven 1:1 98.0 %, quota 87.1 %, canvas pairs 184 / 209) confirmed TW-193, TW-194, the
   runner item TW-174 and TW-192 for the colours and found TW-195, TW-196 and Q-63; follow-up run 32 (21 Oct 2026,
   3 units, preview `2026-10-21_08-16`, runner R-28; 49 / 0 / 0 / 0) confirmed TW-189 and closed TW-190, TW-191 and
   Q-60, follow-up run 31 (8 units, preview `2026-10-21_06-27`) confirmed TW-173 and closed TW-188, and on the
   evening of 21 Oct 2026 measurement RE-26 (RE spec 58) led to the fixes F56 to F60 and runner R-29.
7. **After full parity run 30** (20/21 Oct 2026, all 49 units, preview `2026-10-20_19-06`; 48 / 1 / 0 / 0, proven
   1:1 98.0 %, quota 87.2 %, canvas pairs 185 / 212): TW-173 (the reference repeats a clone with Ctrl+Y when nothing
   was undone) is fixed (F53a, after RE spec 55 §7) and confirmed by run 31; the runner item TW-174 is fixed by R-29 and the
   questions Q-58 and Q-59 are measured and fixed as TW-192 and TW-193 (21 Oct 2026). The
   tiled canvas (BA-098) showed no one-device-pixel sampling offset at fractional zoom: the zoom shots are
   pixel-identical to the untiled baseline or differ only in label anti-aliasing, with no new offset.
8. **Remaining deferred items:** BA-101 (PSD stroke; JPEG 2000 in PDF done in `974af253`); BA-109 closed
   (`ai.upscale` `9f3a1647`, `ai.faceBlur` `4f811360`).

## What remains

### Fixable remnants (`[F]`)

- Tester feedback batch 6 of 23 Oct 2026 (E-Man, RE spec 61): TW-204 / FB-26 (the paragraph indent on point text;
  F69), TW-205 / FB-28 (live Layers panel opacity, blank fields on mixed values; F70), TW-206 / FB-29 (live colour
  pop-up alpha, HEX and RGB keep the base colour; F70), TW-207 / FB-24, FB-25 (patterns move with the object, measured
  default handles; F68) and TW-208 / FB-30 (Layers Shift range, Ctrl/Cmd toggle; F72) are fixed and wait for A4-51 to
  A4-55; TW-209 / FB-27 (blend mode hover preview) is declined. After v0.2.9 F74 and F75 fixed the TW-207 rests (the
  transform tools, the Numeric Transform dialog and the scripting calls honour their scale-attributes option; A4-57)
  and F73 the TW-203 rest (Feather and Dither on text; A4-56). Measurement RE-30 (RE spec 62) then measured Q-64:
  F77 `2b702176` draws gradient and pattern fills on text from the fill handles (A4-59). Open: the unmeasured TW-207
  rests (the Pointer's own scale handles, `setSelectionBounds` / `resizeSelection`) and TW-212 (a scaled text keeps
  its font size and gets a matrix; the reference changes the font size).
- Evening of 21 Oct 2026 (RE-26, RE spec 58): fixed and waiting for the click-test A4-38 to A4-42 are TW-186 (swap
  colours with a selection), TW-187 (point-drag threshold 3·√zoom), TW-194 (the Marquee's Escape keeps the bitmap
  selected; was TW-04), TW-192 (script shapes and tool attributes; was Q-58) and TW-193 (Layers right-click; was
  Q-59); full run 33 confirmed TW-193, TW-194 and TW-192 for the colours; Q-61 is closed as measured (tag per tool),
  the TW-175 rest is equal and the runner item TW-174 is fixed by R-29 and confirmed by run 33. Q-62 (rules
  derived from plain-key runs before R-27, DOC-27) was re-measured by RE-30 (RE spec 62 §1) and fixed by F76
  `3cf23641`, pending A4-58. Open: the unmeasured rests of TW-186, TW-193 and Q-61.
- From full run 33, fixed in the night of 22 Oct 2026 after RE-27 (RE spec 59) and confirmed by follow-up run 34: TW-195
  (F61 `132b4941`: auto shapes take the tool attributes in full; the Arrow Line draws black `bn_Hard Line` 1 px and
  the Measure tool red `bn_Hard Line` 1 px inside, both set the tool; click-test A4-43; open rest: gradient or pattern
  tool fills on auto shapes), the refined follow rules of TW-192 (F61: only a single path or text, fill none per
  document, the Default colours button keeps the tip, fresh-start stroke `bn_1-Pixel Anti-Aliased` 1 px centred;
  click-test A4-44) and the runner item TW-196 (R-30 `924fbfc7`: unit-start reset repeated after a reference
  restart); the question Q-63 is closed (runner R-31 `63f1e059` sets the tool stroke on both sides at unit start).
- Tester feedback batch 5 of 22 Oct 2026 (E-Man, RE spec 60): TW-198 / FB-20 (Pattern fill "Cotton not found" and the
  stuck pointer; F62), TW-199 / FB-18 (ghost cursor after focus return; F63), TW-200 / FB-21 (live canvas for more
  sliders, pop-up place; F64) and TW-201 / FB-22 (text inspector sliders and tooltips; F64, F66) are fixed and wait for
  A4-45 to A4-48; FB-17 made View ▸ Tooltips off by default (F64). Open: TW-202 / FB-19 (no box around a free path with
  the Pen and the Pointer, measured; fixed by F65 `8c636395`, pending the click-test A4-49) and TW-203 / FB-23 (Dither fill edge, done as an extension by F67, pending
  the click-test A4-50).
- From follow-up run 34: the runner item TW-197 (the reference's Properties inspector was collapsed from the run start
  until a reference restart; inspector and overview pairs only, no compared value; next: runner R-32 sets it at unit
  start).
- Closed by follow-up run 32: TW-189 (fixed in F55 and confirmed: Enter on the canvas keeps a selected group in both
  programs), Q-60 (runner matter: the runner's click hit the page heading, not the category row) and the runner items
  TW-190 (View ▸ Tooltips off in Firepixel's shots, no tooltip over the flyouts) and TW-191 (no reference-only
  re-selection before Delete). From run 31: the question Q-61 (closed on 21 Oct 2026, see above). Confirmed in run 31: TW-173 (Ctrl+Y repeats a clone in both programs); closed: TW-188 (runner
  matter).
- From full run 30: the runner item TW-174 (the reference's docked panels had other sizes, so `panel-layers/04`
  and `panel-swatches-mixer/19` hit other rows; fixed by R-29) and the questions Q-58 and Q-59 (now TW-192 and TW-193).
- Closed on 20 Oct 2026: U-1 rest (PDF import creates layers from optional content groups; a real
  layered PDF is click-test A4-28), TW-138 rest (R3 measured; the text tool reset of RE spec 56 in F52, click-test
  A4-29; known: the reference writes two History rows for a new text, Firepixel one) and the runner item TW-167
  (run 29).
- Moved to the click-test A1-19 on 20 Oct 2026 (runner measurement gaps, not deviations): TW-146 rest (text selection
  and size in `tool-text/07, 11, 13`) and TW-165 (the text tool reset derived from it).
- Confirmed and closed in run 28 (runner R-25): TW-166 (the reference leaves full screen by F; no `runner error` in
  `toolbox`).
- Confirmed and closed in run 27 (runner R-24): TW-159 rest (both programs keep the Freeform tool and read size 72
  after the size keys), TW-163 (the reference's Standard mode read from the window) and TW-164 (Freeform head on the
  reference side).
- Confirmed and closed in run 26: TW-158 (three screen modes, full screen shows only the canvas on black, F49), the
  runner item TW-159 except (c) and the entry part of TW-146 rest; TW-138 rest (F49) shows no deviation in `tool-text`
  and `menu-text`.
- Tester feedback, second batch (macOS): TW-160 / FB-04 (frozen window after minimise/restore), TW-161 / FB-05 (Cmd+Q
  through the Quit action with the save prompt; own macOS app menu since `e11f8d1b`, CI build green) and TW-162 / FB-03 (native title
  bar with traffic lights) are fixed, only type-checked for macOS, and wait for A4-19 to A4-21.
- Confirmed and closed in run 25: TW-148 (consecutive key nudges merge into one History row, F48), TW-157 (Select
  Behind hover line and no frame after a click, F48), TW-145 (the next text after a size edit is 72 pt in both
  programs), the runner items TW-153 to TW-156 and the TW-149 leftover. Full run 24 closed TW-149, TW-150 and TW-152.
- Tester feedback of 19 Oct 2026 (E-Man): TW-151 / FB-01 (pen tool anchor modes: the click on the last anchor
  retracts its outgoing handle) is fixed and kept until the click-test; TW-152 / FB-02 (Select Behind hover frame) is
  confirmed by run 24.
- Tester feedback of 20 Oct 2026 (E-Man): TW-168 / FB-06 (the Subselection tool shows the path outline, the anchors
  and the handles of selected points on vector objects instead of the bounding box) is fixed and waits for the
  hands-on comparison A4-23.
- Tester feedback batch 3 of 20 Oct 2026 (E-Man, RE spec 55): TW-169 / FB-07 (one History step per Pen point),
  TW-170 / FB-08 (Redo is redo only), TW-171 / FB-09, FB-10 (inspector opacity and stroke width sliders apply once,
  with a live canvas preview) and TW-172 / FB-11 (free Subselection point drags, View ▸ Snap to Pixels) are fixed in
  F51a to F51c and wait for A4-24 to A4-27.

### Deferred expansion items (`[V]`)

- **Batch studio:** none open. Done on 20 Oct 2026: `ai.upscale` (Real-ESRGAN, `9f3a1647`), `ai.faceBlur` (Haar
  cascade, `4f811360`), EXIF/XMP written into
  AVIF/HEIF and JPEG XL (BA-108) and lossy LZW with an SSIM bound plus cancel inside one encode in the GIF
  optimiser (BA-110, `lossyMinSsim`). Done in wave 3: 27 new steps (file, document, perspective with auto-straighten,
  tiles, noise, LUT, style, JPEG/web-safe, boomerang/crossfade, colour depth/profile, multi-page, metadata export,
  contact sheet, script steps with opt-ins), chained collect steps, TIFF/AVIF/HEIF/JXL metadata reading, TIFF
  EXIF/GPS/IPTC writing, MCP refusal of external/IO scripts with a deep script sandbox, opt-in GIF optimiser modes
  (figures in [docs/BATCH.md](docs/BATCH.md)), per-file progress with ETA, file order, the pro filter bar,
  drag/keyboard sorting, Save as Template and Help in the card menu.
- **PDF/PSD export:** JPEG 2000 in PDF is real JPEG 2000 (`/JPXDecode`, `974af253`). Done in wave 3: selectable PDF
  text, PSD layers per object with text layers, vector masks and layer effects.
- Done in wave 4: HTML setup (Set Defaults, external `styles.css`, Generic HTML5 style, nav-bar tests), CMD-1
  (screen capture, symbol scripts, shared palette), the ffmpeg row in Preferences, the optional AI hover preview
  and the brush preview in stacking order.
- Brush pressure and speed from a real tablet. Done on 20 Oct 2026: the batch and export tests run on macOS and
  Linux CI and all golden hashes are equal on the three systems (BA-083).

### Not feasible / informational (`[N]`)

Differences that are accepted or cannot be closed, e.g. PNG file size (+0.6 % for PNG 32 because the pure-Rust
deflate differs from the original's zlib; row filters are identical), behaviour that depends on the reference
application's embedded panel technology, and runner limitations. Full list in
[docs/FINDINGS.md](docs/FINDINGS.md).

## Decisions taken (17 Oct 2026)

| Topic | Decision |
|---|---|
| Symbol editing as a true view inside the same document | **No** — keep the internal symbol view (own session document) |
| Compact effect popup instead of effect dialogs | **No** — keep the effect dialogs |
| Symbol origin when converting to a symbol | **Centre** of the object bounds (older Firepixel files are converted on load) |
| Opacity storage | **8-bit** like the original (30 % → 77/255) |
| File Info dialog | Hide the tabs that are not implemented (only "Description") |
| CI | **On demand only** (`workflow_dispatch`, called from the pre-release workflow) |
| XMP identifier in PNG | Keep the neutral standard (`XMP` key, Dublin Core only) |
| C zlib backend for smaller PNGs | **No** — size difference stays informational |
| Gradient handles with the pointer tool | Keep: pointer shows and drags the handles, live fill while dragging |
