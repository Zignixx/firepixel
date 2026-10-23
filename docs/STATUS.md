# Status

Last updated: **23 Oct 2026**. This page is the single place for the current figures; the history of how they
came about is kept outside the repository ([ARCHIVE_NOTE.md](ARCHIVE_NOTE.md)).

## In one paragraph

Firepixel is feature-complete as a preview: all menus, tools, panels, the property inspector and the dialogs of
the reference application exist and work, FW-PNG files round-trip, and everything is drivable over CLI, JSF and
MCP. Work is now parity polishing: a scripted comparison runner measures Firepixel against the reference
application unit by unit, and the remaining differences are tracked as findings. Release **v0.2.10** was published on
23 Oct 2026 with the follow-up work after batch 6 (Feather and Dither on text, gradient and pattern fills on text,
"Scale attributes" for the transform tools and scripts, the Escape rules re-measured with real keys, the save retry);
v0.2.9 was published on 23 Oct 2026 with tester feedback batch 6 (pattern fills move with their object, the paragraph indent on point
text, live Layers opacity and colour pop-up alpha, Layers range selection); v0.2.8 was published on 22 Oct 2026 with
tester feedback batch 5 (patterns, cursor, Pen display, live sliders, tooltips) and the Dither fill edge; v0.2.7 was published on 22 Oct 2026 with the measured fixes of 21/22 Oct 2026 (Ctrl+Y repeat, tool attributes follow the selection, auto shapes,
Layers right-click, swap colours); v0.2.6 was published on 21 Oct 2026 for Windows x64, macOS universal arm64 + x86_64 (macOS 10.15 or later) and Linux x86_64 (tar.gz and
AppImage), with tester feedback batch 4 (Subselection snapping switch, live retouch tools, smudge past the bitmap
edge, tooltips) plus JPEG 2000 in PDF and the AI batch steps; v0.2.5 (20 Oct 2026) brought tester feedback batch 3,
PDF layers, the text default reset and the performance work, v0.2.4 (19 Oct 2026) the three screen modes and the
macOS fixes, and v0.2.2 (18 Oct 2026) was the first with Linux. The repository is public since 20 Oct 2026. Further preview builds are made on request. The Intel slice of the macOS bundle is verified with tools only, not yet
run on Intel hardware, and the Linux build has run only in CI, not yet on a real Linux desktop.

## Figures

| Area | Value | Source |
|---|---|---|
| Parity units | 49 (all classified since full run 24; the measurement unit `xs-select-behind` was added on 19 Oct 2026; full run 33 classified all 49 again on 21/22 Oct 2026, follow-up run 34 repeated eight of them and follow-up run 35 six on 22 Oct 2026, follow-up run 36 nine and follow-up run 37 the same nine with runner R-32 on 23 Oct 2026) | [PARITY.md](PARITY.md) |
| Equal except for design | **47** (2 real deviations, 0 unclear, 0 under-measured; follow-up run 37 over the nine units of run 36 with runner R-32, which reaches tester feedback batch 6: `panel-layers` is a real deviation by TW-205 (the Layers opacity slider commits 81 against 80 after a 41 px drag, the slider scale, fixed by F79 `7337428e` after the build) and TW-208 (a Shift range across layers selects 2 objects against 1, a case RE spec 61 did not measure), `inspector-text` by TW-214 (a point text made by `addNewText` lies 13 px higher); TW-206 and TW-207 confirmed, TW-204 as documented, the runner items TW-197, TW-210 and TW-211 confirmed; follow-up run 36 over the nine units touched by tester feedback batch 6 changed no class, no Firepixel-side step status and no Firepixel-side value and reached none of the batch 6 interactions (the Layers object-row steps are `unmapped` on Firepixel's side since run 24, runner item TW-210); three reference-side errors of run 33 are `ok` again; follow-up run 35 over the six units touched by tester feedback batch 5 changed no class, no step status and no compared value: TW-202 (F65) shows in the Pointer shots of a selected rectangle, TW-200 (F64) in the slider pop-ups, TW-201 (F66) in the width field's tooltip; the runner item TW-197 did not recur; run 34 had confirmed TW-195, the refined follow rules of TW-192 and the runner items TW-196 and R-31, run 33 TW-193 and TW-194) | follow-up run 37 over 9 units on top of runs 36, 35, 34 and 33, 23 Oct 2026 |
| Proven 1:1 | **95.9 %** (47 / 49; no unit under-measured; over the 48 units before run 24: 95.8 %; run 36: 100.0 %; run 35: 100.0 %; run 34: 100.0 %; run 33: 98.0 %; run 32: 100.0 %; run 31: 95.9 %; run 30: 98.0 %; runs 26 to 29: 100.0 %; run 25: 98.0 %; run 24: 93.9 %; run 17: 87.5 %) | run 37 on top of runs 36, 35, 34 and 33 |
| Canvas image pairs within 1 % | **191 / 220** (run 37: eight new canvas pairs from the batch 6 steps, five within 1 % (`panel-layers/23` press-drag 0.10 %, `color-popup/23` result 0.18 %, `inspector-text/25` 0.52 %, `inspector-fields/25` 0.09 % twice) and three above by Firepixel's deliberate live preview (2.10, 2.10 and 7.71 %); the other 43 canvas pairs of its nine units stay on their side of 1 %, `inspector-text/09, 13, 14` now compare the reference's applied edits (0.15–0.16 %) and the `tool-text` pairs fall with the left alignment of TW-211; over the 48 old units 182 / 211. Run 36, 186 / 212: three canvas pairs are back with their reference shots, `tool-text/13` within 1 % at 0.80 % although the reference's edit did not happen, `tool-text/11` at 2.35 % and `fx-bevel-emboss-fill/09` at 21.8 % above; the other 40 canvas pairs of its nine units stay on their side of 1 %; over the 48 old units 177 / 203. Run 35: 185 / 209, unchanged: its six units keep 32 of 40 canvas pairs within 1 %, the same pairs; `inspector-fields/14_popup_drag_live_canvas_pressed` fell from 2.03 to 1.58 % because the slider pop-up now sits flush with its arrow (F64) and stays above 1 % by the deliberate live opacity preview of TW-171; the Pointer display of F65 changes 15 px per shot and moves no pair, since the reference's shots carry no selection markings; `tool-text/14, 16` stay above 1 % by TW-165, click-test A1-19; over the 48 old units 176 / 200). Run 34: `tool-autoshapes/05` back below 1 %, 0.97 %, with TW-195 fixed. Tiled canvas (BA-098): no one-pixel sampling offset at fractional zoom; Firepixel's `xs-canvas` zoom shots are pixel-identical to run 33 | run 37 on top of runs 36, 35, 34 and 33 |
| Measurement quota, all 49 units | **87.8 %** (1090 / 1242 measurable steps both `ok`; run 37: ten new steps, all `ok` on both sides, and in `panel-layers` four object-row steps `ok` on Firepixel's side, `09_drag_reorder_pressed` `unmapped` on Firepixel's side and two steps that now have an `unsupported` side; its nine units 190 / 223, run 36: 177 / 215; over the 48 old units 87.7 %, 1081 / 1233. Run 36: 87.3 %, 1077 / 1234, +3, `fx-bevel-emboss-fill/09` and `tool-text/11`, `13` are `ok` on the reference side again, its nine units 177 / 215; over the 48 old units 87.2 %, 1068 / 1225. Run 35: 87.0 %, 1074 / 1234, unchanged by run 35, whose six units read 159 / 176 as in their baselines; run 33: 87.1 %, 1075 / 1234, the step lost since is `tool-autoshapes/10_scale_result`, a reference-side error after another application's window took the foreground in run 34; over the 48 old units 86.9 %, 1065 / 1225; runs 26 to 29: 87.3 %; run 24: 87.0 %; run 17 alone 86.3 %) | run 37 on top of runs 36, 35, 34 and 33 |
| Errors | 34 reference-side, 21 Firepixel-side (run 37: unchanged, the six errors of each side in its nine units are the same steps; 11 `runner error` lines in run 37, all reference-side text selection and entry in `tool-text`, TW-146 rest, now also the total over the newest run of every unit. Run 36: the three reference-side errors new in run 33 are `ok` again, no Firepixel-side status changed; 7 `runner error` lines in run 36, all reference-side text selection in `tool-text`, TW-146 rest. Run 35: 37 and 21, unchanged by run 35: no step status changed; its one Firepixel-side error is the known `inspector-fields/20_undo_each_result`, no text field at the stroke size; run 34 added the reference-side `tool-autoshapes/10`); 0 `runner error` lines in runs 34 and 35 (over the newest run of every unit: 4, all reference-side text entry and selection in `tool-text` from run 33, TW-146 rest) | run 37 on top of runs 36, 35, 34 and 33 |
| Latest release | **v0.2.10** (23 Oct 2026, pre-release from `8d1e3a77`, version bump `3cdb7cf5`, run 39928999872; Windows x64 ZIP, macOS universal ZIP, Linux x86_64 tar.gz and AppImage): the work after v0.2.9 of 23 Oct 2026 (F73 Feather/Dither edges on text, F74/F75 the transform tools and the script calls honour the scale-attributes option, F76 Escape rules re-measured with real keys (RE spec 62), F77 gradient and pattern fills on text, F78 save retry); the code of preview `2026-10-23_11-02`, GT-1s 3065 passed. Before: v0.2.9 (23 Oct 2026, tester feedback batch 6), v0.2.8 (22 Oct 2026). The macOS re-test by the tester is still pending; the release notes ask for it; fixes after v0.2.10 on `main`: F79 `7337428e` (TW-215, layer disclosure from the document, opacity sliders 2 px per per cent; pending run 38 and the click-test A4-60) | GitHub releases |
| Workspace tests | Tree with the fixes after v0.2.9 (F73 to F78, `main` at `f5478fa0`): **3065 passed, 0 failed, 141 ignored** at `f5478fa0` (GT-1s, 23 Oct 2026, the code of preview `2026-10-23_11-02`); the first full run GT-1r on `3cf23641` had 1 failure (the CLI parameter-text check over four new parameter texts of F74 and F76) and `fp-ui --test all` ended early (BA-117), the F78 save retry fixed the `fp-script` f25 failures that a real-time scanner caused on this machine since 09:46 (os error 32 on the rename). Before: Tree with tester feedback batch 6 (F68 to F72): **3025 passed, 0 failed, 141 ignored** at `99114278` (GT-1q, 23 Oct 2026, the code of preview `2026-10-23_07-49` and of v0.2.9; the `fp-ui` lib process crashed once with an access violation, BA-117, and passed its 149 tests on the rerun). Before: Tree with the Dither fill edge (F67): **2986 passed, 1 failed, 141 ignored** at `80359c0b` (GT-1p, 22 Oct 2026, the code of preview `2026-10-22_21-03`); the one failure was the CLI parameter-text check over the two new `pi.fill` texts, translated in `873fa4bd` (CLI, `fp-i18n`, `fp-neutral`, `fp-engine` lib green again). Before: Tree with tester feedback batch 5 (F62 to F66): **2969 passed, 0 failed, 141 ignored** at `8c636395` (GT-1o, 22 Oct 2026, the code of preview `2026-10-22_14-27`); interim run on `8a681109` (F62 to F64) 2963 / 0 / 141. Before: tree with F61 (auto shapes, follow rules) and runners R-30/R-31: 2948 passed, 0 failed, 141 ignored at `84346fec` (GT-1n, 22 Oct 2026, the code of preview `2026-10-22_05-02`). Before: tree with F56 to F59 and runner R-29: **2929 passed, 2 failed, 141 ignored** at `5e9847e6` (GT-1m, 21 Oct 2026): the timing test `image_preview_p1::worker_cancels_outdated_jobs_and_ui_stays_fluid` (green on the rerun, BA-117) and `tw186_swap_colors::rectangle_swaps_with_one_row_repeat_and_undo`, where F57's restored toolbox colours and F58's tool-attribute follow met — reconciled in F60 `6ebdc374` (the follow wins when an object is selected; `fp-engine`/`fp-script` and the touched `fp-ui` modules green); interim run on `36874d34` (without F58) 2918 / 0 / 141 (GT-1m-pre). Before: tree with F55 (TW-189 and the Q-60 UI test) on top of the F53 fixes and release v0.2.6: 2899 passed, 0 failed, 141 ignored at `f543d7b9` (GT-1l); preview `2026-10-21_08-16` is built from this code and is the code of follow-up run 32; before it: tree with the F53 fixes: 2892 passed, 0 failed, 141 ignored at `3b30220b` (= `e1340a21` on `main`, GT-1k); before it: tree with tester feedback batch 4 (F54a to F54d): 2877 passed, 1 failed, 141 ignored at `82b0b5b2` (GT-1j, the code of preview `2026-10-21_05-40` and of v0.2.6; the `fp-ui` lib process crashed once with an access violation, BA-117, and passed its 144 tests on the rerun; the one failure was the untranslated `confineToBounds` parameter text, fixed in `1b95dd37`); interim run on `6e9e5891` 2864 / 0 / 141; tree with JPEG 2000 (BA-101) and the AI batch steps (BA-109): 2852 / 1 / 141 at `3b7386fa` (GT-1i, the i18n scanner failure fixed in `8ab8564d`) | `main`, 23 Oct 2026 |
| Open findings | 63 rows in `[F]` after follow-up run 37, open: the deviations TW-208 (follow-up run 37: a Shift range from an object on "Ebene 1" to the shared layer's object selects 2 objects in Firepixel and 1 in the reference; ranges across layers were not measured in RE spec 61, to measure), TW-213 (a text made by `addNewText` keeps the inspector's last paragraph indent, to fix in `crates/fp-script`), TW-214 (a point text made by `addNewText` lies 13 px higher than the reference's, to measure and fix) and TW-212, the runner item TW-216 (the Layers drag steps collapse layers on the reference's side only; no selection-dot target) and the question Q-65; TW-205 (the slider scale) and TW-215 (F79) are fixed after run 37's build and wait for run 38 and the click-test A4-52 and A4-60; the runner items TW-197, TW-210 and TW-211 are fixed by R-32 and confirmed by run 37. Before run 37: 59 rows in `[F]`, 5 open: the new deviation TW-212 (RE spec 62 §2.4: the reference turns a Scale tool, Free Transform or numeric scale of a text into a new font size, 60 → 84.37 pt, Firepixel stores a matrix), the new runner items TW-210 (follow-up run 36: Firepixel's Layers panel in the runner, 268 × 291 px, is shorter than the reference's flyout, 245 × 389 px, so the Layers object-row steps of `panel-layers` have been `unmapped` on Firepixel's side since run 24 and TW-208 is not covered; set the panel height or scroll the row into view through the bridge) and TW-211 (the unit-start text tool reset does not set the reference's stored alignment; right-aligned texts in `tool-text` in runs 27, 28, 30 and 36), the new question Q-65 (after opening T07 the reference makes "Ebene 1" current, Firepixel "Ebene 3"; what the file stores and which layer the reference makes current is to measure) and the runner item TW-197 (follow-up run 34: the reference's Properties inspector was collapsed from the run start until a restart; not recurred in run 35; runner R-32 is to set it at unit start), plus the unmeasured rests noted in TW-186, TW-193, TW-195 (gradient or pattern tool fills on auto shapes), TW-202 (the display under the Freeform, Reshape Area, Path Scrubber and Knife tools), TW-207 (the Pointer's own scale handles always transform the fill handles; the Numeric Transform dialog's rotate mode passes no flag; `setSelectionBounds` and `resizeSelection` ignore the options string; the menu rotations do not trim bitmaps although the measured row carries `autoTrimImages`), Q-61 (the tag for further tools), Q-62 and Q-64 (RE spec 62 §5). Fixed on `main` after follow-up run 36 (23 Oct 2026) after measurement RE-30 (RE spec 62), not checked in a real window, pending the click-test A4-58 and A4-59: F76 `3cf23641` (Q-62, the rules from plain-key runs re-measured with real keys: transform mode Escape undoes the session and leaves a "Transform" redo step, Escape with the mouse held ends the mode on release without a row; the Brush switches to the Pointer at Escape mid-stroke and paints the whole stroke on release; the Marquee and the Lasso end bitmap mode at Escape while dragging; marquee arrow nudges merge into one "Move" row; the number field, rectangle, Magic Wand and text editor rules kept, none dropped) and F77 `2b702176` (Q-64: gradient and pattern fills on text render from the fill handles in document coordinates, the measured default handles, a linear gradient's third handle at h1 + (height, 0) for paths too, the numeric transform scales a text's handles about the centre whatever "Scale attributes" says, the Scale tool and Free Transform leave them). Fixed on `main` after v0.2.9 (23 Oct 2026), not checked in a real window, pending the click-test A4-56 and A4-57: F73 `2e0fe826` (TW-203 rest: text fills take the Feather and Dither edge kinds like paths, reachable through `pi.fill` and scripting since the text inspector has no Edge combo; the Modify ▸ Alter Path "Dither Fill" item is dropped, the reference has no such item; TW-203 stays done as a Firepixel extension, pending the click-test A4-50 and A4-56), F74 `b585f6b8` (TW-207 rest: the Scale, Skew, Distort and Free Transform tools honour `XformToolXformsBFE` for the fill handles, the Numeric Transform dialog's checkbox reads and writes `NumericXformXformsBFE`) and F75 `2150bbf3` (TW-207 rest: `scaleSelection`, `rotateSelection`, `transformSelection` and `setQuadrangle` transform the fill handles only with the option `transformAttributes`; `setQuadrangle` takes the four points of the API table; History rows carry `scaleAttributes` explicitly). Tester feedback batch 6 (23 Oct 2026, FB-24 to FB-30, measurement RE-29, RE spec 61), fixed on `main` after v0.2.8 and pending the click-test A4-51 to A4-55 (follow-up run 36 reached none of them): TW-204 (F69 `c9c9a329`: the paragraph indent moves the first line of every paragraph also on point text), TW-205 (F70 `4657c7cb`: live Layers panel opacity, blank opacity and blend mode fields on mixed values), TW-206 (F70: live colour pop-up alpha, applied on Enter or on release after a press-and-drag, HEX and RGB always the base colour; also the Tools panel wells, the Color Mixer, the text colours and the rectangle roundness live), TW-207 (F68 `99114278`: the pattern tile origin sits on fill handle 1, so patterns move with the object; measured default pattern handles; real-window check over the bridge 0 differing pixels) and TW-208 (F72 `249f4569`: Shift range from the anchor, Ctrl/Cmd toggle, a click on a selected row keeps the selection, a layer row selects its objects); TW-209 (FB-27, a blend mode preview while hovering the list) is closed as declined by our call. Status of 22 Oct 2026 before batch 6, then 4 open: TW-202 (tester feedback FB-19: the Pen and the Pointer draw a bounding box around a free path; measured in RE spec 60 §1, the reference shows outline + anchors; fixed by F65 `8c636395`; follow-up run 35 shows the Pointer display of a selected rectangle, the Pen is reached by no step; pending the click-test A4-49; unmeasured rest: the display under the Freeform, Reshape Area, Path Scrubber and Knife tools, where Firepixel keeps the box), TW-203 (FB-23: the Dither fill edge, done as a Firepixel extension by F67 `80359c0b`, pending the click-test A4-50), the runner item TW-197 (follow-up run 34: the reference's Properties inspector was collapsed from the run start until a restart; inspector and overview pairs only, no compared value; not recurred in run 35) and the question Q-62 (Firepixel rules derived from plain-key runs before runner R-27, to re-measure with real keys), plus the unmeasured rests noted in TW-186 (English row names, the Eyedropper's undo of the tool colours), TW-193 (right-click inside a multi-selection, the footer Add Mask button), TW-195 (gradient or pattern tool fills on auto shapes) and Q-61 (the tag for the Subselection, transform, crop and marquee tools and handle drags). Tester feedback batch 5 (22 Oct 2026, FB-17 to FB-23, measurement RE-28, RE spec 60), fixed on `main` after v0.2.7 and pending the click-test A4-45 to A4-48: TW-198 (F62 `208d0564`: eight built-in patterns drawn in code, an unknown pattern name falls back to the default, a failed inspector command no longer leaves the pointer stuck), TW-199 (F63 `bd397c68`: one cursor after focus loss and return), TW-200 (F64 `8a681109`: live canvas preview for the feather, softness, texture and text sliders, the slider pop-up flush with its arrow and the thumb under the press point) and TW-201 (F64 and F66 `08ef298d`: 17 measured text inspector tooltips; leading and paragraph spacing on one-line text are invisible in the reference too); FB-17: View ▸ Tooltips is off by default (F64); TW-198 and TW-199 are not yet checked on a real Mac. Fixed in the night of 22 Oct 2026 after measurement RE-27 (RE spec 59), confirmed by follow-up run 34 and pending the click-test A4-43 and A4-44: TW-195 (F61 `132b4941`: auto shapes take the tool attributes in full; the Arrow Line and the Measure tool draw their own strokes and set the tool; one History row "Shape") and the refined follow rules of TW-192 (F61 `132b4941`, `84346fec`: only a single path or text, fill none per document, the Default colours button keeps the tip, fresh-start stroke `bn_1-Pixel Anti-Aliased` 1 px centred); the runner item TW-196 is fixed by runner R-30 (`924fbfc7`: unit-start reset repeated after a reference restart) and confirmed by run 34; Q-63 is closed (runner R-31 `63f1e059` sets the tool stroke on both sides at unit start; cases c and d by F61). TW-185 (the own Tool Properties tab) was declined by the user on 21 Oct 2026. Fixed on the evening of 21 Oct 2026 after measurement RE-26 (RE spec 58), pending the click-test A4-38 to A4-42: TW-187 (F56 `f35997ca`: Subselection point drags start at 3·√zoom screen px), TW-186 (F57 `36874d34`: swap colours with a selection colours every selected object as one row), TW-194 (F57: the Marquee's Escape keeps the bitmap selected; was TW-04), TW-192 (F58 `5e9847e6`: script shapes take the tool attributes, which follow the selection; replaces Q-58) and TW-193 (F59 `33225630`: a right-click on a Layers object row selects it, Add Mask is its own command; replaces Q-59); full run 33 confirmed TW-193, TW-194 and TW-192 for the colours. Closed: Q-61 as measured (tag per tool, F57), Q-58 and Q-59 (replaced by TW-192 and TW-193), TW-175's open rest (grid and guide snapping of anchors measured and equal) and the runner item TW-174 (runner R-29 `6945bf79`: fixed reference panel layout per unit, `MaxUndos` guard; confirmed by full run 33). Closed by follow-up run 32: TW-189 (Enter on the canvas keeps the selection; fixed in F55 `f543d7b9` and confirmed, pending the click-test), the runner items TW-190 (View ▸ Tooltips off in Firepixel's shots) and TW-191 (no reference-only re-selection before Delete; both runner R-28 `4e1a35af`) and Q-60 (runner matter: the click hit the page heading; product evidence: the F55 UI test `q60_pref_category_click`). TW-188 closed by run 31 as a runner matter (Firepixel's "popup visible" was the swatch's own change box). TW-179 fixed by runner R-27 (`b61d8e01`; the plain-key problem was a bug in our reference host, not the key path). Fixed on 21 Oct 2026 pending the click-test: tester feedback batch 4 TW-180 to TW-184 with TW-175 (A4-33 to A4-37, released in v0.2.6) and the RE-24 rules TW-173, TW-176, TW-177, TW-178 (F53a `d1bb1d87`, F53b `e1340a21`, after v0.2.6; TW-173 confirmed by follow-up run 31, TW-176 and TW-177 not reached by the runner); batch 3 (TW-169 to TW-172, FB-07 to FB-11), TW-151 / FB-01, the macOS fixes TW-160 to TW-162 and TW-168 / FB-06 wait for the click-test (A4-15, A4-19 to A4-21, A4-23 to A4-27) and the macOS re-test; TW-146 rest and TW-165 are click-test item A1-19. 5 rows in `[V]`: open BA-101 (PSD stroke only). 20 informational `[N]` | [FINDINGS.md](FINDINGS.md) |
| Real differences known | Fixed on `main` after v0.2.10 (23 Oct 2026), pending follow-up run 38 and the click-test A4-60: F79 `7337428e` (TW-215: the Layers panel shows each layer's stored disclosure, its triangle runs `layer.setExpanded` as one History step, no panel state across documents; the opacity sliders and the colour pop-up's alpha slider run 2 px per per cent, so a 41 px drag gives 80 as in the reference, TW-205). Open after follow-up run 37: TW-208 (the Shift range across layers), TW-213 and TW-214 (text made by `addNewText`). Fixed on `main` after v0.2.9, pending the click-test A4-57: the measured TW-207 rest (RE spec 61 §1.5: the transform tools follow `XformToolXformsBFE` and the Numeric Transform dialog `NumericXformXformsBFE` for the fill handles, F74 `b585f6b8`; the scripting calls follow the option `transformAttributes`, F75 `2150bbf3`). Open: the unmeasured rests of TW-186, TW-193, TW-195 (gradient or pattern tool fills on auto shapes), TW-207 (the Pointer's own scale handles, `setSelectionBounds` / `resizeSelection`, the auto-trim of menu rotations) and Q-61, the unmeasured cases of RE spec 62 §5 (Q-62, Q-64) and TW-212 (a transform tool or numeric scale of a text: the reference changes the font size, Firepixel stores a matrix). Fixed on `main` after follow-up run 36, pending the click-test A4-58 and A4-59: the Q-62 rules re-measured with real keys (RE spec 62 §1, F76 `3cf23641`: transform mode Escape as a redo step, Brush and Marquee/Lasso Escape, merged marquee nudges) and gradient and pattern fills on text (Q-64, RE spec 62 §2, F77 `2b702176`). Deliberate deviations from tester feedback batch 6 (RE spec 61, 23 Oct 2026): Firepixel previews the Layers panel opacity and the colour pop-up alpha on the canvas (the reference only counts the field; TW-171 `[N]`), the colour pop-up always shows the base colour (the reference blends a reopened semi-transparent colour over white, a stroke over `#DFDFDF`), the stroke alpha writes one row (the reference two), and an indented auto-width text keeps its left and grows on the right (the reference moves its left). Fixed on 23 Oct 2026 after RE spec 61, pending the click-test A4-51 to A4-55: TW-204 to TW-208 (patterns move with the object, the indent on point text, Layers range selection and blank mixed fields). Before batch 6: TW-202 (RE spec 60 §1, RE-28: the reference draws a free path with the Pen and the Pointer as outline + anchors without a box, a primitive as outline + 4 corner squares, and has a "×" pen cursor for a new path; Firepixel draws the box; fixed by F65 `8c636395` on 22 Oct 2026, pending the click-test A4-49), the unmeasured rests of TW-186, TW-193, TW-195 (gradient or pattern tool fills on auto shapes) and Q-61 and the rules of Q-62 that still rest on distorted plain-key runs. Fixed on 22 Oct 2026 after RE spec 60, pending the click-test A4-47 and A4-48: the slider pop-up place (TW-200: flush with the arrow, thumb under the press point; Firepixel keeps it inside the window, the reference only inside the monitor) and the text inspector tooltips (TW-201); the six sliders update only the field in the reference, Firepixel's live canvas stays a deliberate enhancement (TW-171 `[N]`). Fixed in the night of 22 Oct 2026 after RE spec 59 (RE-27), confirmed by follow-up run 34 (the Arrow Line black on both sides, the tool state equal in every logged step) and pending the click-test A4-43 and A4-44: TW-195 (F61 `132b4941`: auto shapes take the tool attributes in full; the Arrow Line draws black `bn_Hard Line` 1 px and the Measure tool red `bn_Hard Line` 1 px inside, both set the tool; one History row "Shape") and the refined follow rules of TW-192 (F61: only a single path or text is followed, fill none per document with the path's stored fill colour, the Default colours button keeps tip, size and placement, fresh-start stroke `bn_1-Pixel Anti-Aliased` 1 px centred); Q-63 is closed (state carried by the runner, now set by runner R-31 at unit start, and the follow rules of F61). Fixed on the evening of 21 Oct 2026 after RE spec 58, pending the click-test (full run 33 confirmed TW-193, TW-194 and TW-192 for the colours; the others are not reached by the runner): TW-187 (F56 `f35997ca`: Subselection point drags start at 3·√zoom screen px), TW-186 (F57 `36874d34`: swap colours with a selection colours every selected object as one row), TW-194 (F57: the Marquee's Escape keeps the bitmap selected; was TW-04), TW-192 (F58 `5e9847e6`: script shapes take the tool attributes, which follow the selection; replaces Q-58) and TW-193 (F59 `33225630`: a right-click on a Layers object row selects it, Add Mask is its own command; replaces Q-59), and the View ▸ Tooltips tag per tool (Q-61). Fixed and confirmed by the runner: TW-173 (Ctrl+Y repeats the last step when nothing is undone; run 31) and TW-189 (Enter on the canvas keeps a selected group; run 32). Fixed on 21 Oct 2026, not reached by the runner: TW-176 (Pointer drag threshold 3·√zoom screen px and a zero-move row), TW-177 (Enter / Escape do nothing while the pen draws), TW-178 (baseline shift resets to `DefaultBaselineShift`; leading unit switch converts from the rounded field value), and from RE spec 57 the live retouch result, smudge growth past the bitmap edge, tip outline and cursor, inspector and toolbar tooltips. Known from RE spec 56, not measured by the runner: for a new text the reference writes two History rows ("Text" and "Text Change"), Firepixel one; the text tool's own opacity/blend reset is not done (Firepixel's text tool has no own opacity) | [PARITY.md](PARITY.md) |
| Closed findings (history) | 850+ | kept outside the repository ([ARCHIVE_NOTE.md](ARCHIVE_NOTE.md)) |
| Startup time | about 0.8 s to the first frame (release build, warm) | smoke test 17 Oct 2026 |

## Last test build

Release **v0.2.6** was published on 21 Oct 2026 as a pre-release from `02481356` (version bump `e0ad6271`, run
39540663545): Windows x64 ZIP, macOS universal ZIP and Linux x86_64 tar.gz and AppImage; the release notes thank the
tester for batch 4 and ask for the macOS re-test. Right after it `main` received the F53 fixes (`d1bb1d87`, `e1340a21`:
Ctrl+Y repeat, pen Enter / Escape, Pointer drag threshold, text reset values; GT-1k 2892 passed, 0 failed, 141 ignored).

Latest test build: preview build **`2026-10-21_05-40`** (release profile, `firepixel.exe` plus `firepixel-cli.exe`) from
`main` at `82b0b5b2`: tester feedback batch 4 (F54a to F54d: Snap to Pixels drives the Subselection drags and is off by default,
no smart-guide snap for anchors; one brush tip outline instead of one per stroke point; live result for all retouch
tools; smudge grows the bitmap past its edge with a "Confine to boundaries" option; tooltips as measured, tool
descriptions, the View ▸ Tooltips coordinate tag). The full workspace test run on its code (GT-1j) gave 2877 passed, 1 failed, 141 ignored (the `fp-ui` lib process crashed once with an access violation, BA-117, and passed its 144 tests on the rerun; the one failure was the untranslated `confineToBounds` parameter text, fixed in `1b95dd37`).
It is the code of pre-release v0.2.6. Before it: preview build **`2026-10-20_19-06`** (release profile, `firepixel.exe` plus `firepixel-cli.exe`) from
`main` at `3b7386fa` (version 0.2.5 plus JPEG 2000 in the PDF export, BA-101, and the AI batch steps `ai.upscale` and
`ai.faceBlur`, BA-109). The full workspace test run on its code (GT-1i) gave 2852 passed, 1 failed, 141 ignored; the
failure was the hardcoded-text scanner over the new `fp-ai` error texts, translated in `8ab8564d` right after the build
(the preview carries the English texts for those eleven internal weight-file errors, nothing a user can reach). It is
the build for full parity run 30. Before it: preview `2026-10-20_09-25` from `612b34bd` (GT-1h), the code of release
v0.2.5.

Release **v0.2.5** was published on 20 Oct 2026 as a pre-release from `30da1158` (version bump `0af5ffc6`, run
39412818493): Windows x64 ZIP, macOS universal ZIP and Linux x86_64 tar.gz and AppImage; the release notes thank the
tester and ask for the macOS re-test. Release **v0.2.4** was published on 19 Oct 2026 as a pre-release (tag `v0.2.4` at `1f26223c`, the version bump on top
of `aaa9b3b5`): Windows x64 ZIP, macOS universal ZIP and Linux x86_64 tar.gz and AppImage. It contains package F49 (the
three screen modes as in the reference, TW-158; the text default rules, TW-138 rest) and the macOS fixes TW-160 to
TW-162 with Firepixel's own macOS app menu (`e11f8d1b`); the Mac checks wait for the tester's re-test (A4-19 to
A4-21). Release **v0.2.3** was published on 19 Oct 2026 from the state of preview `2026-10-19_06-06` (`main` at
`44d7c5f5` plus documentation): Windows x64 ZIP, macOS universal ZIP and Linux x86_64 tar.gz and AppImage, all
assets verified after publishing, with thanks to the tester in the release notes. The previous release was **v0.2.2** (18 Oct 2026, pre-release on GitHub, tag from
`c031a987`, version bump `2cfff08d`): Windows x64 ZIP, macOS universal ZIP (arm64 + x86_64) and Linux x86_64 tar.gz
and AppImage. All assets were verified after publishing (fat binary, CLI version, archive contents). The Linux build
is tested in CI only, the Intel macOS slice by tools only (not run on Intel hardware). Earlier releases were v0.2.1
(18 Oct 2026, Windows x64, macOS universal) and v0.2.0 (17 Oct 2026, Windows x64, macOS arm64).

**Linux CI:** `prerelease.yml` now also calls `linux-build.yml`, which builds Linux x86_64 on Ubuntu 22.04 (glibc
2.35 baseline): release build of `firepixel` and `firepixel-cli`, a tar.gz (manuals, licence, `.desktop` file,
icons) and an AppImage (pinned, checksum-verified `appimagetool`). TLS for the model download uses rustls on all
non-Windows targets, so the binaries do not link OpenSSL (the build fails if `ldd` lists it). A startup probe runs
the app under Xvfb and checks that the control channel comes up. The first CI runs are green. Not yet tested on a
real Linux desktop; file dialogs need `xdg-desktop-portal`. v0.2.2 is the first release with Windows x64, macOS
universal and Linux x86_64.

Earlier test build: preview build **`2026-10-19_16-07`** (release profile, single `firepixel.exe` plus
`firepixel-cli.exe`) from `main` at `37c0ee81` (package F49: the three screen modes as in the reference, full screen
shows only the canvas on black; text preference rules, TW-138 rest; runner R-23). The full workspace test run on its
code (`439a3ec5`) gave 2776 passed, 0 failed, 134 ignored. Follow-up runs 26 and 27 used this build. Since then `main`
has the macOS app menu (`e11f8d1b`, FB-05; the macOS CI build with it is green, released in v0.2.4) and runner R-24
(`f7d13d61`); the workspace tests are unchanged. Before that came preview `2026-10-19_12-16`
from `a225a01c` (package F48: consecutive key nudges merge into one History row, crisp Select Behind hover line
without a frame right after a click; runner R-22; 2762 passed, 0 failed, 134 ignored), used by follow-up run 25.
Before that came preview
`2026-10-19_06-06` from `44d7c5f5` (packages F46 and F47; 2761 passed, 0 failed, 134 ignored), used by full run 24
and published as v0.2.3. Before that came preview `2026-10-19_01-32` from
`cd86ce75` (package F46: Escape keeps the selection and only commits text, cancels crop or transform or leaves bitmap
mode; one History row per arrow-key nudge), used by run 23. Before that came preview `2026-10-18_23-25`
from `a4c2bf2b` (package F45: element-list read-backs see the live text element while editing, TW-143). The
preview before was `2026-10-18_22-05` from `65db4012` (packages F42–F44: panel pop-up menu state, live bounds of an
edited text, stretched alignment stored as left, text attribute commands writing the text tool defaults, paragraph
separator tests).

Full comparison run 17 (18 Oct 2026, 04:59–09:40, all 48 units, 1284 steps, 683 image pairs) used the earlier
preview build `2026-10-18_04-57` from `main` at `7e3d2d12` (fixes after run 16, runner R-12): 42 units equal except
for design, 1 real deviation, 5 unclear; step level 86.3 %; the animated GIF export is byte-identical again.

Follow-up run 18 (18 Oct 2026, 11:00–11:54, 10 units, 252 steps, 126 image pairs) used preview `2026-10-18_10-59`
and runner R-13/R-13b (the "Web Standard" set in both programs): 5 of the 10 units equal except for design, 2 real
deviations (`panel-library-styles`: URL panel menu, `xs-shortcuts`: text shortcuts), 3 unclear (`menu-edit`,
`xs-preferences`, `tool-text`); TW-125 to TW-129 closed.

Follow-up run 19 (18 Oct 2026, 13:04–13:40, 6 units, 149 steps, 66 image pairs) used preview `2026-10-18_13-04` and
runner R-14: `panel-library-styles` is now equal except for design (URL panel menu), `menu-edit` became a real
deviation (no Copy row in History, TW-137), `xs-shortcuts` stays a real deviation for one shortcut (TW-133 rest),
`tool-text` stays unclear (reference-side text-tool state), `xs-canvas` and `panel-pages-states-history` stay equal
(States menu 210 vs. 207 px). Step quota and canvas pairs unchanged; TW-125, TW-130 / Q-57, TW-131, TW-132 and
TW-135 closed, TW-136 to TW-141 added.

After run 19 (on `main` up to `2d64547a`, not yet in a preview build): TW-136 closed as no deviation (the labels
already matched, the shot included the ≡ row and the shadow; 323 × 231 vs. 321 × 231 px), TW-137 (History rows for
Copy, Cut, Select All and Deselect), TW-138 (text tool defaults) and TW-139 (live read-back while editing) fixed,
runner R-15 for TW-140; TW-141 needs a Rust change (F40, in progress). Run 20 is to confirm.

Follow-up run 20 (18 Oct 2026, 15:30–16:09, 7 units, 170 steps, 78 image pairs) used preview `2026-10-18_15-28` and
runner R-15: `menu-edit` is now equal except for design (Copy row in both Histories, TW-137), `xs-shortcuts` stays a
real deviation for one shortcut (TW-133 rest), `tool-text` and `xs-preferences` stay unclear (reference-side runner
effects: text attributes carried between units, the stray "c", the crop not applied on the reference side),
`xs-canvas`, `panel-library-styles` and `panel-pages-states-history` stay equal. Step quota unchanged, one more
canvas pair within 1 % (`tool-text/14`); TW-137, TW-138, TW-139 and TW-141 closed; TW-140 stays open (R-15's
`key_echo` failed with a script error, fixed in R-16), TW-142 to TW-144 added.

Follow-up run 21 (18 Oct 2026, 22:06–22:51, 7 units, 170 steps, 78 image pairs) used preview `2026-10-18_22-05` and
runner R-17 / R-17b / R-18: no class changed (45 / 1 / 2 / 0). `xs-shortcuts` stays a real deviation, now for the
live bounds of an edited text (TW-143 not confirmed) after Ctrl+Alt+Shift+S became equal (TW-133 closed); the URL
panel menu reads the same enabled states and "..." flags on both sides (TW-142 closed) and both programs separate
paragraphs with `\r` (TW-144 closed). Step quota unchanged, one more canvas pair within 1 % (`tool-text/02`). The new
key lines show that the reference takes plain Delete, Right, Down and Escape as tool letters although the keys
arrive correctly and the document view has focus (TW-140, runner R-19); the crop option (TW-134) and the size of a
new text after a partial-selection edit (TW-145, new) remain uncompared.

Follow-up run 22 (18/19 Oct 2026, 23:27–00:10, 7 units, 170 steps, 77 image pairs) used preview `2026-10-18_23-25`
and runner R-19: two classes changed, 47 / 0 / 1 / 0. `xs-shortcuts` is now equal except for design (the live bounds
of an edited text read 50,150–120,194 in both programs, TW-143 closed; the nudges are equal), `xs-preferences` too
(R-19's double-click commits the reference's crop: 280 × 220 and the same four paths in both programs, TW-134
closed). R-19's substitutes for Delete, nudges, Escape and the crop commit work on the reference side (TW-140
closed). `tool-text` stays unclear: on the reference side texts created by a click stay empty and the drag selection
fails (TW-146, runner R-20), so the size rule TW-145 is still not compared. Step quota unchanged (138 / 159), one
canvas pair fewer (`tool-text/13` without reference shot); further runner side effects as TW-147.

Follow-up run 23 (19 Oct 2026, 01:34–02:10, 5 units, 123 steps, 49 image pairs) used preview `2026-10-19_01-32`
and runner R-20: one class changed, 46 / 0 / 2 / 0. `panel-pages-states-history` became unclear: with real
Shift+arrows the reference reads 4 History steps after three nudges and a nudge of a second object, Firepixel 5
(TW-148; the count points to merged nudges in the reference, against RE spec 51 and F46, but the row list is not
read). `tool-text` stays unclear: R-20 creates 14 of 17 reference texts as typed and verifies drag selections by
highlight pixels, but the size edit on "123" does not take effect, so TW-145 is still not compared (TW-146 rest).
Confirmed and closed (TW-147): Escape keeps the selection in both programs, the Pointer tool is reached through the
flyout, the paste read-back and `MaxUndos` (20) are equal. Step quota 1065 / 1226 (`xs-shortcuts/08` over the 90 s
step limit, TW-150), canvas pairs unchanged; new runner items TW-149 (text tool font) and TW-150.

After run 23 (on `main` up to `44d7c5f5`, preview `2026-10-19_06-06`): runner R-21 (`b1916aa6`) reads the History row
labels on both sides, checks for a caret before typing on the reference side, resets Firepixel's text tool at unit
start to the reference's font, size, style and colour, allows 2 s more per substitute and fixes the console
code-page error in the log; run 24 confirmed TW-149 and TW-150, decided TW-148 from the logged
row labels and left TW-146 rest open (see below).

**Tester feedback (19 Oct 2026, E-Man, credited in the README):** two reports, both written down as RE spec 52 and
fixed. FB-01 (TW-151, `c3a9e264`): with the pen tool, clicking the last anchor while drawing now retracts its outgoing
handle, so the next segment leaves the point straight; a drag re-pulls it as a symmetric curve point (Alt, Ctrl and
Shift variants), interior points are converted or deleted, a segment click inserts a point, another open path is
joined or an open path continued, with matching cursors; the hit radius follows the snap distance. Limits: a join
across elements appends instead of merging, no drag after a join, a close or an interior press, a double-click still
ends the path. FB-02 (TW-152, `44d7c5f5`): Select Behind shows a hover frame (#E00000, 1 px, filled 5 × 5 px corner
squares) on the object a click would select, also while the pointer is over the selected object; with nothing
selected it frames the topmost object; the Pointer tool is unchanged. The rule was measured in the reference
application with the new parity unit `xs-select-behind` (`c7515986`, 9 steps, the 49th unit). Open: whether the
reference draws the blue selection frame together with the red one (its runner shots never show a selection
outline) and one preview-build case without a frame with nothing selected that tests do not reproduce; both are in
the click-test ([VERIFICATION.md](VERIFICATION.md) A4-15 to A4-17).

**Tester feedback, second batch (19 Oct 2026, macOS):** three reports, fixed directly on `main`, all only
type-checked for macOS (no Mac at hand). FB-04 (TW-160, `b9f085f4`): after the app's own minimise and a restore from
the Dock the window stayed frozen, because the minimised flag was never refreshed; the stale flag is now healed
against the real window state. FB-05 (TW-161, `0352e7c9`): Cmd+Q quit through the default app menu without the save
prompt; the default menu is off and Cmd+Q goes through the Quit action with the prompt; since `e11f8d1b` the app has
its own macOS app menu (About, Services, Hide, Hide Others, Show All and Quit Firepixel with the save prompt), and the
macOS CI build with it is green. FB-03 (TW-162, `53e08484`): native title bar with traffic lights on
macOS, without Firepixel's own window buttons. Hands-on checks on a Mac: [VERIFICATION.md](VERIFICATION.md) A4-19
to A4-21.

**Tester feedback, third report (20 Oct 2026, E-Man):** FB-06 (TW-168, `0b71e6ca`, fixed directly on `main`): with
the Subselection tool a selected vector object showed the bounding box like the Pointer tool. Now paths, rectangle
primitives and autoshape children show the path outline through the anchors, hollow 5 × 5 px anchor squares (filled
when selected; *Show solid points* reverses this) and the control handles of every segment with a selected end,
without a bounding box; bitmaps, text and symbol instances keep the box, and the Pointer and Select Behind tools are
unchanged. Known limits: a whole autoshape or group selected as one element and compound shapes keep the box; the
facing handles of the neighbouring points are shown but cannot be dragged. The runner never captures the
reference's selection markings, so the comparison is the hands-on check [VERIFICATION.md](VERIFICATION.md) A4-23.

**Tester feedback, batch 3 (20 Oct 2026, E-Man):** FB-07 to FB-11, measured in the reference as RE spec 55 and fixed
in three commits. F51a (`22f62a54`): every Pen point is its own History step "Pen", Ctrl+Z while drawing removes only
the last point and drawing goes on from the new last point, finishing adds no step, the closing click is one step,
Vector Path ignores undo/redo while the button is held (TW-169 / FB-07); Redo is redo only, with "Can't Redo" when
nothing is undone and no repeat-last-command fallback (TW-170 / FB-08). F51b (`0932f67c`): the inspector opacity and
stroke width sliders apply once, on release or when the pop-up closes, the number fields on Enter, Tab or a click into
the document; deliberately beyond the reference, the canvas shows a live render-only preview while dragging, and an
unchanged value makes no step (TW-171 / FB-09, FB-10). F51c (`901f0a56`): Subselection anchor and handle drags are
free (no whole-pixel rounding), snapping only to grid, guides and smart guides; the new Firepixel-own toggle View ▸
Snap to Pixels (on by default) controls the whole-pixel rounding of Pointer moves and Pen points; a hollow
arrowhead cursor over points (TW-172 / FB-11). Tests on the merged tree (GT-1g): 2797 passed, 0 failed, 134 ignored at
`901f0a56`; preview `2026-10-20_06-08`. Hands-on checks: [VERIFICATION.md](VERIFICATION.md)
A4-24 (including the re-check of Enter in the opacity field, not reproducible in tests) to A4-27.

After run 25 (on `main` up to `439a3ec5`): runner R-23 (`5f498cd7`) for TW-159 and TW-146 rest; package F49
(`8a9cf670`, `439a3ec5`) with the three screen modes as in the reference (TW-158: full screen shows only the canvas
on black, full screen with menus keeps menu bar and toolbar with a floating Tools panel and an icon dock, F needs a
document, back to Standard when the last document closes) and the text preference rules of TW-138 (the inspector
never writes the baseline shift default, font changes store the PostScript name and the recent font list, a style
applied to text writes the text defaults). The full test run on `439a3ec5` gave 2776 passed, 0 failed, 134 ignored;
preview `2026-10-19_16-07` (from `37c0ee81`, the same code plus documentation) was built for run 26.

**Full run 24 (19 Oct 2026, 06:09–11:10, all 49 units, 1295 steps, 698 image pairs)** used preview
`2026-10-19_06-06` and runner R-21: 46 / 2 / 1 / 0, proven 1:1 93.9 % (46 / 49; 95.8 % over the 48 old units),
quota 87.0 % (86.9 % over the 48 old units, unchanged in 46 of them). It decided TW-148: the History row labels show
that the reference merges three Shift+arrow presses on one selection into a single row (`moveSelectionBy({x:20,
y:10})`), so `panel-pages-states-history` is a real deviation and F46's one row per press has to go back to merging.
The first run of `xs-select-behind` confirms the hover rule (TW-152 closed: the same #E00000 frame in both programs)
but shows two smaller differences (TW-157: Firepixel's hover line on a vector path is anti-aliased over two pixel rows,
and Firepixel shows the frame right after a click). Closed: TW-149 (Firepixel's text tool starts at Arial 12 pt like
the reference) and TW-150 (`xs-shortcuts/08` within the new limit). `tool-text` stays unclear: the caret check passes,
but the reference still loses the keys of 4 of 16 new texts (TW-146 rest), so TW-145 is not compared. No regression
from F47 in the pen-related units. New runner items: TW-153 (the reference's unit-start text reset sets its fill to
#000000; `tool-autoshapes` canvas 7 / 27), TW-154 (Text Editor and Edit Guides have no key in the "Web Standard" set),
TW-155 (plain arrow keys outside nudges, History read-back with two documents) and TW-156 (screen-mode steps without
a defined start mode).

After run 24 (on `main` up to `ace7873c`, not yet in a preview build): package F48 fixes TW-148 (`0419534c`:
consecutive arrow-key nudges of an unchanged selection merge into one Move row with the summed offset; a selection
change, another command or an undo starts a new row; script moves stay one row each; Alt+arrow creates the copy on
the first press and merges further presses into that row; this corrects F46) and TW-157 (`ace7873c`: crisp
pixel-aligned hover line, no frame right after a Select Behind click until the pointer moves). Runner R-22
(`72c695dd`) fixes TW-153 (the text reset no longer touches the fill colour, default colours restored and read
back), closes TW-154 as not comparable (Text Editor and Edit Guides have no menu entry and no key in the reference;
Firepixel opens them by command, the reference side is unsupported), fixes TW-155 (arrow substitutes only with the
Pointer or Subselection tool or `nudge: true`; `inspector-stroke/01` hovers instead of using arrows; `menu-edit`
closes extra documents) and TW-156 (standard screen mode at the start and end of `toolbox/13`), and retries the
reference-side text entry (TW-146 rest: first character alone and read back, 35 ms per character). The TW-149
leftover (a bridge error with the Web/locked layer active) is handled by reading the scratch text back from other
layers. Run 25 over the R-22 units is to confirm all of them.

**Follow-up run 25 (19 Oct 2026, 12:19–14:39, 20 units, 556 steps, 317 image pairs)** used preview
`2026-10-19_12-16` and runner R-22: 48 / 1 / 0 / 0 over all 49 units, proven 1:1 98.0 % (48 / 49), quota 87.3 %
(1075 / 1232), canvas pairs 186 / 211. Confirmed and closed: TW-148 (one History row per selection for consecutive key
nudges in both programs), TW-157 (680 px #E00000 hover line on FR in both, no frame right after a click), TW-153
(`tool-autoshapes` 26 / 27), TW-154, TW-155, TW-156 and the TW-149 leftover; TW-145 is decided (the next text after
the size edit is 72 pt in both programs). `panel-pages-states-history`, `xs-select-behind` and `tool-text` are now
equal except for design. New: `toolbox` is a real deviation because Firepixel's full screen mode keeps the menu bar,
the document tab and the status bar while the reference shows only the document on black (TW-158, behaviour
description first); runner rests TW-159 (screen-mode read-back, a real arrow with the Subselection tool in
`xs-shortcuts/10_nudge_point`, the Freeform size keys, a 24 pt text size after one reset). TW-146 rest is down to one
lost text in 17.

**Follow-up run 26 (19 Oct 2026, 16:11–16:55, 5 units, 155 steps, 72 image pairs)** used preview
`2026-10-19_16-07` and runner R-23 over `toolbox`, `xs-shortcuts`, `tool-path-reshape`, `tool-text` and `menu-text`:
49 / 0 / 0 / 0 over all 49 units, proven 1:1 100.0 % (49 / 49), quota 87.3 % (1077 / 1233), canvas pairs 186 / 211.
Confirmed and closed: TW-158 (after the second F both programs show only the canvas on black, `toolbox` is equal except
for design again) and TW-159 (b) and (d) (the Subselection nudge reads 201,210 in both; the reference's text reset goes
from 72 pt to 12 pt). The entry part of TW-146 rest is confirmed (all 15 new texts and three edits arrive as typed).
Open, all runner: TW-146 rest (text selections in `tool-text/11, 13`), TW-159 rest (the Freeform size keys keep the
tool but record no size), new TW-163 (the reference's screen-mode button reads "full screen with menus" in Standard)
and TW-164 (the reference shows the Path Scrubber head in `tool-path-reshape/12_inspector_heads_freeform`).

**Follow-up run 27 (19 Oct 2026, 20:49–21:18, 3 units, 106 steps, 62 image pairs)** used the same preview
`2026-10-19_16-07` and runner R-24 (`f7d13d61`) over `tool-text`, `tool-path-reshape` and `toolbox`: 49 / 0 / 0 / 0
over all 49 units, proven 1:1 100.0 % (49 / 49), quota 87.3 % (1077 / 1233; the three quotas unchanged), canvas pairs
185 / 212. Confirmed and closed: TW-159 rest (with the mouse held both programs keep the Freeform tool and read size 72
after the size keys), TW-164 (the reference shows the Freeform head in `12_inspector_heads_freeform`) and TW-163 (the
reference's Standard mode is read from the window; one `runner error` instead of six). TW-146 rest stays open: none of
the three size routes (field, keys, Text Editor) reaches the selected "123" in the reference. New, both runner: TW-165
(only the reference's text tool is reset after the failed size step, so `tool-text/14, 16` compare 12 pt with 72 pt
text and move above 1 %) and TW-166 (the Standard reset after full screen finds neither the screen-mode button nor
the menu bar).

**Follow-up run 28 (19 Oct 2026, 21:57–22:21, 2 units, 83 steps, 42 image pairs)** used the same preview
`2026-10-19_16-07` and runner R-25 (`74d6bfc8`) over `tool-text` and `toolbox`: 49 / 0 / 0 / 0 over all 49 units,
proven 1:1 100.0 % (49 / 49), quota 87.3 % (1077 / 1233; both quotas unchanged), canvas pairs 185 / 212 (unchanged).
Confirmed and closed: TW-166 (the reference leaves full screen by F; no `runner error` in `toolbox`). Open, all
runner: TW-146 rest (no selection try reaches the expected highlight, so the size routes are skipped; 7 `runner error`
lines), TW-165 (the two-sided reset is not armed when every size route is skipped) and new TW-167 (the reference's
panel dock started collapsed to icons and is taken as the reference width; after the screen modes it is expanded and
not restored).

**Follow-up run 29 (20 Oct 2026, 06:17–06:43, 3 units, 88 steps, 54 image pairs)** used preview `2026-10-20_06-08`
(`main` at `901f0a56`) and runner R-26 over `toolbox`, `tool-path-reshape` and `xs-select-behind`: 49 / 0 / 0 / 0,
proven 1:1 100.0 %, quota 87.3 % (1077 / 1233), canvas pairs 185 / 212, all unchanged; 85 of 88 steps `ok` on both
sides, no `runner error`. Confirmed and closed: TW-167 (the layout check expands the reference's collapsed panel dock
at unit start and after the screen modes; overview 25.5 %, Standard 25.9 %).

**Full run 30 (20/21 Oct 2026, 19:22–00:40, 49 units, 1295 steps, 699 image pairs)** used preview
`2026-10-20_19-06` (`main` at `3b7386fa`: v0.2.5 plus JPEG 2000 in the PDF export and the AI batch steps) and runner
R-26: 48 / 1 / 0 / 0, proven 1:1 98.0 % (48 / 49), quota 87.2 % (1075 / 1233), canvas pairs 185 / 212; 36
reference-side and 21 Firepixel-side errors (the same 21 steps and texts as before), 11 `runner error` lines in
`tool-text`. New: TW-173 (real deviation, `menu-edit/02`: the reference repeats a clone with Ctrl+Y when nothing was
undone, Firepixel no longer since F51a), the runner item TW-174 (the reference's panel sizes) and the questions Q-58
and Q-59. Equal now: the redo cases of TW-170 (`xs-shortcuts/02, 17`) and the Layers context menu `03`. The tiled
canvas of BA-098 shows no sampling offset at fractional zoom. JPEG 2000 in PDF (BA-101) is not exercised by the runner
(`export-results/10` is `unsupported` on both sides).

**Follow-up run 31 (21 Oct 2026, 06:28–07:30, 8 units, 229 steps, 110 image pairs)** used preview
`2026-10-21_06-27` (`main` at `e1340a21`: v0.2.6 plus the F53 fixes) and runner R-27 (real plain keys on the
reference side by scan code, no R-19 substitutes) over `menu-edit`, `xs-shortcuts`, `tool-path-reshape`, `toolbox`,
`xs-canvas`, `tool-text`, `panel-pages-states-history` and `xs-preferences`: 47 / 1 / 1 / 0, proven 1:1 95.9 %
(47 / 49), quota 87.2 % (1075 / 1233), canvas pairs 185 / 212, both unchanged; no step status changed; 36
reference-side and 21 Firepixel-side errors as before; 10 `runner error` lines in `tool-text`. Confirmed: TW-173
(`menu-edit/02` three R1 in both programs, `menu-edit` equal again) and TW-189 (`xs-shortcuts/22`: the reference keeps
a selected group after Enter, Firepixel drops it; `xs-shortcuts` real deviation). Closed: TW-188 (runner matter).
New: the runner items TW-190 (Firepixel's View ▸ Tooltips coordinate tag and flyout description tooltips appear in
its shots) and TW-191 (the key check re-selects before Delete on the reference side only), and the questions Q-60
(`xs-preferences/20`: after a click on a category the reference's list moves with Down, Firepixel's does not; unit
unclear) and Q-61. TW-176 and TW-177 are not reached by any step.

**Follow-up run 32 (21 Oct 2026, 08:22–08:50, 3 units, 103 steps, 41 image pairs)** used preview
`2026-10-21_08-16` (`main` at `f543d7b9`: v0.2.6 plus the F53 fixes and F55, TW-189 and the Q-60 UI test; workspace
tests GT-1l: 2899 passed, 0 failed, 141 ignored) and runner R-28 (`4e1a35af`) over `xs-shortcuts`, `xs-preferences` and
`toolbox`: 49 / 0 / 0 / 0, proven 1:1 100.0 % (49 / 49), quota 87.2 % (1075 / 1233), canvas pairs 185 / 212, both
unchanged; no step status changed (92 `ok`, 8 `unmapped` and 2 `unsupported` on both sides, plus the Firepixel-only
step); 36 reference-side and 21 Firepixel-side errors as before; 2 `runner error` lines (reference-side text entry in
`xs-shortcuts/15`). Confirmed: TW-189 (`xs-shortcuts/22`: `[object Group]` on both sides; `xs-shortcuts` equal again).
Closed: TW-190 (Firepixel's View ▸ Tooltips off at unit start, its canvas shots pixel-identical to run 30, flyout shots
without a tooltip), TW-191 (the Delete with nothing selected removes nothing on either side, 9 elements in both) and
Q-60 (runner matter: the click hit the page heading; with the list row both category lists move to "Bearbeitung";
`xs-preferences` equal again). No new finding.

**Evening of 21 Oct 2026 (no run):** measurement RE-26 (RE spec 58) answered TW-186, Q-58, Q-59, Q-61 and the rest of
TW-175 and re-measured TW-04 with the real Escape. Fixed on `main` up to `5e9847e6`: F56 (TW-187), F57 (TW-186,
TW-194, the Q-61 tag), F58 (TW-192 from Q-58) and F59 (TW-193 from Q-59); runner R-29 (`6945bf79`) fixes TW-174. DOC-27
(`35bc5116`) traced the TW-179 host bug back to the first runner version; the rules still resting on it are Q-62.
Interim workspace test run on `36874d34`: 2918 passed, 0 failed, 141 ignored (GT-1m-pre). Full run GT-1m on `5e9847e6`: 2929 passed, 2 failed, 141 ignored (one timing flake, one F57/F58 interaction fixed by F60 `6ebdc374`). Preview `2026-10-21_21-14` was built from `5e9847e6`; the preview for run 33 is built from `6ebdc374`.

**Full run 33 (21/22 Oct 2026, 21:48–03:04, 49 units, 1295 steps, 699 image pairs)** used preview
`2026-10-21_21-47` (`main` at `6ebdc374`: v0.2.6 plus F53, F55 and F56 to F60) and runner R-29 (`main` at `01dc22a0`:
real plain keys on the reference side in every unit for the first time, the reference's dock in icon mode with fixed
flyout sizes for the panel units, `MaxUndos` held at 20): 48 / 1 / 0 / 0, proven 1:1 98.0 % (48 / 49), quota 87.1 %
(1075 / 1234), canvas pairs 184 / 209; 36 reference-side and 21 Firepixel-side errors (no Firepixel-side step status
changed), 4 `runner error` lines in `tool-text`. Confirmed: TW-193 (`panel-layers-menu/04` equal), TW-194
(`menu-modify-a/22`: B1 stays selected after the real Escape in both programs), TW-192 for the colours
(`menu-modify-a/20` 6.32 → 0.17 %) and the runner item TW-174 (`panel-layers/04` and `panel-swatches-mixer/19` equal,
the panel pairs at the run 24/25 values). New: TW-195 (real deviation, `tool-autoshapes`: the Arrow Line's stroke),
the runner item TW-196 (the reference's state after a restart in the middle of a unit) and the question Q-63 (tool
stroke attributes carried between steps and units). The reference was restarted seven times (three planned, four after
a script access violation), without lost steps.

**Night of 22 Oct 2026 (no run):** measurement RE-27 (RE spec 59) answered TW-195 and Q-63 and refined the follow
rules of TW-192. Runner R-30 (`924fbfc7`) repeats the unit-start reset after a reference restart inside a unit and
reopens the test file (TW-196); runner R-31 (`63f1e059`) sets the tool stroke on both sides at unit start
(`bn_1-Pixel Anti-Aliased` 1 px centred, fill solid; verified on `xs-select-behind`: FR 400,200–600,300 on both
sides). F61 (`132b4941`, `84346fec`) fixes TW-195 (auto shapes take the tool attributes; the Arrow Line and the Measure
tool draw their own strokes and set the tool; one History row "Shape") and the follow rules of TW-192 (only a single
path or text; fill none per document; the Default colours button keeps the tip; fresh-start stroke
`bn_1-Pixel Anti-Aliased` 1 px centred, the FIX-G5 size decision stands). Q-63 is closed. GT-1n on `84346fec` (F61):
2948 passed, 0 failed, 141 ignored; preview `2026-10-22_05-02` is built from it.

**Follow-up run 34 (22 Oct 2026, 05:06–06:10, 8 units, 234 steps, 142 image pairs)** used preview `2026-10-22_05-02`
(`main` at `84346fec`: run 33's code plus F61) and runner R-31 (`main` at `5fbd4007`: the same tool stroke on both
sides at unit start, with R-30's repeated reset after a reference restart): 49 / 0 / 0 / 0, proven 1:1 100.0 %
(49 / 49), quota 87.0 % (1074 / 1234), canvas pairs 185 / 209; 37 reference-side and 21 Firepixel-side errors (one
new reference-side error, `tool-autoshapes/10`, by a focus loss), no `runner error` line. Confirmed: TW-195
(`tool-autoshapes`: the Arrow Line black on both sides, the tool state equal in all 37 logged steps,
`05_handle_drag_rounded_corner_result` 1.01 → 0.97 %; the unit is equal again), the refined follow rules of TW-192
(`tool-path-reshape/02` to `07`, `tool-web/12` to `15`), R-31 (the same tool stroke at all eight unit starts; FR of
`xs-select-behind` 400,200–600,300 on both sides) and the runner item TW-196 (one reference restart in `toolbox`, the
unit-start reset repeated and logged; `xs-canvas` and `xs-windows-tabs` back at their run 30/31 window values). New:
the runner item TW-197 (the reference's Properties inspector collapsed until the restart; inspector and overview pairs
only).

**Follow-up run 35 (22 Oct 2026, 14:31–15:15, 6 units, 178 steps, 107 image pairs)** used preview `2026-10-22_14-27`
(`main` at `8c636395`: run 34's code plus tester feedback batch 5, F62 to F66) and runner R-31 as in run 34 over
`tool-path-reshape`, `inspector-text`, `inspector-fields`, `xs-select-behind`, `toolbox` and `color-popup`: 49 / 0 / 0
/ 0, proven 1:1 100.0 % (49 / 49), quota 87.0 % (1074 / 1234), canvas pairs 185 / 209; 37 reference-side and
21 Firepixel-side errors, no step status change, no `runner error` line, one planned reference restart. No compared
value changed. TW-202 (F65) shows in the Pointer shots of a selected rectangle (`xs-select-behind/01, 07`,
`color-popup/00, 01, 18, 20`: outline + four 5 × 5 px corner squares instead of the box, 15 px, pairs unchanged); the
`tool-path-reshape` shots, taken under the Freeform, Reshape Area, Path Scrubber and Knife tools, keep the box and are
pixel-identical to run 34, and no step uses the Pen. TW-200 (F64) shows in the slider pop-ups
(`inspector-fields/14_popup_drag_live_canvas_pressed` 2.03 → 1.58 %, still above 1 % by TW-171), TW-201 (F66) in the
width field's tooltip of `inspector-fields/24`, and View ▸ Tooltips is off at every Firepixel start. TW-198 and TW-199
are not reached. The runner item TW-197 did not recur (`tool-path-reshape` inspector pairs 9.7–10.6 %, `toolbox/00`
25.9 %).

**Follow-up run 36 (23 Oct 2026, 07:56–08:57, 9 units, 218 steps, 128 image pairs)** used preview `2026-10-23_07-49`
(`main` at `62efd30b`, the code of v0.2.9: run 35's code plus F67 and tester feedback batch 6, F68 to F72) and runner
R-31 as in runs 34 and 35 over `panel-layers`, `panel-layers-menu`, `color-popup`, `color-popup-contexts`, `inspector-text`, `inspector-fields`, `fx-bevel-emboss-fill`, `tool-text` and `xs-select-behind`: 49 / 0 / 0 / 0, proven 1:1 100.0 % (49 / 49), quota 87.3 %
(1077 / 1234), canvas pairs 186 / 212; 34 reference-side and 21 Firepixel-side errors; two reference restarts (one
planned, one after a script access violation, with R-30's repeated reset); 7 `runner error` lines in `tool-text`
(TW-146 rest). No class, no Firepixel-side step status and no Firepixel-side value changed; the three reference-side
errors new in run 33 (`fx-bevel-emboss-fill/09`, `tool-text/11`, `13`) are `ok` again. None of the batch 6 interactions
(TW-204 to TW-208) is reached by a step. New: the runner items TW-210 (Firepixel's Layers panel is shorter than the
reference's flyout; the Layers object-row steps have been `unmapped` on Firepixel's side since run 24) and TW-211 (the
reference's stored text alignment at unit start) and the question Q-65 (the current layer after opening T07).

**Follow-up run 37 (23 Oct 2026, 11:17–12:23, 9 units, 228 steps, 138 image pairs)** used preview `2026-10-23_11-02`
(`main` at `8d1e3a77`, the code of v0.2.10: run 36's code plus F73 to F78; without F79) and runner R-32 (`f05236b5`:
the reference's inspector expanded at unit start, TW-197; collapsed parent layers expanded by the Layers row lookup,
TW-210; left alignment in the text tool reset, TW-211; the reference's `set_field` types first and `popup.hex_field`
is the HEX field; ten new steps for tester feedback batch 6) over the same nine units: 47 / 2 / 0 / 0, proven 1:1
95.9 % (47 / 49), quota 87.8 % (1090 / 1242), canvas pairs 191 / 220; 34 reference-side and 21 Firepixel-side errors
(unchanged); one planned reference restart; 11 `runner error` lines in `tool-text` (TW-146 rest). `panel-layers` is a
real deviation by TW-205 (the Layers opacity slider: 31 against 30 mid-drag, 81 against 80 after a 41 px press-drag;
fixed by F79 after the build) and TW-208 (a Shift range from L1obj to the shared layer's object: 2 selected objects in
Firepixel, 1 in the reference), `inspector-text` by the new TW-214 (a script-made point text 13 px higher). TW-206 and
TW-207 are confirmed, TW-204 behaves as documented (the ink equal, Firepixel widens the bounds). The runner items
TW-197, TW-210 and TW-211 are confirmed; the host fixes void the reference sides of `inspector-text/09`, `13`, `14`
and `color-popup/05` in runs up to 36, which agree in run 37. New: TW-213, TW-214, TW-215 (F79) and the runner item
TW-216.

**20 Oct 2026, roadmap items 9–19 (after tester batch 3, `main` from `7d51762c` to `612b34bd`):**

- *Runner R-26* (`f4f41991`): TW-167 closed by run 29; the layout check expands the reference's collapsed panel
  dock (figures above, unchanged).
- *PDF layers* (U-1 rest, `9c6a52b9`): PDF (and AI ≥ 9) import creates one layer per optional content group, per
  page: names from `/Name`, nested groups as "Parent / Child", groups that are off by default as hidden layers,
  content outside any group in "Layer 1". Limits: visibility expressions (OCMDs) are not split, groups used only
  inside form XObjects stay with the layer of the `Do`, `/Intent`, `/Usage` and `/AS` are ignored, one render per
  layer; only hand-made test files so far, so a real layered PDF is click-test A4-28.
- *Tests on three systems* (BA-083, `c326d7c0`/`e830a9c2`, `c9cf143f`): the macOS and Linux workflows run the
  batch, export, text and other platform-neutral crates with the input `tests` (artifacts `tests-macos`,
  `tests-linux`); all golden hashes are equal on Windows, macOS and Linux. Test fixes: a platform-native path
  helper, skips without the reference font, `fp-text` indexes the Linux font folders (`/usr/share/fonts` and
  others) with Liberation/DejaVu fallbacks, and the Linux job installs `fonts-liberation` and `fonts-dejavu-core`
  ([RELEASE.md](RELEASE.md)).
- *Metadata in AVIF, HEIF and JPEG XL* (BA-108, `9f00f3fb`): EXIF and XMP are written (AVIF/HEIF: `Exif`/`mime`
  items with `iinf`/`iloc`/`iref`/`ipma` updated and old payloads blanked; JPEG XL: `Exif`/`xml ` boxes, a bare
  codestream is wrapped, `jbrd` dropped); IPTC core fields are mirrored into XMP. Limits: no ICC, files do not
  shrink, the JXL writer runs only with a registered encoder ([BATCH.md](BATCH.md) §3.7).
- *GIF optimizer* (BA-110 rest, `f83a621f`): option `lossyMinSsim` (0.5–0.9999, 0 = off) bisects the loss per frame
  over 0…600 and keeps the SSIM of the drawn area against the frame before smoothing at or above the bound; cartoon
  −38.1 % vs. the export at SSIM ≥ 0.99, screen unchanged at −54.3 %, the camera pan never reaches 0.99 and stays
  loss-free; about 2× the encode time. Cancel now works inside one encode (per frame, search step and LZW row; the
  temporary file is deleted). Limits in [BATCH.md](BATCH.md) §5.6.
- *Text tool defaults* (TW-138 rest, F52, RE spec 56, `f5e61c58`): activating the Text tool resets indent, spacing
  before/after, leading (`DefaultLeading`, % mode), tracking, horizontal scale and baseline shift unless the
  preference `TextEditor_RememberALLSettings` is on, so values set with the Text tool active survive and values set
  with the Pointer are lost; new text gets tracking 0; each field change is a "Text Change" History row (nothing
  selected or pointer selection), an editor session one row; `Text_RangeKerning` is stored as field / 100; the %→px
  leading switch rounds; `draw.text` honours `Text_LeadingMode`. Open: the reference's two rows for a new text
  against Firepixel's one, the unmeasured px→% conversion and the opacity/blend part of the reset (no own text tool
  opacity in Firepixel); click-test A4-29.
- *Performance* (PERF-3, `090abd30`, `852e00eb`; 6000 × 1600 page, 3200 objects, release, software rendering):
  BA-097 drag preview limited to the view (first drag step 5.4 s → 1.7 s; the `fp-raster` composite thread hand-off
  5.2 s → 0.06 s; residue: a full preview when the whole selection is in view); BA-098 canvas in 512-px tiles on a
  device-pixel grid anchored at the page corner (worst frame after a zoom 1.0–2.5 s → 0.44–0.77 s, after a scroll
  1.2–2.4 s → 0.31–0.58 s; full run 30 found no one-device-pixel sampling shift at fractional zoom: the zoom shots
  are pixel-identical to the untiled baseline or changed only in label anti-aliasing, with no new offset); BA-099 virtualised Layers panel rows (frame 281 → 112 ms), early exit in the command state
  checks (all 641 commands 252 → 46 ms), the render cache kept on move and small paths rasterised on the calling
  thread (render after a move 3.7 s → 0.26 s).
- *Maintenance* (`612b34bd`): image series and function-test reports regenerated after the hinting change; BA-092
  and BA-087 closed (nothing tracked changed).

## How tests run

- Every crate has unit tests; `fp-ui` and `fp-engine` bundle their integration tests into one program `all`
  (`cargo test -p fp-ui --test all`). Corpus and golden tests (`#[ignore]`) run with `-- --ignored` and compare
  against measured reference output.
- UI tests run offscreen (egui kittest and a wgpu renderer) and drive the app through the same command registry
  as the user. Tests never write into the real user profile.
- Guards: `fp-neutral` (no third-party product names in UI texts and manuals), `fp-i18n` (every key in German and
  English), clipping audit for dialogs.
- A **full test run** builds everything in debug with optimisations and runs all suites; it precedes every
  preview build.

## How parity runs work

A parity run plays scripted steps in both programs on the same machine and compares screenshots, values, texts and
exported files ([PARITY.md](PARITY.md)). A full run over 49 units takes about four hours and runs unattended
(usually overnight); follow-up runs cover only affected units. Each run is evaluated by script into tweaks
(`TW-…`) and questions (`Q-…`), which become findings or fixes. Runs need a licensed installation of the
reference application and test files that are not part of this repository.

## What is not verified automatically

Embedded panel dialogs of the reference application, real keyboard paths (Alt tap, access keys), brush strokes
with a real mouse and drag and drop between windows need a human click-test: [VERIFICATION.md](VERIFICATION.md).

## Next

1. Human click-test of [VERIFICATION.md](VERIFICATION.md), including section A4 (wave-4 features, URL panel menu,
   typing after a double-click, crop option off, Linux start, pen anchor modes, the Select Behind hover frame,
   the History rows of key nudges, the screen modes, the Subselection display A4-23, tester feedback batch 3 A4-24
   to A4-27, a real layered PDF A4-28, the text tool reset A4-29, JPEG 2000 in PDF A4-30, the AI batch steps A4-31 and A4-32, the parity fixes of 21/22 Oct 2026 A4-38 to A4-44, tester feedback batch 5 A4-45 to A4-49, the Dither fill edge A4-50, tester feedback batch 6 A4-51 to A4-55 and the fixes after v0.2.9 A4-56 (Feather and Dither on text) and A4-57 (the transform tools and the Numeric Transform dialog with the scale-attributes option off), and the RE-30 fixes A4-58 (the Escape and arrow rules of Q-62) and A4-59 (gradient and pattern fills on text) and A4-60 (F79: the Layers disclosure from the document and the opacity slider scale)), the text selection edits of A1-19 and the open
   A2 checks.
2. The macOS re-test of v0.2.4 by the tester (A4-19 to A4-21), now also for TW-198 and TW-199 (A4-45, A4-46) and the
   Cmd and Ctrl clicks of TW-208 (A4-55).
3. Tester feedback batch 6 (23 Oct 2026, FB-24 to FB-30, measurement RE-29, RE spec 61) is on `main` after v0.2.8:
   F69 `c9c9a329` (TW-204), F70 `4657c7cb` (TW-205, TW-206), F72 `249f4569` (TW-208) and F68 `99114278` (TW-207); TW-209
   declined; GT-1q 3025 passed; released as v0.2.9. Follow-up run 36 (23 Oct 2026) over
   `panel-layers`, `panel-layers-menu`, `color-popup`, `color-popup-contexts`, `inspector-text`, `inspector-fields`, `fx-bevel-emboss-fill`, `tool-text` and `xs-select-behind` reached none of the batch 6
   interactions and changed no class (49 / 0 / 0 / 0, quota 87.3 %, canvas pairs 186 / 212). Follow-up run 37 (23 Oct 2026, runner R-32, the code of v0.2.10) reached TW-204 to TW-208: TW-206 and TW-207 confirmed, TW-204 as documented, real deviations TW-205 (fixed by F79 after the build) and TW-208 (the range across layers) in `panel-layers` and TW-214 in `inspector-text` (47 / 2 / 0 / 0, quota 87.8 %, canvas pairs 191 / 220); TW-197, TW-210 and TW-211 are handled by R-32. After v0.2.10: F79 `7337428e` (TW-215, layer disclosure from the document, opacity sliders 2 px per per cent), pending run 38 and the click-test A4-60. Then the open items:
   the TW-208 measurement (Shift ranges across layers) and its fix, TW-213 and TW-214 (text created by script), the
   runner item TW-216 (the Layers drag and selection-dot steps), the Q-65 measurement (the current layer after opening
   T07), and the rests (the coordinate tag for the other tools, TW-195 gradient or pattern tool fills on auto
   shapes, the TW-207 rests: the Pointer's own scale handles, `setSelectionBounds` / `resizeSelection`, the auto-trim
   of menu rotations). After v0.2.9 `main` received F73 `2e0fe826` (TW-203 rest: Feather and Dither on text; the
   Alter Path item is dropped, the reference has none), F74 `b585f6b8` and F75 `2150bbf3` (TW-207 rests: the
   transform tools, the Numeric Transform dialog and the scripting calls honour their scale-attributes option),
   pending the click-test A4-56 and A4-57. Measurement RE-30 (RE spec 62, 23 Oct 2026) is done: it re-measured the Q-62
   rules with real keys and measured Q-64; F76 `3cf23641` (Escape and arrow rules) and F77 `2b702176` (gradient and
   pattern fills on text) are on `main`, pending the click-test A4-58 and A4-59; new row TW-212 (font size instead of
   a matrix when a text is scaled). Still open per the spec: Escape mid-drag after earlier drags of a transform
   session, other shapes than the rectangle, other paint tools, the oval Marquee and the Polygon Lasso mid-drag,
   Ctrl/Alt+arrow with a pixel selection, radial gradients, rotation and `XformToolXformsBFE=false` for text.
4. Preview `2026-10-22_14-27` (GT-1o, `main` at `8c636395`, tester feedback batch 5 F62 to F66 on top of F61) is the
   code of follow-up run 35; before it preview `2026-10-22_05-02` (GT-1n, `84346fec`) was the code of run 34. The
   latest pre-release is v0.2.7 (22 Oct 2026, from `8160a306`); the batch 5 fixes came after it.
5. After follow-up run 35 (22 Oct 2026, 6 units, tester feedback batch 5; 49 / 0 / 0 / 0, proven 1:1 100.0 %, quota
   87.0 %, canvas pairs 185 / 209; no step status and no compared value changed): the click-test A4-45 to A4-49
   (item 1; the Pen display and the pen cursors of TW-202, the live sliders of TW-200, the patterns of TW-198 and the
   focus return of TW-199 are reached by no runner step) and the macOS re-test (item 2, also TW-198 and TW-199); the
   click-test A4-50 for the new Dither fill edge (TW-203, F67); the re-measurement of the Q-62 rules with real keys; runner R-32 for
   the runner item TW-197 (set the reference's Properties inspector at unit start; it did not recur in run 35).
6. After follow-up run 34 (49 / 0 / 0 / 0, proven 1:1 100.0 %, quota 87.0 %, canvas pairs 185 / 209): TW-195 and the
   refined follow rules of TW-192 (F61) and the runner items TW-196 (R-30) and R-31 are confirmed. Next: the
   click-test items A4-38 to A4-44 (item 1) and the macOS re-test (item 2), the re-measurement of the Q-62 rules with
   real keys, runner R-32 for the new runner item TW-197 (set the reference's Properties inspector at unit start) and
   the unmeasured rests of TW-186, TW-193, TW-195 and Q-61. TW-173, TW-189, TW-192, TW-193, TW-194 and TW-195 are
   confirmed by the runner; TW-176, TW-177, TW-186, TW-187 and the Q-61 tag are not reached by any step and stay with
   the click-test.
7. Remaining deferred items: BA-101 (PSD stroke; JPEG 2000 in PDF done in `974af253`); BA-109 closed (`ai.upscale` `9f3a1647`, `ai.faceBlur` `4f811360`).

Wave 3 (18 Oct 2026) added 27 batch steps, chained collect steps, more metadata formats, the MCP gate for external
programs and unsandboxed scripts, opt-in GIF optimiser modes, the remaining Batch Studio UI conveniences, selectable
PDF text and editable PSD layers ([FINDINGS.md](FINDINGS.md), [BATCH.md](BATCH.md)). Wave 4 (18 Oct 2026)
added HTML Setup defaults, an external page style sheet and a Generic HTML5 style, working screenshot, symbol
script and shared palette commands, the ffmpeg row in Preferences, an optional AI object-selection preview on hover
and the brush live preview in the bitmap's stacking position.

See [ROADMAP.md](../ROADMAP.md) for the complete list.
