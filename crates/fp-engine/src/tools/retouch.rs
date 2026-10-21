//! Pinselbasierte Bitmapwerkzeuge (10 §2.6): Weichzeichner, Scharfzeichner, Abwedler,
//! Nachbelichter, Verwischen, Stempel, Farbe ersetzen, Rote Augen entfernen. Ziehen sammelt die
//! Strichpunkte (Umschalt = gerade Linie), beim Loslassen wirkt `bitmap.retouch` auf die Bitmap.
//! While dragging, the stroke is computed live on a copy of the target bitmap with the same code
//! as the command, so releasing changes nothing more (FB-14a, RE spec 57 section 1). Over an
//! editable bitmap the system cursor is hidden and only the tip outline shows the pointer; with
//! nothing to edit the cursor is "not allowed" and no outline is drawn (RE spec 57 section 2).
//! Durchmesser, Kante, Form, Intensität/Belichtung/Druck, Bereich und Toleranz kommen aus den
//! Voreinstellungen des Werkzeugs (`BlurToolDiameter` …), die der Eigenschaften-Inspektor setzt.
//! Stempel: Alt+Klick setzt die Quelle; der Versatz zur ersten Malstelle bleibt erhalten
//! (`StampToolSource` = 0 ausgerichtet, 1 fest).

use super::{
    Cursor, Invocation, Overlay, PointerKind, ToolContext, ToolEvent, ToolId, ToolInput, ToolKey,
    ToolMachine, ToolResponse, dist, pt_json,
};
use fp_geom::Point;
use serde_json::{Value, json};

/// Werkzeugparameter aus den Voreinstellungen.
#[derive(Clone, Debug, PartialEq)]
pub struct RetouchParams {
    pub mode: &'static str,
    pub size: f32,
    pub round: bool,
    /// Weiche Kante 0..1.
    pub soft: f32,
    /// Intensität/Belichtung/Druck/Stärke 0..100.
    pub strength: f32,
    /// Bereich (Abwedler/Nachbelichter): 0 Tiefen, 1 Mitteltöne, 2 Lichter.
    pub range: i64,
    /// Abstand der Abdrücke in % des Durchmessers.
    pub spacing: f32,
}

fn prefix(id: ToolId) -> &'static str {
    match id {
        ToolId::Blur => "BlurTool",
        ToolId::Sharpen => "SharpenTool",
        ToolId::Dodge => "DodgeTool",
        ToolId::Burn => "BurnTool",
        ToolId::Smudge => "SmudgeTool",
        ToolId::RubberStamp => "StampTool",
        ToolId::ReplaceColor => "ReplaceColorTool",
        _ => "RedEyeRemovalTool",
    }
}

/// Parameter eines Retusche-Werkzeugs.
pub fn params(id: ToolId, ctx: &ToolContext) -> RetouchParams {
    let p = ctx.prefs;
    let pre = prefix(id);
    let key = |s: &str| format!("{pre}{s}");
    let size = if id == ToolId::RubberStamp {
        p.get_int("StampToolSize", 16)
    } else {
        p.get_int(&key("Diameter"), 13)
    };
    let soft_raw = if id == ToolId::RubberStamp {
        p.get_int("StampToolEdge", 500)
    } else {
        p.get_int(&key("SoftEdge"), 1000)
    };
    let strength = match id {
        ToolId::Blur | ToolId::Sharpen => p.get_int(&key("Intensity"), 50),
        ToolId::Dodge | ToolId::Burn => p.get_int(&key("Exposure"), 50),
        ToolId::Smudge => p.get_int(&key("Pressure"), 50),
        ToolId::RedEyeRemoval => p.get_int(&key("Strength"), 50),
        // Stärke im Inspektor 0..255, Werksvorgabe 200 (`ReplaceColorToolStrength`, PARITAET_A W8);
        // der Befehl bekommt sie unverändert als `replaceStrength` (PAR 05/08)
        ToolId::ReplaceColor => p.get_int(&key("Strength"), 200) * 100 / 255,
        _ => 100,
    };
    RetouchParams {
        mode: match id {
            ToolId::Blur => "blur",
            ToolId::Sharpen => "sharpen",
            ToolId::Dodge => "dodge",
            ToolId::Burn => "burn",
            ToolId::Smudge => "smudge",
            ToolId::RubberStamp => "stamp",
            ToolId::ReplaceColor => "replaceColor",
            _ => "redEye",
        },
        size: size.clamp(1, 1000) as f32,
        round: p.get_bool(&key("IsCircle"), true),
        soft: (soft_raw.clamp(0, 1000) as f32) / 1000.0,
        strength: strength.clamp(0, 100) as f32,
        range: p.get_int(&key("Range"), 1).clamp(0, 2),
        spacing: p.get_int(&key("StampSpacing"), 20).clamp(1, 1000) as f32,
    }
}

/// Retusche-Werkzeuge.
#[derive(Clone, Debug)]
pub struct RetouchTool {
    id: ToolId,
    points: Vec<Point>,
    active: bool,
    hover: Option<Point>,
    /// Stempel: Quelle bzw. Versatz Quelle − Malstelle.
    source: Option<Point>,
    offset: Option<(f32, f32)>,
    /// Der letzte Zug wurde gemalt (Esc danach: „Bitmapmodus beenden“).
    painted: bool,
    /// There is an editable bitmap for the pointer (or, while dragging, for the stroke): tip
    /// outline and hidden cursor; otherwise "not allowed" and no outline.
    target_ok: bool,
    /// Live preview of the running stroke (target bitmap at the press, computed copy; stamp
    /// PAR 05/04, the other brush modes FB-14a).
    live: Option<RetouchLive>,
}

/// Running retouch stroke: original image of the target bitmap and the last computed preview.
#[derive(Clone, Debug)]
struct RetouchLive {
    base: std::sync::Arc<fp_raster::RgbaImage>,
    sel: Option<fp_raster::Mask>,
    /// The stroke reads pixels far from the brush (stamp source): compute on the whole image
    /// instead of the part around the stroke.
    whole: bool,
    /// Smudge with "Entire document": the page rendered once at the press.
    page: Option<std::sync::Arc<fp_raster::RgbaImage>>,
    /// Canvas colour behind pixels whose coverage drops (see `paint::diff_tiles`).
    backdrop: fp_color::Argb,
    preview: crate::paint::PaintPreview,
}

fn next_live_stroke() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(1 << 40);
    N.fetch_add(1, Ordering::Relaxed)
}

impl RetouchLive {
    /// Computes the stroke (parameters as sent by `bitmap.retouch` on release) on a copy of the
    /// target bitmap and keeps the changed pixels as tiles. `false`: no live preview for this mode.
    fn update(&mut self, pts: &[Point], params: &Value, size: f32) -> bool {
        // part of the bitmap the stroke can touch, with room for the 3 x 3 neighbourhood of
        // blur/sharpen, the smudge's work margin and the growth past the bitmap edge
        let h = (size / 2.0).ceil() as i32 + 8;
        let reach = pts.iter().fold(fp_geom::IRect::default(), |r, p| {
            let (x, y) = (p.x.floor() as i32, p.y.floor() as i32);
            r.union(&fp_geom::IRect::new(x - h, y - h, x + h + 1, y + h + 1))
        });
        let mut img = if self.whole {
            (*self.base).clone()
        } else {
            self.base.cropped(reach)
        };
        if !crate::commands::retouch_preview(
            &mut img,
            pts,
            params,
            self.base.bounds(),
            self.sel.as_ref(),
            self.page.as_deref(),
        ) {
            return false;
        }
        // the computed part, including pixels the stroke grew past the bitmap edge
        let area = reach.intersect(&img.bounds());
        let next_gen = self.preview.generation + 1;
        let tiles = crate::paint::diff_tiles(&img, &self.base, area, self.backdrop, next_gen);
        self.preview = crate::paint::PaintPreview {
            bounds: area.to_rect(),
            generation: next_gen,
            stroke: self.preview.stroke,
            target: self.preview.target,
            tiles: std::sync::Arc::new(tiles),
        };
        true
    }
}

impl RetouchTool {
    /// Neues Werkzeug.
    pub fn new(id: ToolId) -> RetouchTool {
        RetouchTool {
            id,
            points: Vec::new(),
            active: false,
            hover: None,
            source: None,
            offset: None,
            painted: false,
            target_ok: false,
            live: None,
        }
    }

    /// Cursor: red eye the crosshair; the brush modes hide the system cursor over an editable
    /// bitmap (the tip outline shows the pointer) and show "not allowed" otherwise (RE 57 §2).
    fn cursor(&self) -> Cursor {
        match self.id {
            ToolId::RedEyeRemoval => Cursor::Crosshair,
            _ if self.target_ok => Cursor::Hidden,
            _ => Cursor::NotAllowed,
        }
    }

    /// Live preview at the press (target as `bitmap.retouch` chooses it; stamp PAR 05/04, the
    /// other brush modes FB-14a). Red eye draws its rectangle instead.
    fn begin_live(&mut self, ctx: &ToolContext, at: Point) {
        self.live = None;
        let Some((id, e)) = retouch_target(ctx, at) else {
            return;
        };
        let fp_model::ElementKind::Bitmap(b) = &e.kind else {
            return;
        };
        let Some(base) = crate::docops::bitmap_image(b) else {
            return;
        };
        let sel = ctx
            .selection
            .pixels
            .clone()
            .filter(|_| ctx.selection.has_pixels());
        // "Entire document": the smudge picks up the visible page; rendered once per stroke
        let page = (self.id == ToolId::Smudge
            && ctx.prefs.get_bool("SmudgeToolUseEntireDocument", false))
        .then(|| {
            crate::commands::smudge_page_source(ctx.doc, ctx.page, ctx.frame, ctx.renderer, &e)
        })
        .flatten()
        .map(std::sync::Arc::new);
        let backdrop = ctx
            .doc
            .pages
            .get(ctx.page)
            .map(|pg| pg.root.background)
            .unwrap_or(fp_color::Argb(0xffff_ffff));
        self.live = Some(RetouchLive {
            base: std::sync::Arc::new(base),
            sel,
            whole: self.id == ToolId::RubberStamp,
            page,
            backdrop,
            preview: crate::paint::PaintPreview {
                stroke: next_live_stroke(),
                target: Some(id.0),
                ..crate::paint::PaintPreview::default()
            },
        });
    }

    /// Recomputes the live preview for the points so far; a mode without preview drops it.
    fn update_live(&mut self, ctx: &ToolContext) {
        let size = params(self.id, ctx).size;
        let stroke = self.stroke_params(ctx, &self.points);
        if let Some(l) = self.live.as_mut()
            && !l.update(&self.points, &stroke, size)
        {
            self.live = None;
        }
    }

    /// Parameters of `bitmap.retouch` for the stroke `pts` (sent on release, also the live preview).
    fn stroke_params(&self, ctx: &ToolContext, pts: &[Point]) -> Value {
        let prm = params(self.id, ctx);
        let mut p = json!({
            "mode": prm.mode, "points": pts.iter().map(|q| pt_json(*q)).collect::<Vec<Value>>(), "size": prm.size,
            "round": prm.round, "soft": prm.soft, "strength": prm.strength, "range": prm.range, "spacing": prm.spacing,
        });
        match self.id {
            ToolId::RubberStamp => {
                let (dx, dy) = self.offset.unwrap_or((0.0, 0.0));
                p["offset"] = json!([dx, dy]);
            }
            ToolId::ReplaceColor => {
                p["from"] = json!(ctx.prefs.get_str("ReplaceColorToolSrcColor", "#00FF00"));
                p["to"] = json!(ctx.prefs.get_str("ReplaceColorToolDestColor", "#FF0000"));
                p["colorize"] = json!(ctx.prefs.get_bool("ReplaceColorToolColorize", false));
                // Toleranz, Stärke 0..255 und „Von: Grafik“ (PAR 05/07–08)
                p["tolerance"] = json!(
                    ctx.prefs
                        .get_int("ReplaceColorToolTolerance", 32)
                        .clamp(0, 255)
                );
                p["replaceStrength"] = json!(
                    ctx.prefs
                        .get_int("ReplaceColorToolStrength", 200)
                        .clamp(0, 255)
                );
                p["fromImage"] = json!(ctx.prefs.get_bool("ReplaceColorToolMode", false));
            }
            ToolId::RedEyeRemoval => {
                p["tolerance"] = json!(ctx.prefs.get_int("RedEyeRemovalToolTolerance", 50));
            }
            ToolId::Smudge => {
                // „Farbe verwischen“ (Farbfeld = Füllfarbe) und „Gesamtes Dokument“ (PAR 04/10, 04/11)
                p["fingerPaint"] = json!(ctx.prefs.get_bool("SmudgeToolFingerPaint", false));
                p["color"] = json!(ctx.fill_color.to_rgba8().to_hex_rgba());
                p["wholeDocument"] =
                    json!(ctx.prefs.get_bool("SmudgeToolUseEntireDocument", false));
                // Firepixel option: keep the smear inside the bitmap edge (FB-15)
                p["confineToBounds"] =
                    json!(ctx.prefs.get_bool("SmudgeToolConfineToBounds", false));
            }
            _ => {}
        }
        p
    }

    fn overlay_for(&self, ctx: &ToolContext) -> Vec<Overlay> {
        let prm = params(self.id, ctx);
        let mut v = Vec::new();
        if self.id == ToolId::RedEyeRemoval {
            // Rote Augen: aufgezogenes Rechteck mit Kreuz (gemessen 05/11)
            if let (true, Some(a), Some(b)) = (self.active, self.points.first(), self.hover) {
                let r = fp_geom::Rect::new(a.x, a.y, b.x, b.y).normalized();
                v.push(Overlay::Rect {
                    rect: r,
                    dashed: false,
                });
                v.push(Overlay::Line {
                    from: Point::new(r.left, r.top),
                    to: Point::new(r.right, r.bottom),
                });
                v.push(Overlay::Line {
                    from: Point::new(r.right, r.top),
                    to: Point::new(r.left, r.bottom),
                });
            }
            return v;
        }
        // the running stroke shows its effect live plus the one tip outline at the pointer, never
        // one outline per stroke point (FB-14a/b, TW-181/182; RE-25)
        if let Some(l) = self.live.as_ref().filter(|_| self.active) {
            v.push(Overlay::Paint(l.preview.clone()));
        }
        if let Some(h) = self.hover.filter(|_| self.target_ok) {
            v.push(Overlay::BrushTip {
                at: h,
                diameter: prm.size,
                round: prm.round,
            });
            if self.id == ToolId::RubberStamp {
                match (self.offset, self.source) {
                    (Some((dx, dy)), _) => v.push(Overlay::Marker {
                        at: Point::new(h.x + dx, h.y + dy),
                    }),
                    (None, Some(s)) => v.push(Overlay::Marker { at: s }),
                    _ => {}
                }
            }
        }
        v
    }
}

/// Target bitmap of a retouch stroke at `at`, chosen like `bitmap.retouch` does: the topmost
/// selected (unlocked) bitmap, otherwise the bitmap under the point.
fn retouch_target(
    ctx: &ToolContext,
    at: Point,
) -> Option<(fp_model::ElementId, std::sync::Arc<fp_model::Element>)> {
    if !ctx.has_document {
        return None;
    }
    crate::paint::target_bitmap(ctx.doc, ctx.page, &ctx.selection.elements).or_else(|| {
        crate::docops::hits_at(
            ctx.doc,
            ctx.page,
            ctx.frame,
            at,
            &crate::geometry::BasicGeometry,
            &crate::docops::HitOptions::default(),
        )
        .into_iter()
        .filter_map(|p| ctx.doc.element_at(&p).cloned())
        .find(|e| matches!(e.kind, fp_model::ElementKind::Bitmap(_)))
        .map(|e| (e.id, e))
    })
}

impl ToolMachine for RetouchTool {
    fn id(&self) -> ToolId {
        self.id
    }

    fn handle(&mut self, ev: &ToolEvent, ctx: &ToolContext) -> ToolResponse {
        if !ctx.has_document {
            return ToolResponse::default();
        }
        if ev.input == ToolInput::Key(ToolKey::Escape) {
            if self.active {
                self.points.clear();
                self.active = false;
                self.live = None;
                return ToolResponse::default();
            }
            // wie im Original: mit Bitmap als Ziel → Zeiger, Bitmap bleibt gewählt (PARITAET_A L39)
            if std::mem::take(&mut self.painted)
                && let Some(r) = super::bitmap::escape_after_stroke(ctx)
            {
                return r;
            }
            return super::bitmap::escape_paint_tool(ctx);
        }
        let ToolInput::Pointer { kind, pos } = &ev.input else {
            return ToolResponse::default();
        };
        self.hover = Some(*pos);
        // while a stroke runs, its target counts (the pointer may leave the bitmap)
        if !self.active {
            self.target_ok =
                self.id == ToolId::RedEyeRemoval || retouch_target(ctx, *pos).is_some();
        }
        match kind {
            PointerKind::Down
                if !ctx.selection.elements.is_empty()
                    && !super::bitmap::bitmap_target(ctx)
                    && !(self.id == ToolId::RubberStamp && ev.mods.alt) =>
            {
                // Vektor o. Ä. gewählt: keine Wirkung, keine Meldung, die Auswahl wird aufgehoben
                // (Original, wie beim Radiergummi; PAR 04/15)
                self.points.clear();
                self.active = false;
                return ToolResponse {
                    commands: vec![Invocation::new(
                        "tool.pick",
                        json!({"command": "select.set", "params": {"ids": []}}),
                    )],
                    cursor: self.cursor(),
                    ..ToolResponse::default()
                };
            }
            PointerKind::Down => {
                if self.id == ToolId::RubberStamp && ev.mods.alt {
                    // Alt+Klick: Quelle setzen
                    self.source = Some(*pos);
                    self.offset = None;
                    return ToolResponse {
                        overlay: self.overlay_for(ctx),
                        cursor: Cursor::Crosshair,
                        ..ToolResponse::default()
                    };
                }
                if self.id == ToolId::RubberStamp && self.source.is_none() {
                    return ToolResponse {
                        status: Some(fp_i18n::t("msg-tool-stamp-needs-source")),
                        cursor: Cursor::NotAllowed,
                        ..ToolResponse::default()
                    };
                }
                if self.id == ToolId::RubberStamp
                    && (self.offset.is_none() || ctx.prefs.get_int("StampToolSource", 0) == 1)
                {
                    let s = self.source.unwrap_or(*pos);
                    self.offset = Some((s.x - pos.x, s.y - pos.y));
                }
                self.points = vec![*pos];
                self.active = true;
                if self.id != ToolId::RedEyeRemoval {
                    self.begin_live(ctx, *pos);
                    self.update_live(ctx);
                }
            }
            PointerKind::Move if self.active && self.id == ToolId::RedEyeRemoval => {}
            PointerKind::Move if self.active => {
                if ev.mods.shift {
                    self.points.truncate(1);
                }
                if self.points.last().is_none_or(|l| dist(*l, *pos) >= 0.5) {
                    self.points.push(*pos);
                    self.update_live(ctx);
                }
            }
            PointerKind::Up if self.active && self.id == ToolId::RedEyeRemoval => {
                // Rote Augen: Rechteck aus Start und Ende; ein Klick ohne Ziehen wirkt nicht
                self.active = false;
                let a = std::mem::take(&mut self.points)
                    .first()
                    .copied()
                    .unwrap_or(*pos);
                if dist(a, *pos) < ctx.drag_threshold() {
                    return ToolResponse::with_overlay(Vec::new(), self.cursor());
                }
                let p = json!({
                    "mode": "redEye", "points": [pt_json(a), pt_json(*pos)],
                    "tolerance": ctx.prefs.get_int("RedEyeRemovalToolTolerance", 50),
                    "strength": ctx.prefs.get_int("RedEyeRemovalToolStrength", 50).clamp(0, 100),
                });
                self.painted = true;
                return ToolResponse {
                    commands: vec![Invocation::new("bitmap.retouch", p)],
                    overlay: Vec::new(),
                    cursor: self.cursor(),
                    ..ToolResponse::default()
                };
            }
            PointerKind::Up if self.active => {
                self.active = false;
                self.live = None;
                if !self.target_ok {
                    // no editable bitmap: no effect, no history step, no message (RE 57 §3.2)
                    self.points.clear();
                    self.target_ok = retouch_target(ctx, *pos).is_some();
                    return ToolResponse::with_overlay(self.overlay_for(ctx), self.cursor());
                }
                if ev.mods.shift {
                    self.points.truncate(1);
                }
                if self.points.last().is_none_or(|l| dist(*l, *pos) > 0.0) {
                    self.points.push(*pos);
                }
                let pts = std::mem::take(&mut self.points);
                self.painted = true;
                let p = self.stroke_params(ctx, &pts);
                self.target_ok = retouch_target(ctx, *pos).is_some();
                return ToolResponse {
                    commands: vec![Invocation::new("bitmap.retouch", p)],
                    overlay: self.overlay_for(ctx),
                    cursor: self.cursor(),
                    ..ToolResponse::default()
                };
            }
            _ => {}
        }
        ToolResponse::with_overlay(self.overlay_for(ctx), self.cursor())
    }

    fn overlay(&self) -> Vec<Overlay> {
        if let Some(l) = self.live.as_ref().filter(|_| self.active) {
            return vec![Overlay::Paint(l.preview.clone())];
        }
        if self.points.is_empty() || self.id == ToolId::RedEyeRemoval {
            Vec::new()
        } else {
            vec![Overlay::Polyline {
                points: self.points.clone(),
                closed: false,
            }]
        }
    }

    fn feedback(&self, ctx: &ToolContext) -> Vec<Overlay> {
        self.overlay_for(ctx)
    }
}
