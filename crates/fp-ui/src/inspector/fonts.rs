//! Schriftenliste für den Text-Inspektor und den Texteditor: Index der Systemschriften über
//! fp-text (zur Laufzeit, im Hintergrund aufgebaut; bis dahin eine kleine Rückfallliste).

use std::sync::{Arc, OnceLock};

/// Eine Schriftfamilie mit ihren Stilen (Anzeigename, PostScript-Name).
#[derive(Clone, Debug, PartialEq)]
pub struct Family {
    pub name: String,
    pub styles: Vec<(String, String)>,
}

static INDEX: OnceLock<Arc<Vec<Family>>> = OnceLock::new();
static STARTED: OnceLock<()> = OnceLock::new();

fn build() -> Vec<Family> {
    let db = fp_text::font::FontDb::system();
    let mut out: Vec<Family> = Vec::new();
    for f in db.faces() {
        let fam = if f.names.family.is_empty() {
            f.names.legacy_family.clone()
        } else {
            f.names.family.clone()
        };
        if fam.is_empty() {
            continue;
        }
        let style = if f.names.subfamily.is_empty() {
            "Regular".to_string()
        } else {
            f.names.subfamily.clone()
        };
        let ps = if f.names.ps_name.is_empty() {
            fam.replace(' ', "")
        } else {
            f.names.ps_name.clone()
        };
        match out.iter_mut().find(|x| x.name == fam) {
            Some(x) => {
                if !x.styles.iter().any(|(s, _)| *s == style) {
                    x.styles.push((style, ps));
                }
            }
            None => out.push(Family {
                name: fam,
                styles: vec![(style, ps)],
            }),
        }
    }
    out.sort_by_key(|f| f.name.to_lowercase());
    out
}

/// Startet den Aufbau im Hintergrund (einmal).
pub fn warm_up() {
    if STARTED.set(()).is_ok() {
        let _ = std::thread::Builder::new()
            .name("fp-fonts".into())
            .spawn(|| {
                let _ = INDEX.set(Arc::new(build()));
            });
    }
}

/// Alle Familien (leer, solange der Index noch aufgebaut wird).
pub fn families() -> Arc<Vec<Family>> {
    warm_up();
    INDEX.get().cloned().unwrap_or_else(|| {
        Arc::new(
            ["Arial", "Segoe UI", "Tahoma", "Times New Roman", "Verdana"]
                .iter()
                .map(|n| Family {
                    name: n.to_string(),
                    styles: vec![("Regular".into(), n.replace(' ', ""))],
                })
                .collect(),
        )
    })
}

/// Grundschnitt einer Familie: „Regular“ bzw. der gleichwertige Name (Normal, Roman, Book,
/// Standard, Medium), sonst der erste Schnitt. Die Reihenfolge der Schnitte im Index folgt den
/// Schriftdateien (bei Myriad Pro z. B. „Bold“ zuerst) und taugt nicht als Vorgabe.
pub fn base_style(f: &Family) -> String {
    for want in ["regular", "normal", "roman", "book", "standard", "medium"] {
        if let Some((s, _)) = f.styles.iter().find(|(s, _)| s.eq_ignore_ascii_case(want)) {
            return s.clone();
        }
    }
    f.styles
        .first()
        .map(|s| s.0.clone())
        .unwrap_or_else(|| "Regular".into())
}

/// Familie und Stil zu einem Schriftnamen (PostScript- oder Familienname). Ein reiner
/// Familienname (z. B. „Myriad Pro“ eines neuen Texts) zeigt wie im Original den Grundschnitt
/// „Regular“ (PARITAET_A W14); Fett/Kursiv des Inspektors sind Faux-Schnitte und ändern ihn nicht.
pub fn lookup(name: &str) -> (String, String) {
    let fams = families();
    for f in fams.iter() {
        if f.name.eq_ignore_ascii_case(name) {
            return (f.name.clone(), base_style(f));
        }
        if let Some((s, _)) = f
            .styles
            .iter()
            .find(|(_, ps)| ps.eq_ignore_ascii_case(name))
        {
            return (f.name.clone(), s.clone());
        }
    }
    let base = name
        .split('-')
        .next()
        .unwrap_or(name)
        .trim_end_matches("MT")
        .to_string();
    (
        if base.is_empty() {
            name.to_string()
        } else {
            base
        },
        "Regular".into(),
    )
}

/// PostScript-Name zu Familie und Stil.
pub fn ps_name(family: &str, style: &str) -> String {
    let fams = families();
    fams.iter()
        .find(|f| f.name == family)
        .and_then(|f| {
            f.styles
                .iter()
                .find(|(s, _)| s == style)
                .or(f.styles.first())
                .map(|(_, ps)| ps.clone())
        })
        .unwrap_or_else(|| family.to_string())
}

// ------------------------------------------------------------------ Kombifeld mit Suche (N19)

/// Zeilenhöhe der Aufklappliste (wie [`super::fields::list_item`]).
const ROW_H: f32 = 18.0;
/// Höchste Höhe der Liste unter dem Suchfeld (wie bei [`super::fields::combo`]).
const LIST_MAX_H: f32 = 360.0;
/// Höhe des Suchfelds.
const SEARCH_H: f32 = 20.0;
/// Markierte Familie der zuletzt gezeichneten Schriftliste (für Tests und Diagnose).
const MARKED_ID: &str = "fp-font-search-marked";

/// Zustand der offenen Liste: Suchtext, markierte Familie (Index in der ganzen Liste), der
/// Bildlauf, den das nächste Bild setzen soll, und der zuletzt gezeichnete Bildlauf.
#[derive(Clone, Debug, Default)]
struct SearchState {
    query: String,
    mark: Option<usize>,
    scroll: Option<f32>,
    offset: f32,
}

/// Bildlaufleiste der Schriftliste (Nutzerbefund N24). Die Farben (Spur `scrollbar_bg`, Griff
/// `scrollbar_thumb`) gelten seit NZ-3 app-weit über [`crate::theme::scrollbar_widgets`]; hier
/// bleibt nur die feste, gut greifbare Breite der Leiste. Gilt nur für diese `Ui`.
pub fn list_scrollbar_style(ui: &mut egui::Ui) {
    let sc = &mut ui.spacing_mut().scroll;
    sc.floating = false;
    sc.bar_width = sc.bar_width.max(LIST_BAR_W);
}

/// Breite der Bildlaufleiste der Schriftliste (wie die Leinwandleisten, 8 px; gut zu greifen).
pub const LIST_BAR_W: f32 = 8.0;

/// Fundstelle von `needle` (schon klein geschrieben) in `hay`, ohne Groß-/Kleinschreibung, als
/// Byte-Bereich in `hay`. Eine leere Suche passt immer (leerer Bereich).
pub fn find_ci(hay: &str, needle: &str) -> Option<(usize, usize)> {
    if needle.is_empty() {
        return Some((0, 0));
    }
    let want: Vec<char> = needle.chars().collect();
    for (start, _) in hay.char_indices() {
        let mut k = 0;
        for (pos, c) in hay[start..].char_indices() {
            let mut ok = true;
            for lc in c.to_lowercase() {
                if k < want.len() && lc == want[k] {
                    k += 1;
                } else {
                    ok = false;
                    break;
                }
            }
            if !ok {
                break;
            }
            if k == want.len() {
                return Some((start, start + pos + c.len_utf8()));
            }
        }
    }
    None
}

/// Indizes der Einträge, die zur Suche passen (Teilwort, ohne Groß-/Kleinschreibung; Leerzeichen
/// am Rand zählen nicht).
pub fn filter_names(names: &[&str], query: &str) -> Vec<usize> {
    let q = query.trim().to_lowercase();
    names
        .iter()
        .enumerate()
        .filter(|(_, n)| find_ci(n, &q).is_some())
        .map(|(i, _)| i)
        .collect()
}

/// Familie, die in der zuletzt gezeichneten Schriftliste markiert war (`None` = Liste zu oder
/// kein Treffer).
pub fn marked_family(ctx: &egui::Context) -> Option<String> {
    ctx.data(|d| d.get_temp::<Option<String>>(egui::Id::new(MARKED_ID)))
        .flatten()
}

/// Kombifeld der Schriftfamilien (Inspektor, Texteditor). Geschlossen und ohne Tippen wie
/// [`super::fields::combo`]; die aufgeklappte Liste hat oben ein Suchfeld (Nutzerwunsch N19, über
/// das Original hinaus), das sofort den Fokus hat: Tippen filtert (Teilwort, ohne Groß-/
/// Kleinschreibung) und hinterlegt die Fundstelle, ↑/↓ (Bild ↑/↓) bewegen die Markierung,
/// Eingabe übernimmt, Esc schließt ohne Änderung. Die Liste zeichnet nur die sichtbaren Zeilen,
/// damit sie auch mit vielen hundert Schriften flüssig bleibt. Liefert den Index in `names`.
pub fn family_combo(
    ui: &mut egui::Ui,
    id: impl std::hash::Hash + std::fmt::Debug,
    r: egui::Rect,
    names: &[&str],
    selected: Option<usize>,
    enabled: bool,
) -> Option<usize> {
    use egui::{Align2, Id, Key, Modifiers, Rect, Sense, Stroke, pos2, vec2};
    let id = Id::new(id);
    let text = selected.and_then(|i| names.get(i)).copied().unwrap_or("");
    let resp = super::fields::combo_box_frame(ui, id, r, text, enabled);
    super::fields::audit_combo_items(ui, r, names);
    let theme = crate::theme::Theme::current(ui.ctx());
    let sid = id.with("font-search");
    let popup_id = egui::Popup::default_response_id(&resp);
    let mut out = None;
    egui::Popup::menu(&resp)
        .frame(crate::menu::menu_frame(&theme))
        .width(r.width().max(60.0))
        // ein Klick in das Suchfeld schließt die Liste nicht (Einträge schließen selbst)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            let c = &theme.colors;
            let stored = ui.data(|d| d.get_temp::<SearchState>(sid));
            let fresh = stored.is_none();
            let mut s = stored.unwrap_or(SearchState {
                mark: selected,
                ..Default::default()
            });
            let mut shown = filter_names(names, &s.query);
            let view_h = |n: usize| (n.max(1) as f32 * ROW_H).min(LIST_MAX_H);
            let centred =
                |p: usize, n: usize| (p as f32 * ROW_H - (view_h(n) - ROW_H) / 2.0).max(0.0);
            if fresh {
                // wie bisher markiert die Liste die aktuelle Schrift; sie steht in Sicht
                if let Some(p) = selected.and_then(|sel| shown.iter().position(|&i| i == sel)) {
                    s.scroll = Some(centred(p, shown.len()));
                }
            }
            // Tasten vor dem Suchfeld abfangen: sonst bewegen ↑/↓ die Schreibmarke, Eingabe
            // nähme dem Feld den Fokus und Esc ginge an die Leinwand
            let (down, up, page_down, page_up, enter, esc) = ui.input_mut(|i| {
                (
                    i.consume_key(Modifiers::NONE, Key::ArrowDown),
                    i.consume_key(Modifiers::NONE, Key::ArrowUp),
                    i.consume_key(Modifiers::NONE, Key::PageDown),
                    i.consume_key(Modifiers::NONE, Key::PageUp),
                    i.consume_key(Modifiers::NONE, Key::Enter),
                    i.consume_key(Modifiers::NONE, Key::Escape),
                )
            });
            // Suchfeld (hat beim Aufklappen sofort den Fokus)
            let (fr, _) = ui.allocate_exact_size(
                vec2(ui.available_width().max(60.0), SEARCH_H),
                Sense::hover(),
            );
            let te_id = sid.with("edit");
            let focused = ui.memory(|m| m.has_focus(te_id));
            super::fields::field_frame(ui, fr, true, focused);
            let before = s.query.clone();
            let te = egui::TextEdit::singleline(&mut s.query)
                .id(te_id)
                .frame(egui::Frame::NONE)
                .font(theme.ui_font())
                .text_color(c.field_text)
                .hint_text(egui::RichText::new(fp_i18n::t("pi-font-search")).color(c.text_disabled))
                .margin(egui::Margin::ZERO)
                .vertical_align(egui::Align::Center)
                .desired_width(fr.width() - 8.0)
                .event_filter(super::fields::TEXT_FILTER);
            let te_resp = ui.put(fr.shrink2(vec2(4.0, 1.0)), te);
            if !te_resp.has_focus() && !esc && !enter {
                te_resp.request_focus();
            }
            ui.add_space(2.0);
            if s.query != before {
                shown = filter_names(names, &s.query);
                // neuer Suchtext: erster Treffer markiert; leer: wieder die aktuelle Schrift
                let empty = s.query.trim().is_empty();
                s.mark = if empty {
                    selected
                } else {
                    shown.first().copied()
                };
                s.scroll = Some(
                    match s.mark.and_then(|m| shown.iter().position(|&i| i == m)) {
                        Some(p) if empty => centred(p, shown.len()),
                        _ => 0.0,
                    },
                );
            }
            let vh = view_h(shown.len());
            let page = ((vh / ROW_H).floor() as isize - 1).max(1);
            let step: isize = if down {
                1
            } else if up {
                -1
            } else if page_down {
                page
            } else if page_up {
                -page
            } else {
                0
            };
            if step != 0 && !shown.is_empty() {
                let cur = s.mark.and_then(|m| shown.iter().position(|&i| i == m));
                let next = match cur {
                    Some(p) => (p as isize + step).clamp(0, shown.len() as isize - 1) as usize,
                    None if step > 0 => 0,
                    None => shown.len() - 1,
                };
                s.mark = Some(shown[next]);
                let top = next as f32 * ROW_H;
                if top < s.offset {
                    s.scroll = Some(top);
                } else if top + ROW_H > s.offset + vh {
                    s.scroll = Some(top + ROW_H - vh);
                }
            }
            if esc {
                ui.close();
            }
            if enter {
                if let Some(m) = s.mark.filter(|m| shown.contains(m)) {
                    out = Some(m);
                    ui.close();
                }
            }
            let list_top = ui.cursor().top();
            if shown.is_empty() {
                let (nr, nresp) = ui.allocate_exact_size(
                    vec2(ui.available_width().max(60.0), ROW_H),
                    Sense::hover(),
                );
                let msg = fp_i18n::t("pi-font-not-found");
                crate::widgets::text_clipped(
                    ui.painter(),
                    pos2(nr.left() + 6.0, nr.center().y),
                    Align2::LEFT_CENTER,
                    &msg,
                    theme.ui_font(),
                    c.menu_text_disabled,
                    nr.width() - 8.0,
                );
                nresp.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::Label, true, msg.clone())
                });
            }
            let q = s.query.trim().to_lowercase();
            list_scrollbar_style(ui);
            let mut area = egui::ScrollArea::vertical()
                .id_salt(sid.with("list"))
                .max_height(LIST_MAX_H)
                .auto_shrink([false, true]);
            if let Some(off) = s.scroll.take() {
                area = area.vertical_scroll_offset(off);
            }
            let pointer_moved = ui.input(|i| i.pointer.delta() != egui::Vec2::ZERO);
            let font = theme.ui_font();
            let so = area.show_rows(ui, ROW_H, shown.len(), |ui, rows| {
                for &i in &shown[rows] {
                    let name = names[i];
                    let (rr, rresp) = ui.allocate_exact_size(
                        vec2(ui.available_width().max(60.0), ROW_H),
                        Sense::click(),
                    );
                    if rresp.hovered() && pointer_moved {
                        s.mark = Some(i);
                    }
                    let p = ui.painter();
                    if rresp.hovered() {
                        p.rect_filled(rr, 0.0, c.menu_hover_bg);
                    } else if s.mark == Some(i) {
                        p.rect_filled(rr, 0.0, c.list_selected_bg);
                    }
                    let tx = rr.left() + 6.0;
                    // Fundstelle hinterlegen
                    if let Some((a, b)) = find_ci(name, &q).filter(|(a, b)| b > a) {
                        let w = |s: &str| {
                            ui.ctx().fonts_mut(|f| {
                                f.layout_no_wrap(s.to_string(), font.clone(), egui::Color32::WHITE)
                                    .size()
                                    .x
                            })
                        };
                        let (x0, x1) = (tx + w(&name[..a]), tx + w(&name[..b]));
                        let hr = Rect::from_min_max(
                            pos2(x0 - 1.0, rr.top() + 2.0),
                            pos2(x1 + 1.0, rr.bottom() - 2.0),
                        );
                        p.rect_filled(hr, 2.0, crate::theme::with_alpha(c.focus, 80));
                        p.rect_stroke(
                            hr,
                            2.0,
                            Stroke::new(1.0, crate::theme::with_alpha(c.focus, 170)),
                            egui::StrokeKind::Inside,
                        );
                    }
                    let col = if rresp.hovered() {
                        c.menu_hover_text
                    } else {
                        c.menu_text
                    };
                    p.text(
                        pos2(tx, rr.center().y),
                        Align2::LEFT_CENTER,
                        name,
                        font.clone(),
                        col,
                    );
                    rresp.widget_info(|| {
                        egui::WidgetInfo::selected(
                            egui::WidgetType::SelectableLabel,
                            true,
                            Some(i) == selected,
                            name,
                        )
                    });
                    if rresp.clicked() {
                        out = Some(i);
                        ui.close();
                    }
                }
            });
            s.offset = so.state.offset.y;
            // gleich hoch wie die ganze Liste: die Liste springt beim Tippen nicht (sonst klappte
            // sie bei wenigen Treffern von oberhalb nach unterhalb des Felds um)
            let used = ui.cursor().top() - list_top;
            let pad = view_h(names.len()) - used;
            if pad > 0.5 {
                ui.add_space(pad);
            }
            let marked = s
                .mark
                .filter(|m| shown.contains(m))
                .map(|m| names[m].to_string());
            ui.data_mut(|d| {
                d.insert_temp(sid, s);
                d.insert_temp(Id::new(MARKED_ID), marked);
            });
        });
    // zugeklappt: Suchtext vergessen (beim nächsten Aufklappen leer, wie bisher die ganze Liste)
    if !egui::Popup::is_id_open(ui.ctx(), popup_id)
        && ui.data(|d| d.get_temp::<SearchState>(sid)).is_some()
    {
        ui.data_mut(|d| {
            d.remove::<SearchState>(sid);
            d.insert_temp(Id::new(MARKED_ID), None::<String>);
        });
    }
    out
}
