# UI design rules: windows, dialogs and modals

This document contains the **design rules** that apply to every new or reworked window in Firepixel
and describes the **shared building blocks** in `crates\fp-ui\src\ui_kit.rs`. The concrete values
live in code as constants and theme functions and are changed there, not here.

Reference images (generated locally, not committed): `docs\screenshots\design_before_*.png` and `design_after_*.png`; each image shows
the three themes side by side (**Normal | Dark | OLED**). They are generated with
`cargo test -j 4 -p fp-ui --test all design_images:: -- --ignored --nocapture` and
`FP_DESIGN_IMAGE=before|after` (start page: `--test all design_welcome:: -- --ignored`).

---

## 1. Principles

1. **Staying close to the original is mandatory.** Dialogs that exist in the reference application
   (Canvas Size, Image Size, New Document, Preferences, Export, Levels, Curves, filter and effect
   dialogs, Keyboard Shortcuts, Spelling, Pop-up Menu Editor ...) keep their **structure, field
   order, labels and operation** (tab order, default button, access keys, Enter/Esc). Only the
   execution is reworked: spacing, alignment, group boxes, header, button bar, colours, states.
   Wording and operation stay the same, and their tests stay green.
2. **Firepixel's own windows** (AI models, AI model missing, language, MCP control channel, start
   page, messages) follow these rules freely.
3. **No hard-coded text.** All building blocks take their texts as parameters; callers fetch them
   via `fp_i18n::t`. New texts are added to `de` and `en` at the same time.
4. **Colours come exclusively from the theme** (`theme.rs`, `Theme::current(ctx)`); no colour values
   in dialog code. Derived tones only via `theme::mix`/`with_alpha` or `ui_kit::tint`/`card_bg`.
5. **Clean room**: own icons (`icons.rs`, `ui_kit::KitIcon`), own logo (`assets\icon`), no graphics
   or trademarks of the original; the product name of the original never appears.

## 2. Grid, spacing, margins

Everything sits on an **8 px grid** (`ui_kit::sp`):

| Constant | Value | Use |
|---|---|---|
| `sp::XS` | 4 | icon ↔ text in dense rows, title ↔ explanatory sentence |
| `sp::S` | 8 | spacing within a group, between buttons, below section headings |
| `sp::M` | 12 | label ↔ field, inner padding of notice blocks and header areas |
| `sp::L` | 16 | **dialog margin**, spacing between sections |
| `sp::XL` | 24 | generous areas (start page, column gap) |
| `sp::ROW` | 24 | row height of label–value rows and lists |
| `sp::BUTTON_H` | 23 | button height |

- 16 px dialog margin all around; nothing sticks to the edge or the title bar.
- Elements in a row are **vertically centred**; labels in a column are **left-aligned on one edge**.
- Group boxes (`dialogs::group_box`) only where the original has them; otherwise sections (§4).
- Dialogs whose content can grow (lists, tables, long texts) are **resizable**
  (`dialog_frame_resizable`) or scroll; fixed dialogs compute their size from the text widths
  (`dialogs::extra_w`, `buttons_w`), never the other way round.

## 3. Header area per dialog type

| Type | Header | Examples |
|---|---|---|
| **Settings dialog of the original** | no extra header (as in the original); title bar only | Canvas Size, Image Size, filters, effects, Levels, Curves |
| **Own management window** | `ui_kit::dialog_header`: 32 px icon, title 15 px bold, explanatory sentence (wrapped, muted), card surface, etched line | Manage AI Models, Language, MCP control channel, Saved Commands |
| **Confirmation / message** | 32 px icon on the left (info/warning/error, `KitIcon`), text to its right, no title in the content (it is in the title bar) | Unsaved changes, Confirm, Message |
| **Info window** | app logo (`HeaderIcon::App`), name, version | About Firepixel |

## 4. Sections and separators

- `ui_kit::section(ui, title)`: heading **12 px bold in `section_label`**, followed by an etched line
  to the right edge; 4 px below it, then the content. 16 px to the next section.
- `ui_kit::separator` / `hline` / `vline`: etched line (dark + highlight edge) in the Normal and Dark
  themes, a single fine line in OLED. No triple splitter lines in dialogs (those belong to docks).

## 5. Label–value grid

- Label column = widest label + 12 px (`ui_kit::label_width`), **the same for all rows of a
  section**; fields start on the same edge.
- Row height 24 px, field height 20–22 px, vertically centred (`ui_kit::kv_row`).
- Units (px, %, °) directly right of the field with 4 px spacing, muted.
- Labels that are too long (other languages) are shortened with "…" **and** reported as an overflow
  (`dialogs::report_overflow`) – but `label_width` should prevent shortening in the first place.

## 6. Button bar

- `ui_kit::button_bar`: at the bottom edge, inner padding 12 px (the footer line is drawn by the
  dialog frame, §12.2).
- **Right**: the main buttons in Windows order: **OK (default button, border in `focus`)**, Cancel,
  optionally Apply. **Left**: secondary buttons (Help, Reset, Preview toggle).
- All buttons of a bar are **the same width** (longest label + 12 px, at least 76 px), height 23 px,
  spacing 8 px.
- Enter triggers the default button, Esc cancels (via `dialogs::push_button`).
- Dialogs of the original with buttons at the top right (e.g. Levels, Curves: OK/Cancel/Auto to the
  right of the content) **keep** that position; only width, height and spacing are unified there.

## 7. Notice and warning blocks

`ui_kit::notice(ui, Tone, title, text)`: tinted surface (`ui_kit::tint`), 3 px accent stripe on the
left in the tone colour, 16 px icon, optional bold title, wrapped text. Tones: `Info` (`focus`),
`Warning` (`icon_yellow`), `Error` (`icon_red`), `Success` (`icon_green`). Errors and warnings in
dialogs are never just red text without a surface.

## 8. Tables and lists

`ui_kit::table`: header in the panel header style (`widgets::header_bar`, 22 px, bold 11 px),
columns with fine separators, rows 22–24 px, **zebra stripes** (`ui_kit::zebra`), **hover**
(`list_hover_bg`), **selection** (`list_selected_bg`/`list_selected_text`, OLED with an accent
border), vertical scrolling, cell text with "…" and tooltip (`ui_kit::cell_text`). Numbers
right-aligned (`Column::right`). Border in `list_border`.

## 9. Empty states and progress

- **Empty**: `ui_kit::empty_state` – muted 32 px icon, title (bold, muted), a short hint on what to
  do (small). Never an empty white area without explanation.
- **Progress**: `ui_kit::progress` – 8 px bar, rounded, fill `focus`, text above; indeterminate =
  moving segment. Cancel button on the right of the button bar.

## 10. Colours and typography

| Role | Colour (palette) |
|---|---|
| Dialog surface | `frame_bg` |
| Card, header area, start page | `ui_kit::card_bg` (= `menu_bg`) |
| Text / muted / disabled | `text` / `text_weak` / `text_disabled` |
| Section heading | `section_label` |
| Link | `focus` (slightly darkened in Normal) |
| Lines | `line_dark`, `line_mid`, `line_light` (via `ui_kit::hline`) |
| Focus, default button, progress | `focus` |

| Level (`ui_kit::type_scale`) | Size | Weight | Use |
|---|---|---|---|
| `CAPTION` | 11 | regular / bold | table headers, hints below fields, footers |
| `BODY` | 11.5 | regular | labels, fields, running text (= `fonts.ui`) |
| `SECTION` | 12 | bold | section headings |
| `TITLE` | 15 | bold | titles in header areas |
| `DISPLAY` | 30 | bold | program name on the start page |

Only these levels; no italic or coloured running text (except links and notice blocks).

## 11. Behaviour in the three themes

- **Normal**: light grey like the original; cards almost white, etched lines, gradients in headers.
- **Dark**: the same structure in dark greys; cards slightly darker than the surface.
- **OLED**: pure black; lines single and fine; focus, selection, hover and header areas with an accent
  glow (`theme.glow`); icons in colour. No grey-on-black text below 4.5:1 contrast.
- Every reworked window is checked in all three themes, in German, English and `qps`, at 1024×768
  and 1920×1080, and at 100 % and 150 % (no overflows, nothing clipped).

## 12. Applied rules: start page, dialog frame, clipping

### 12.1 Start page (`crates\fp-ui\src\welcome.rs`)

- **Header banner**: rounded banner with Firepixel's own flame-pixel logo (`assets\icon`),
  "Firepixel" and a tagline. The pixel field gets denser to the right. Logo 64 px, 48 px in a narrow
  window, 36 px in a very narrow one. In OLED the border glows. `paint_banner` is also used by "About
  Firepixel".
- **Three cards**:
  - **Create New**: two large entries (Firepixel document, From Template...), height 54 px, with
    icon chip, bold title and muted explanation (the existing `.tooltip` texts).
  - **Recent Files**: up to 8 rows of 44 px with a 36 px thumbnail (PNG/FW-PNG, JPEG, BMP, QOI;
    checkerboard behind transparency; otherwise the file-type icon). Below it the folder, muted and
    shortened in the middle. Missing files are muted; the tooltip shows the full path. There is an
    empty state. "Open..." sits at the bottom of the card. Thumbnails are read in the background
    (`welcome\thumbs.rs`), cached by path and modification time; files over 48 MB are not read.
  - **Help and Learning**: four »-links (Quick Start, Keyboard Shortcuts, AI Models, Automation),
    height 26 px; below them a section "Extend" (Saved Commands, Extensions; only when the command is
    registered).
- **Footer** 28 px: "Don't show again" on the left (preference `ShowStartPage`), version on the right.
- **Dimensions**: at most 940 px wide, gap between cards and inner margin `sp::L` (16 px) each, card
  header 34 px, corner radius 8 px. Two columns from 600 px content width, below that one column in
  the order New, Recent, Help; scrolling when the height is not enough.
- **States**: hover tinted in the card's accent colour, hand cursor; focus ring in `focus` for every
  entry. **Keyboard**: Tab/Shift+Tab, arrow keys (also between columns), Enter/Space activate.
- **Themes**:
  - Normal: light cards (`ui_kit::card_bg`) with a soft shadow (offset 2 px, blur 10 px) on the grey
    surface.
  - Dark: cards lighter than the canvas, otherwise they disappear.
  - OLED: black, no shadow. Each card has its own glow border and a title in its group colour
    (green, cyan, orange). Icons are in colour.

Screenshots of the start page in all themes and both languages (generated locally, not committed):
`docs\screenshots\nutzerbefunde\N11_start_{normal,dunkel,oled}_{de,en}.png` (1280 × 860, with
thumbnails and hover over a file), generated with
`cargo test -p fp-ui --test all welcome_des2::n11_start_bilder -- --ignored`.


### 12.2 Dialog frame (`dialogs\mod.rs`, `theme.rs`)

The design lives centrally in the frame (`dialog_frame_ex`) and in the shared building blocks.
Individual dialogs need no code of their own for it. Operation, labels, position and size of fields
and buttons, and dialog sizes stay the same.

- **Footer** behind the bottom button row (`footer_top`):
  - The bottom button row consists of `push_button` buttons that reach to within 18 px of the bottom
    edge of the content.
  - The footer starts 12 px above the button row, but at least 3 px below anything drawn above it.
    If less than 5 px remain above the buttons, it is omitted.
  - **No footer** if anything drawn or another button reaches into the button row (buttons stacked on
    the right as in the filters, a list reaching to the bottom). The original's arrangement takes
    precedence.
  - Surface `dialog_footer_bg`, an etched line `dialog_footer_lines` at the top (dark line +
    highlight edge; OLED a single line). It is drawn into space reserved before the content and
    therefore lies beneath everything.
  - `ui_kit::button_bar` draws no line of its own and registers its bar via `dialogs::set_footer`
    (Confirm, Warning, About, Trace Bitmap). "Unsaved changes" has no line of its own.
- **Title bar**: gradient `dialog_title_gradient`, 1 px line `dialog_title_line` below it. In Dark it
  is clearly darker than the surface. OLED: black, line in `focus`.
- **Shadow** `dialog_shadow`: Normal offset 6 px, blur 20 px, black α 70; Dark 6 px / 22 px / α 160;
  OLED uses an accent glow in `focus` instead (α 46, 18 px, spread 1 px). Position and rectangle of
  the dialog stay unchanged.
- **Default button (OK)** `default_button_fill`: button surface mixed towards `focus`, Normal 12 %
  (20 % under the mouse), Dark 28 % (40 %), OLED 20 % (32 %). The focus ring stays.
- **Group box** `group_box_lines`: etched like a Windows group box (dark line + highlight edge); OLED
  fine, single line.
- **Section heading in forms** (`FieldKind::Caption`, `form.rs`): colour `section_label`, etched line
  (§4).
- All tokens are derived from the palette (the `impl Theme` block for dialogs at the end of
  `theme.rs`). There are no separate colour values.

Before/after images per dialog (Normal | Dark | OLED, generated locally, not committed) go to `docs\screenshots\des1\`, generated with
`FP_DES1_STAGE=before|after cargo test -p fp-ui --test all dialogs_des1::des1_images -- --ignored`.
Dialogs with buttons at the top right (e.g. Gaussian Blur) and Preferences (category list reaches to
within 3 px of the button row) intentionally have **no** footer.


### 12.3 Nothing clipped: dialogs inside the window

- **A dialog always lies entirely inside the main window.** If it is larger, its visible size is
  limited to the window. The content keeps its full size and scrolls in both directions
  (`scrolled_body`). Mouse wheel and scroll bar scroll.
- **Pinned button row**: a 48 px strip stays at the bottom (`DIALOG_PIN_STRIP`, at most half the
  content height). `push_button` moves buttons from the bottom content strip there: right-hand
  buttons stay right, left-hand ones ("Help") stay left. OK, Cancel, Help and Close are always
  visible and usable; Enter triggers the default button, focus stays the same. This strip is also
  the footer from §12.2, with a single instead of an etched line.
- Dialogs that fit stay unchanged (no second code path).
- **Horizontal scrolling in narrow areas** (Properties inspector): the scroll bar floats over the
  content and takes no height. At rest it is 3 px wide (opacity 0.7), 6 px under the mouse. The
  mouse wheel scrolls horizontally without Shift when only that direction is possible.
- **Lists** never extend over a footer. If a panel is too short, the list gets smaller (no minimum
  scroll height).
- Menus follow the same rule: they are limited to the window and get arrow strips.
- **Checker**: `crates\fp-ui\tests\clipping_audit.rs` checks all panels, the inspector, dialogs, the
  start page, menus and tooltips in 24 variants (3 themes × de/en × 100/150 % × window 1024 × 768
  and 1920 × 1080). Text clipped exactly at the edge of a scroll area counts as a scroll hint
  (`hinweis-bildlauf`), not as an error. New windows must have **0 findings** there. The allow list
  `clipping_audit_allow.txt` is not extended, only reduced. Tests for the rule:
  `dialog_scroll_rand2b` (800 × 512: dialog inside the window, OK at the bottom works).

## 13. Open design items

- Tables and lists of individual dialogs: key–value pairs as a grid in Document Info / messages,
  version and licence as a grid in About Firepixel, the label–value grid (§5) in the generic form
  dialogs.
- A footer for Preferences (requires shortening the category list by about 10 px, i.e. a layout
  change).
- Uniform button widths according to §6 in all dialogs.

Deliberately kept as in the original after comparison: Keyboard Shortcuts (no header/zebra), Export
(the space above the buttons holds a hint line), Pop-up Menu Editor (tabs, header, column widths and
22 px buttons already match the original).

Every change comes with before/after images in all three themes (pattern `docs\screenshots\des1\`)
and 0 findings in `clipping_audit`.
