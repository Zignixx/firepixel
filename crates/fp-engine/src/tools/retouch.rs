//! Pinselbasierte Bitmapwerkzeuge (10 §2.6): Weichzeichner, Scharfzeichner, Abwedler,
//! Nachbelichter, Verwischen, Stempel, Farbe ersetzen, Rote Augen entfernen. Ziehen sammelt die
//! Strichpunkte (Umschalt = gerade Linie), beim Loslassen wirkt `bitmap.retouch` auf die Bitmap.
//! Durchmesser, Kante, Form, Intensität/Belichtung/Druck, Bereich und Toleranz kommen aus den
//! Voreinstellungen des Werkzeugs (`BlurToolDiameter` …), die der Eigenschaften-Inspektor setzt.
//! Stempel: Alt+Klick setzt die Quelle; der Versatz zur ersten Malstelle bleibt erhalten
//! (`StampToolSource` = 0 ausgerichtet, 1 fest).

use super::{dist, pt_json, Cursor, Invocation, Overlay, PointerKind, ToolContext, ToolEvent, ToolId, ToolInput, ToolKey, ToolMachine, ToolResponse};
use fp_geom::Point;
use serde_json::{json, Value};

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
    let size = if id == ToolId::RubberStamp { p.get_int("StampToolSize", 16) } else { p.get_int(&key("Diameter"), 13) };
    let soft_raw = if id == ToolId::RubberStamp { p.get_int("StampToolEdge", 500) } else { p.get_int(&key("SoftEdge"), 1000) };
    let strength = match id {
        ToolId::Blur | ToolId::Sharpen => p.get_int(&key("Intensity"), 50),
        ToolId::Dodge | ToolId::Burn => p.get_int(&key("Exposure"), 50),
        ToolId::Smudge => p.get_int(&key("Pressure"), 50),
        ToolId::RedEyeRemoval => p.get_int(&key("Strength"), 50),
        // Stärke im Inspektor 0..255, Werksvorgabe 200 (`ReplaceColorToolStrength`, PARITAET_A W8);
        // Annahme: 200 = volle Wirkung (Skala des Originals nicht gemessen)
        ToolId::ReplaceColor => p.get_int(&key("Strength"), 200) / 2,
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
}

impl RetouchTool {
    /// Neues Werkzeug.
    pub fn new(id: ToolId) -> RetouchTool {
        RetouchTool { id, points: Vec::new(), active: false, hover: None, source: None, offset: None, painted: false }
    }

    fn cursor(&self) -> Cursor {
        if self.id == ToolId::RubberStamp { Cursor::Stamp } else { Cursor::Brush }
    }

    fn overlay_for(&self, ctx: &ToolContext) -> Vec<Overlay> {
        let prm = params(self.id, ctx);
        let mut v = Vec::new();
        if !self.points.is_empty() {
            v.push(Overlay::Stroke { points: self.points.clone(), diameter: prm.size });
        }
        if let Some(h) = self.hover {
            v.push(Overlay::BrushTip { at: h, diameter: prm.size, round: prm.round });
            if self.id == ToolId::RubberStamp {
                match (self.offset, self.source) {
                    (Some((dx, dy)), _) => v.push(Overlay::Marker { at: Point::new(h.x + dx, h.y + dy) }),
                    (None, Some(s)) => v.push(Overlay::Marker { at: s }),
                    _ => {}
                }
            }
        }
        v
    }
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
        let ToolInput::Pointer { kind, pos } = &ev.input else { return ToolResponse::default() };
        self.hover = Some(*pos);
        match kind {
            PointerKind::Down if !ctx.selection.elements.is_empty() && !super::bitmap::bitmap_target(ctx) && !(self.id == ToolId::RubberStamp && ev.mods.alt) => {
                // Vektor o. Ä. gewählt: keine Wirkung, keine Meldung, die Auswahl wird aufgehoben
                // (Original, wie beim Radiergummi; PAR 04/15)
                self.points.clear();
                self.active = false;
                return ToolResponse {
                    commands: vec![Invocation::new("tool.pick", json!({"command": "select.set", "params": {"ids": []}}))],
                    cursor: self.cursor(),
                    ..ToolResponse::default()
                };
            }
            PointerKind::Down => {
                if self.id == ToolId::RubberStamp && ev.mods.alt {
                    // Alt+Klick: Quelle setzen
                    self.source = Some(*pos);
                    self.offset = None;
                    return ToolResponse { overlay: self.overlay_for(ctx), cursor: Cursor::Crosshair, ..ToolResponse::default() };
                }
                if self.id == ToolId::RubberStamp && self.source.is_none() {
                    return ToolResponse {
                        status: Some(fp_i18n::t("msg-tool-stamp-needs-source")),
                        cursor: Cursor::NotAllowed,
                        ..ToolResponse::default()
                    };
                }
                if self.id == ToolId::RubberStamp && (self.offset.is_none() || ctx.prefs.get_int("StampToolSource", 0) == 1) {
                    let s = self.source.unwrap_or(*pos);
                    self.offset = Some((s.x - pos.x, s.y - pos.y));
                }
                self.points = vec![*pos];
                self.active = true;
            }
            PointerKind::Move if self.active => {
                if ev.mods.shift {
                    self.points.truncate(1);
                }
                if self.points.last().is_none_or(|l| dist(*l, *pos) >= 0.5) {
                    self.points.push(*pos);
                }
            }
            PointerKind::Up if self.active => {
                self.active = false;
                if ev.mods.shift {
                    self.points.truncate(1);
                }
                if self.points.last().is_none_or(|l| dist(*l, *pos) > 0.0) {
                    self.points.push(*pos);
                }
                let pts = std::mem::take(&mut self.points);
                self.painted = true;
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
                    }
                    ToolId::RedEyeRemoval => {
                        p["tolerance"] = json!(ctx.prefs.get_int("RedEyeRemovalToolTolerance", 50));
                    }
                    ToolId::Smudge => {
                        // „Farbe verwischen“ (Farbfeld = Füllfarbe) und „Gesamtes Dokument“ (PAR 04/10, 04/11)
                        p["fingerPaint"] = json!(ctx.prefs.get_bool("SmudgeToolFingerPaint", false));
                        p["color"] = json!(ctx.fill_color.to_rgba8().to_hex_rgba());
                        p["wholeDocument"] = json!(ctx.prefs.get_bool("SmudgeToolUseEntireDocument", false));
                    }
                    _ => {}
                }
                return ToolResponse { commands: vec![Invocation::new("bitmap.retouch", p)], overlay: self.overlay_for(ctx), cursor: self.cursor(), ..ToolResponse::default() };
            }
            _ => {}
        }
        ToolResponse::with_overlay(self.overlay_for(ctx), self.cursor())
    }

    fn overlay(&self) -> Vec<Overlay> {
        if self.points.is_empty() { Vec::new() } else { vec![Overlay::Polyline { points: self.points.clone(), closed: false }] }
    }

    fn feedback(&self, ctx: &ToolContext) -> Vec<Overlay> {
        self.overlay_for(ctx)
    }
}
