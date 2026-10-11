// Weitere Formen des Bedienfelds (Firepixel, gemeinsames Skript; `__miscKind` steht davor):
// Sprechblase, Gedankenblase, Anmerkung, Rahmen 1–3, Registerkarten, Lineal, Perspektive,
// Gliederpuppe, Flussdiagramm, Kalender, Uhr, Dateiinfo, Zeitstempel, Anregung.
var MK = typeof __miscKind === "string" ? __miscKind : "talking";
var MONTHS = ["Januar", "Februar", "März", "April", "Mai", "Juni", "Juli", "August", "September", "Oktober", "November", "Dezember"];

function bubbleEllipse(cx, cy, rx, ry, tipAng, tipW) {
  // Ellipse mit Lücke für den Zeiger um den Winkel tipAng (Bogenmaß)
  var a0 = tipAng + tipW, a1 = tipAng - tipW + 2 * Math.PI;
  return SS.arc(cx, cy, rx, ry, a0, a1);
}

function rect(el, p, x, y, w, h) { SS.contour(el, p, [[x, y], [x + w, y], [x + w, y + h], [x, y + h]], true); }

var FLOW = ["Alternativer Prozess", "Karte", "Sortieren", "Verbindung", "Daten", "Entscheidung", "Verzögerung", "Speicher mit direktem Zugriff", "Anzeige", "Dokument",
  "Extrahieren", "Interner Speicher", "Magnetplatte", "Manuelle Eingabe", "Manuelle Steuerung", "Verbinden", "Mehrere Dokumente", "Verbindung zu anderer Seite", "Oder",
  "Vordefinierter Prozess", "Vorbereitung", "Prozess", "Lochstreifen", "Speicher mit sequentiellem Zugriff", "Sortieren nach", "Gespeicherte Daten", "Summierglied", "Terminator"];

// Flussdiagramm-Symbol k in den Rahmen (x, y, w, h) als Konturenliste
function flowSymbol(el, p, k, x, y, w, h) {
  var r = Math.min(w, h) / 2, cx = x + w / 2, cy = y + h / 2;
  switch (k) {
    case 0: { var q = h / 4; SS.contour(el, p, [[x + q, y], [x + w - q, y, x + w - q, y, x + w - q + q * SS.K, y], [x + w, y + q, x + w, y + q - q * SS.K, x + w, y + q], [x + w, y + h - q], [x + w - q, y + h, x + w - q + q * SS.K, y + h, x + w - q, y + h], [x + q, y + h], [x, y + h - q, x, y + h - q + q * SS.K, x, y + h - q], [x, y + q, x, y + q, x, y + q - q * SS.K]], true); break; }
    case 1: SS.contour(el, p, [[x + w * 0.25, y], [x + w, y], [x + w, y + h], [x, y + h], [x, y + h * 0.3]], true); break;
    case 2: case 5: SS.contour(el, p, [[cx, y], [x + w, cy], [cx, y + h], [x, cy]], true); if (k === 2) SS.contour(el, p, [[x, cy], [x + w, cy]], false); break;
    case 3: SS.ellipse(el, p, cx, cy, r, r); break;
    case 4: SS.contour(el, p, [[x + w * 0.2, y], [x + w, y], [x + w * 0.8, y + h], [x, y + h]], true); break;
    case 6: SS.contour(el, p, [[x, y], [x + w - r, y]].concat(SS.arc(x + w - r, cy, r, h / 2, -Math.PI / 2, Math.PI / 2)).concat([[x, y + h]]), true); break;
    case 7: SS.ellipse(el, p, cx, cy, w / 2, h / 2); SS.contour(el, p, [[x + w * 0.75, y], [x + w * 0.75, y + h]], false); break;
    case 8: SS.contour(el, p, [[x + w * 0.2, y], [x + w * 0.8, y], [x + w, cy], [x + w * 0.8, y + h], [x + w * 0.2, y + h], [x, cy]], true); break;
    case 9: SS.contour(el, p, [[x, y], [x + w, y], [x + w, y + h * 0.8], [x, y + h * 0.8, x + w * 0.7, y + h * 1.1, x + w * 0.3, y + h * 0.5]], true); break;
    case 10: SS.contour(el, p, [[cx, y], [x + w, y + h], [x, y + h]], true); break;
    case 11: rect(el, p, x, y, w, h); SS.contour(el, p, [[x + w * 0.15, y], [x + w * 0.15, y + h]], false); SS.contour(el, p, [[x, y + h * 0.2], [x + w, y + h * 0.2]], false); break;
    case 12: SS.ellipse(el, p, cx, y + h * 0.15, w / 2, h * 0.15); SS.contour(el, p, [[x, y + h * 0.15], [x + w, y + h * 0.15], [x + w, y + h * 0.85], [x, y + h * 0.85]], true); break;
    case 13: SS.contour(el, p, [[x, y + h * 0.3], [x + w, y], [x + w, y + h], [x, y + h]], true); break;
    case 14: SS.contour(el, p, [[x, y], [x + w, y], [x + w * 0.8, y + h], [x + w * 0.2, y + h]], true); break;
    case 15: SS.contour(el, p, [[x, y], [x + w, y], [cx, y + h]], true); break;
    case 16: for (var o = 2; o >= 0; o--) rect(el, p, x + o * 4, y + (2 - o) * 4, w - 8, h - 8); break;
    case 17: SS.contour(el, p, [[x, y], [x + w, y], [x + w, y + h * 0.7], [cx, y + h], [x, y + h * 0.7]], true); break;
    case 18: SS.ellipse(el, p, cx, cy, r, r); SS.contour(el, p, [[cx, cy - r], [cx, cy + r]], false); SS.contour(el, p, [[cx - r, cy], [cx + r, cy]], false); break;
    case 19: rect(el, p, x, y, w, h); SS.contour(el, p, [[x + w * 0.1, y], [x + w * 0.1, y + h]], false); SS.contour(el, p, [[x + w * 0.9, y], [x + w * 0.9, y + h]], false); break;
    case 20: SS.contour(el, p, [[x + w * 0.2, y], [x + w * 0.8, y], [x + w, cy], [x + w * 0.8, y + h], [x + w * 0.2, y + h], [x, cy]], true); break;
    case 21: rect(el, p, x, y, w, h); break;
    case 22: SS.contour(el, p, [[x, y + h * 0.1, x, y + h * 0.1, x + w * 0.3, y - h * 0.1], [x + w, y + h * 0.1, x + w * 0.7, y + h * 0.3, x + w, y + h * 0.1], [x + w, y + h * 0.9, x + w, y + h * 0.9, x + w * 0.7, y + h * 1.1], [x, y + h * 0.9, x + w * 0.3, y + h * 0.7, x, y + h * 0.9]], true); break;
    case 23: SS.ellipse(el, p, cx, cy, r, r); SS.contour(el, p, [[cx, y + h], [x + w, y + h]], false); break;
    case 24: SS.contour(el, p, [[cx, y], [x + w, cy], [x, cy]], true); SS.contour(el, p, [[x, cy], [x + w, cy], [cx, y + h]], true); break;
    case 25: SS.contour(el, p, [[x + w * 0.2, y], [x + w, y, x + w, y, x + w * 0.85, y + h * 0.2], [x + w, y + h, x + w * 0.85, y + h * 0.8, x + w, y + h], [x + w * 0.2, y + h, x + w * 0.2, y + h, x - w * 0.05, y + h * 0.8], [x + w * 0.2, y, x - w * 0.05, y + h * 0.2, x + w * 0.2, y]], true); break;
    case 26: SS.ellipse(el, p, cx, cy, r, r); SS.contour(el, p, [[cx - r * 0.7, cy - r * 0.7], [cx + r * 0.7, cy + r * 0.7]], false); SS.contour(el, p, [[cx + r * 0.7, cy - r * 0.7], [cx - r * 0.7, cy + r * 0.7]], false); break;
    default: { var rr = h / 2; SS.contour(el, p, [[x + rr, y]].concat(SS.arc(x + w - rr, cy, rr, rr, -Math.PI / 2, Math.PI / 2)).concat(SS.arc(x + rr, cy, rr, rr, Math.PI / 2, 3 * Math.PI / 2)), true); }
  }
}

function now() { return new Date(); }

var DEFS = {
  talking: {
    init: function (cd, at, r) { cd.cx = r ? (r.left + r.right) / 2 : at.x; cd.cy = r ? (r.top + r.bottom) / 2 : at.y; cd.rx = r ? (r.right - r.left) / 2 : 100; cd.ry = r ? (r.bottom - r.top) / 2 * 0.8 : 80; cd.tx = cd.cx - cd.rx * 0.9; cd.ty = cd.cy + cd.ry * 1.5; },
    drag: function (cd, i, m, d, st) {
      if (i === 0) { cd.rx = Math.max(4, Math.abs(m.x - cd.cx) / 0.6); cd.ry = Math.max(4, Math.abs(m.y - cd.cy) / 0.8); }
      if (i === 1) { cd.tx = m.x; cd.ty = m.y; }
    },
    build: function (el, cd, thinking) {
      var ta = Math.atan2((cd.ty - cd.cy) / cd.ry, (cd.tx - cd.cx) / cd.rx), p = SS.path(el, 0), n = 1;
      if (!thinking) {
        var arc = bubbleEllipse(cd.cx, cd.cy, cd.rx, cd.ry, ta, 0.18);
        SS.contour(el, p, arc.concat([[cd.tx, cd.ty]]), true);
      } else {
        var w = [];
        for (var i = 0; i < 12; i++) {
          var a = i * Math.PI / 6, b = (i + 1) * Math.PI / 6, mid = (a + b) / 2;
          var q0 = { x: cd.cx + cd.rx * Math.cos(a), y: cd.cy + cd.ry * Math.sin(a) }, q1 = { x: cd.cx + cd.rx * Math.cos(b), y: cd.cy + cd.ry * Math.sin(b) };
          var out = { x: cd.cx + cd.rx * 1.25 * Math.cos(mid), y: cd.cy + cd.ry * 1.25 * Math.sin(mid) };
          w.push([q0.x, q0.y, q0.x, q0.y, q0.x + (out.x - q0.x) * 0.6, q0.y + (out.y - q0.y) * 0.6]);
          w[w.length - 1].next = [q1.x + (out.x - q1.x) * 0.6, q1.y + (out.y - q1.y) * 0.6];
        }
        for (var j = 0; j < w.length; j++) { var nx = w[(j + 1) % w.length]; nx[2] = w[j].next[0]; nx[3] = w[j].next[1]; }
        SS.contour(el, p, w.map(function (q) { return [q[0], q[1], q[2], q[3], q[4], q[5]]; }), true);
        // kleine Kreise zum Zeiger
        for (var k = 1; k <= 3; k++) {
          var f = 1 + k * 0.25, ex = cd.cx + (cd.tx - cd.cx) * (0.55 + k * 0.15), ey = cd.cy + (cd.ty - cd.cy) * (0.55 + k * 0.15), rr = Math.min(cd.rx, cd.ry) * 0.12 / f;
          SS.ellipse(el, SS.path(el, n++), ex, ey, rr, rr);
        }
      }
      SS.trim(el, n);
      SS.cp(el, 0, cd.cx + cd.rx * 0.6 * Math.cos(2.4), cd.cy + cd.ry * 0.8 * Math.sin(2.4) * -1 + cd.ry * 0.0, "Größe", Math.round(cd.rx));
      SS.cp(el, 1, cd.tx, cd.ty, "Zeiger", "");
      SS.cpTrim(el, 2);
    }
  },
  annotation: {
    init: function (cd, at, r) { cd.x = r ? r.left : at.x; cd.y = r ? r.top : at.y; cd.w = r ? r.right - r.left : 160; cd.h = r ? r.bottom - r.top : 60; cd.tx = cd.x - 40; cd.ty = cd.y + cd.h + 50; cd.text = "Anmerkung"; },
    drag: function (cd, i, m, d, st) {
      if (i === 0) { cd.tx = m.x; cd.ty = m.y; }
      if (i === 1) { cd.w = Math.max(20, st.w + m.x - d.x); cd.h = Math.max(16, st.h + m.y - d.y); }
    },
    build: function (el, cd) {
      var p = SS.path(el, 0);
      SS.contour(el, p, [[cd.x, cd.y], [cd.x + cd.w, cd.y], [cd.x + cd.w, cd.y + cd.h], [cd.x, cd.y + cd.h]], true);
      var l = SS.path(el, 1);
      SS.style(l, false, 1);
      var ax = SS.clamp(cd.tx, cd.x, cd.x + cd.w), ay = SS.clamp(cd.ty, cd.y, cd.y + cd.h);
      SS.contour(el, l, [[ax, ay], [cd.tx, cd.ty]], false);
      SS.ellipse(el, l, cd.tx, cd.ty, 3, 3);
      SS.text(el, 2, cd.text, cd.x + 8, cd.y + cd.h / 2 + 6, 16, "#000000");
      SS.trim(el, 3);
      SS.cp(el, 0, cd.tx, cd.ty, "Zeiger");
      SS.cp(el, 1, cd.x + cd.w, cd.y + cd.h, "Größe ändern");
      SS.cpTrim(el, 2);
    }
  },
  frame: {
    init: function (cd, at, r) { cd.x = r ? r.left : at.x - 60; cd.y = r ? r.top : at.y - 60; cd.w = r ? r.right - r.left : 120; cd.h = r ? r.bottom - r.top : 120; cd.fw = 40; cd.corner = 0; cd.color = "#8b5a2b"; },
    click: function (cd, i) { if (i === 4) cd.corner = (cd.corner + 1) % 2; if (i === 2) cd.color = cd.color === "#8b5a2b" ? "#c0c0c0" : cd.color === "#c0c0c0" ? "#d4af37" : "#8b5a2b"; },
    drag: function (cd, i, m, d, st) {
      if (i === 0 || i === 1) cd.fw = SS.clamp(st.fw + (i === 0 ? -(m.x - d.x) : (m.x - d.x)) * (i === 0 ? 1 : -1), 1, Math.min(cd.w, cd.h) / 2);
      if (i === 3) { cd.x = st.x + m.x - d.x; cd.y = st.y + m.y - d.y; }
      if (i === 5) { cd.w = Math.max(4, st.w + m.x - d.x); cd.h = Math.max(4, st.h + m.y - d.y); }
    },
    build: function (el, cd) {
      var p = SS.path(el, 0), x = cd.x, y = cd.y, w = cd.w, h = cd.h, f = cd.fw;
      SS.style(p, true, 1, null, cd.color);
      p.pathAttributes.fillColor = cd.color;
      rect(el, p, x, y, w, h);
      if (cd.corner === 1) SS.contour(el, p, [[x + f, y + f + 8], [x + f + 8, y + f], [x + w - f - 8, y + f], [x + w - f, y + f + 8], [x + w - f, y + h - f - 8], [x + w - f - 8, y + h - f], [x + f + 8, y + h - f], [x + f, y + h - f - 8]], true);
      else rect(el, p, x + f, y + f, w - 2 * f, h - 2 * f);
      SS.trim(el, 1);
      SS.cp(el, 0, x + f, y + h / 2, "Rahmenbreite");
      SS.cp(el, 1, x, y + h / 2, "Rahmenbreite");
      SS.cp(el, 2, x + w / 2, y + h, "Klicken: Farbe wechseln", "", true);
      SS.cp(el, 3, x + w / 2, y + h / 2, "Ziehen: verschieben", "");
      SS.cp(el, 4, x, y, "Klicken: Ecken umschalten", cd.corner);
      SS.cp(el, 5, x + w + 10, y + h + 10, "Größe ändern");
      SS.cpTrim(el, 6);
    }
  },
  frame2: {
    init: function (cd, at, r) { cd.x = r ? r.left : at.x - 35; cd.y = r ? r.top : at.y - 35; cd.w = r ? r.right - r.left : 70; cd.h = r ? r.bottom - r.top : 70; cd.fw = 10; cd.over = 10; cd.margin = 5; },
    drag: function (cd, i, m, d, st) {
      if (i === 0) { cd.x = st.x + m.x - d.x; cd.y = st.y + m.y - d.y; }
      if (i === 1) cd.fw = SS.clamp(st.fw - (m.x - d.x), 1, cd.w / 2);
      if (i === 2) cd.over = SS.clamp(st.over - (m.y - d.y), 0, 200);
      if (i === 3) { cd.w = Math.max(4, st.w + m.x - d.x); cd.h = Math.max(4, st.h + m.y - d.y); }
    },
    build: function (el, cd) {
      var x = cd.x, y = cd.y, w = cd.w, h = cd.h, f = cd.fw, o = cd.over, n = 0;
      [x, x + f, x + w - f, x + w].forEach(function (lx) { var p = SS.path(el, n++); SS.style(p, false, 1); SS.contour(el, p, [[lx, y + o - o], [lx, y + h]], false); });
      [y, y + f, y + h - f, y + h].forEach(function (ly) { var p = SS.path(el, n++); SS.style(p, false, 1); SS.contour(el, p, [[x - o, ly], [x + w + o, ly]], false); });
      SS.trim(el, n);
      SS.cp(el, 0, x + w / 2, y + h / 2, "Ziehen: verschieben");
      SS.cp(el, 1, x, y + h / 2, "Rahmenbreite");
      SS.cp(el, 2, x - o, y, "Überhang");
      SS.cp(el, 3, x + w + 10, y + h + 10, "Größe ändern");
      SS.cpTrim(el, 4);
    }
  },
  frame3: {
    init: function (cd, at, r) { cd.x = r ? r.left : at.x - 45; cd.y = r ? r.top : at.y - 45; cd.w = r ? r.right - r.left : 90; cd.h = r ? r.bottom - r.top : 90; cd.off = 5; cd.corner = 15; },
    drag: function (cd, i, m, d, st) {
      if (i === 0) { cd.x = st.x + m.x - d.x; cd.y = st.y + m.y - d.y; }
      if (i === 1) cd.off = SS.clamp(st.off + m.x - d.x, 0, cd.w / 3);
      if (i === 2) cd.corner = SS.clamp(st.corner + m.y - d.y, 0, Math.min(cd.w, cd.h) / 2);
      if (i === 3) { cd.w = Math.max(4, st.w + m.x - d.x); cd.h = Math.max(4, st.h + m.y - d.y); }
    },
    build: function (el, cd) {
      function notched(p, x, y, w, h, c) {
        SS.contour(el, p, [[x + c, y], [x + w - c, y], [x + w - c, y + c], [x + w, y + c], [x + w, y + h - c], [x + w - c, y + h - c], [x + w - c, y + h], [x + c, y + h], [x + c, y + h - c], [x, y + h - c], [x, y + c], [x + c, y + c]], true);
      }
      var a = SS.path(el, 0), b = SS.path(el, 1);
      SS.style(a, false, 1); SS.style(b, false, 1);
      notched(a, cd.x + cd.off, cd.y + cd.off, cd.w - 2 * cd.off, cd.h - 2 * cd.off, Math.max(0, cd.corner - cd.off));
      notched(b, cd.x, cd.y, cd.w, cd.h, cd.corner);
      SS.trim(el, 2);
      SS.cp(el, 0, cd.x + cd.w / 2, cd.y + cd.h / 2, "Ziehen: verschieben");
      SS.cp(el, 1, cd.x + cd.off, cd.y + cd.h / 2, "Rahmenversatz");
      SS.cp(el, 2, cd.x, cd.y + cd.corner, "Eckgröße");
      SS.cp(el, 3, cd.x + cd.w + 10, cd.y + cd.h + 10, "Größe ändern");
      SS.cpTrim(el, 4);
    }
  },
  tabs: {
    init: function (cd, at, r) { cd.x = r ? r.left : at.x - 150; cd.y = r ? r.top : at.y; cd.tw = 100; cd.th = 30; cd.count = 3; cd.hue = 200; },
    drag: function (cd, i, m, d, st) {
      if (i >= 2) cd.hue = ((st.hue + (m.x - d.x)) % 360 + 360) % 360;
    },
    click: function (cd, i) { if (i === 0) cd.count = Math.max(1, cd.count - 1); if (i === 1) cd.count = Math.min(20, cd.count + 1); },
    build: function (el, cd) {
      var s = 0.6, l = 0.6, h = cd.hue / 60, c = (1 - Math.abs(2 * l - 1)) * s, xx = c * (1 - Math.abs(h % 2 - 1)), rgb = [0, 0, 0];
      var idx = Math.floor(h) % 6; rgb = [[c, xx, 0], [xx, c, 0], [0, c, xx], [0, xx, c], [xx, 0, c], [c, 0, xx]][idx];
      var mm = l - c / 2, col = "#" + rgb.map(function (v) { var n = Math.round((v + mm) * 255); return (n < 16 ? "0" : "") + n.toString(16); }).join("");
      for (var t = 0; t < cd.count; t++) {
        var x = cd.x + t * cd.tw, p = SS.path(el, t), k = cd.th / 3;
        SS.contour(el, p, [[x, cd.y + cd.th], [x, cd.y + k, x, cd.y + k, x, cd.y + k - k * SS.K], [x + k, cd.y, x + k - k * SS.K, cd.y, x + k, cd.y], [x + cd.tw - k, cd.y, x + cd.tw - k, cd.y, x + cd.tw - k + k * SS.K, cd.y], [x + cd.tw, cd.y + k, x + cd.tw, cd.y + k - k * SS.K, x + cd.tw, cd.y + k], [x + cd.tw, cd.y + cd.th]], true);
        p.pathAttributes.fillColor = col;
      }
      SS.trim(el, cd.count);
      SS.cp(el, 0, cd.x - 5, cd.y + cd.th / 2, "Klicken: Registerkarte löschen", "", true);
      SS.cp(el, 1, cd.x + cd.count * cd.tw + 5, cd.y + cd.th / 2, "Klicken: Registerkarte hinzufügen", "", true);
      for (var j = 0; j < cd.count; j++) SS.cp(el, 2 + j, cd.x + j * cd.tw + cd.tw * 0.55, cd.y - 5, "Farbton: " + Math.round(cd.hue), "", true);
      SS.cpTrim(el, 2 + cd.count);
    }
  },
  ruler: {
    init: function (cd, at, r) { cd.x1 = r ? r.left : at.x; cd.y1 = r ? r.top : at.y; cd.x2 = r ? r.right : at.x + 100; cd.y2 = cd.y1; },
    drag: function (cd, i, m, d, st) {
      if (i === 0) { cd.x1 = m.x; cd.y1 = m.y; }
      if (i === 1) { cd.x2 = m.x; cd.y2 = m.y; }
      if (i >= 2) { var dx = m.x - d.x, dy = m.y - d.y; cd.x1 = st.x1 + dx; cd.y1 = st.y1 + dy; cd.x2 = st.x2 + dx; cd.y2 = st.y2 + dy; }
    },
    build: function (el, cd) {
      var a = { x: cd.x1, y: cd.y1 }, b = { x: cd.x2, y: cd.y2 }, len = SS.dist(a, b), ang = SS.ang(a, b), c = Math.cos(ang), s = Math.sin(ang);
      function P(u, v) { return [a.x + u * c - v * s, a.y + u * s + v * c]; }
      var body = SS.path(el, 1), ticks = SS.path(el, 0);
      SS.style(ticks, false, 1);
      SS.contour(el, body, [P(-8, -8), P(len + 8, -8), P(len + 8, 8), P(-8, 8)], true);
      for (var u = 0; u <= len + 0.01; u += 10) { var big = Math.round(u) % 50 === 0; SS.contour(el, ticks, [P(u, -8), P(u, big ? 0 : -4)], false); }
      SS.trim(el, 2);
      var tip = "Länge: " + Math.round(len) + " px Winkel: " + Math.round(-ang * 180 / Math.PI) + "°";
      SS.cp(el, 0, a.x, a.y, tip, "", true);
      SS.cp(el, 1, b.x, b.y, tip, "", true);
      var q = P(len + 8, 8), r2 = P(-8, 8);
      SS.cp(el, 2, q[0], q[1], "Verschieben", "", true);
      SS.cp(el, 3, r2[0], r2[1], "Verschieben", "", true);
      SS.cpTrim(el, 4);
    }
  },
  perspective: {
    init: function (cd, at, r) { var W = (__in.docWidth || 600), H = (__in.docHeight || 600); cd.vx = r ? r.left : 10; cd.vy = r ? r.top : H * 0.8; cd.hx = W / 2; cd.n = 5; cd.W = W; cd.H = H; },
    drag: function (cd, i, m, d, st) {
      if (i === 1) { cd.vx = m.x; cd.vy = m.y; }
      if (i === 0) cd.hx = m.x;
      if (i === 2 || i === 3 || i === 4) cd.n = SS.clamp(Math.round(st.n + (m.x - d.x) / 20), 1, 40);
    },
    build: function (el, cd) {
      var v = { x: cd.vx, y: cd.vy }, k = 0, W = cd.W, H = cd.H;
      for (var i = 0; i <= cd.n * 2; i++) {
        var y = H * i / (cd.n * 2), p = SS.path(el, k++);
        SS.style(p, false, 1, "#3080ff");
        SS.contour(el, p, [[v.x, v.y], [W, y]], false);
      }
      for (var j = 1; j <= cd.n; j++) {
        var x = v.x + (cd.hx - v.x) * j / cd.n, q = SS.path(el, k++);
        SS.style(q, false, 1, "#3080ff");
        SS.contour(el, q, [[x, 0], [x, H]], false);
      }
      SS.trim(el, k);
      SS.cp(el, 0, cd.hx, H, "Vertikale Ebene", 1, true);
      SS.cp(el, 1, v.x, v.y, "Fluchtpunkt", 1, true);
      SS.cp(el, 2, cd.hx / 2, H, "Raster anpassen (waagerecht ziehen)", "", true);
      SS.cp(el, 3, cd.hx, H * 0.75, "Raster anpassen (waagerecht ziehen)", "", true);
      SS.cp(el, 4, cd.hx / 2, 0, "Gitter anpassen (waagerecht ziehen)", "", true);
      SS.cpTrim(el, 5);
    }
  },
  posable: {
    init: function (cd, at, r) {
      var x = r ? (r.left + r.right) / 2 : at.x, y = r ? r.top + (r.bottom - r.top) * 0.36 : at.y - 20, s = r ? (r.bottom - r.top) / 160 : 1;
      var P = [[0, 0], [0, -35], [0, -60], [-20, -30], [-20, 0], [-20, 20], [-20, 33.3], [20, -30], [20, 0], [20, 20], [20, 33.3], [-10, 20], [-10, 60], [-10, 100], [-26.7, 100], [10, 20], [10, 60], [10, 100], [26.7, 100]];
      cd.pts = P.map(function (q) { return [x + q[0] * s, y + q[1] * s]; });
    },
    drag: function (cd, i, m, d, st) {
      var dx = m.x - d.x, dy = m.y - d.y, pts = SS.copy(st.pts);
      // Gelenk mit allen abhängigen Punkten verschieben (Figur = alles)
      var kids = { 0: "all", 1: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10], 3: [3, 4, 5, 6], 4: [4, 5, 6], 5: [5, 6], 7: [7, 8, 9, 10], 8: [8, 9, 10], 9: [9, 10], 11: [11, 12, 13, 14], 12: [12, 13, 14], 13: [13, 14], 15: [15, 16, 17, 18], 16: [16, 17, 18], 17: [17, 18] };
      var list = kids[i] === "all" ? pts.map(function (_, k) { return k; }) : (kids[i] || [i]);
      list.forEach(function (k) { pts[k][0] += dx; pts[k][1] += dy; });
      cd.pts = pts;
    },
    build: function (el, cd) {
      var p = cd.pts, n = 0, names = ["Figur", "Oberkörper", "Kopf", "Linke Schulter", "Linker Ellenbogen", "Linkes Handgelenk", "Linke Hand", "Rechte Schulter", "Rechter Ellenbogen", "Rechtes Handgelenk", "Rechte Hand", "Linke Hüfte", "Linkes Knie", "Linker Knöchel", "Linker Fuß", "Rechte Hüfte", "Rechtes Knie", "Rechter Knöchel", "Rechter Fuß"];
      var bones = [[0, 1], [1, 3], [1, 7], [3, 4], [4, 5], [5, 6], [7, 8], [8, 9], [9, 10], [0, 11], [0, 15], [11, 12], [12, 13], [13, 14], [15, 16], [16, 17], [17, 18]];
      var body = SS.path(el, n++);
      SS.style(body, false, 3);
      bones.forEach(function (b) { SS.contour(el, body, [[p[b[0]][0], p[b[0]][1]], [p[b[1]][0], p[b[1]][1]]], false); });
      var head = SS.path(el, n++);
      SS.ellipse(el, head, p[2][0], p[2][1] - 8, 10, 12);
      SS.trim(el, n);
      for (var i = 0; i < p.length; i++) SS.cp(el, i, p[i][0], p[i][1], names[i], "p" + i, true);
      SS.cpTrim(el, p.length);
    }
  },
  flowchart: {
    init: function (cd, at, r) { cd.x = r ? r.left : at.x - 210; cd.y = r ? r.top : at.y - 100; cd.sel = -1; cd.w = 96; cd.h = 64; },
    click: function (cd, i) { if (cd.sel < 0 && i >= 0 && i < FLOW.length) cd.sel = i; },
    drag: function (cd, i, m, d, st) { if (cd.sel >= 0 && i === 0) { cd.w = Math.max(8, st.w + m.x - d.x); cd.h = Math.max(8, st.h + m.y - d.y); } },
    build: function (el, cd) {
      if (cd.sel < 0) {
        for (var k = 0; k < FLOW.length; k++) {
          var cx = cd.x + 20 + (k % 7) * 60, cy = cd.y + 20 + Math.floor(k / 7) * 54;
          flowSymbol(el, SS.path(el, k), k, cx - 24, cy - 16, 48, 32);
          SS.cp(el, k, cx, cy, FLOW[k] + " (klicken)");
        }
        SS.trim(el, FLOW.length);
        SS.cpTrim(el, FLOW.length);
      } else {
        flowSymbol(el, SS.path(el, 0), cd.sel, cd.x, cd.y, cd.w, cd.h);
        SS.trim(el, 1);
        SS.cp(el, 0, cd.x + cd.w + 8, cd.y + cd.h + 8, FLOW[cd.sel] + ": Größe ändern");
        SS.cpTrim(el, 1);
      }
    }
  },
  calendar: {
    init: function (cd, at, r) { var t = now(); cd.year = t.getFullYear(); cd.month = t.getMonth(); cd.x = r ? r.left : at.x - 140; cd.y = r ? r.top : at.y - 122.5; cd.cw = 40; cd.ch = 35; },
    click: function (cd, i) {
      if (i === 0) cd.year--; if (i === 1) cd.year++;
      if (i === 2) { cd.month--; if (cd.month < 0) { cd.month = 11; cd.year--; } }
      if (i === 3) { cd.month++; if (cd.month > 11) { cd.month = 0; cd.year++; } }
      if (i === 4 || i === 5) { var t = now(); cd.year = t.getFullYear(); cd.month = t.getMonth(); }
    },
    drag: function () {},
    build: function (el, cd) {
      var x = cd.x, y = cd.y, w = cd.cw * 7, h = cd.ch, n = 0, t = 0;
      function cell(cx, cy, cw, ch, label, sz) {
        var p = SS.path(el, n++); SS.style(p, false, 1);
        rect(el, p, cx, cy, cw, ch);
        if (label !== "") { SS.text(el, n++, label, cx + cw / 2 - String(label).length * sz * 0.28, cy + ch / 2 + sz * 0.35, sz, "#000000"); }
      }
      cell(x, y, w, h, String(cd.year), 24);
      cell(x, y + h, w, h, MONTHS[cd.month], 24);
      var wd = ["M", "D", "M", "D", "F", "S", "S"];
      for (var c = 0; c < 7; c++) cell(x + c * cd.cw, y + 2 * h, cd.cw, h, wd[c], 20);
      var first = (new Date(cd.year, cd.month, 1).getDay() + 6) % 7, days = new Date(cd.year, cd.month + 1, 0).getDate();
      for (var dnum = 1; dnum <= days; dnum++) {
        var pos = first + dnum - 1, row = Math.floor(pos / 7), col = pos % 7;
        cell(x + col * cd.cw, y + (3 + row) * h, cd.cw, h, String(dnum), 20);
      }
      SS.trim(el, n);
      SS.cp(el, 0, x + 17.5, y + h / 2, String(cd.year - 1), cd.year);
      SS.cp(el, 1, x + w - 17.5, y + h / 2, String(cd.year + 1));
      SS.cp(el, 2, x + 17.5, y + 1.5 * h, MONTHS[(cd.month + 11) % 12], cd.month);
      SS.cp(el, 3, x + w - 17.5, y + 1.5 * h, MONTHS[(cd.month + 1) % 12]);
      SS.cp(el, 4, x + w / 2, y + h / 2, "Klicken: aktuelles Jahr");
      SS.cp(el, 5, x + w / 2, y + 1.5 * h, "Klicken: aktueller Monat");
      SS.cpTrim(el, 6);
      return t;
    }
  },
  clock: {
    init: function (cd, at, r) { var t = now(); cd.cx = r ? (r.left + r.right) / 2 : at.x; cd.cy = r ? (r.top + r.bottom) / 2 : at.y; cd.r = r ? Math.min(r.right - r.left, r.bottom - r.top) / 2 : 60; cd.hour = t.getHours() % 12; cd.minute = t.getMinutes(); },
    click: function (cd, i) { if (i === 0) { var t = now(); cd.hour = t.getHours() % 12; cd.minute = t.getMinutes(); } },
    drag: function (cd, i, m) {
      var a = (Math.atan2(m.y - cd.cy, m.x - cd.cx) * 180 / Math.PI + 90 + 360) % 360;
      if (i === 1) cd.hour = Math.floor(a / 30) % 12;
      if (i === 2) cd.minute = Math.round(a / 6) % 60;
      if (i === 3) cd.r = Math.max(8, Math.sqrt((m.x - cd.cx) * (m.x - cd.cx) + (m.y - cd.cy) * (m.y - cd.cy)));
    },
    build: function (el, cd) {
      var c = { x: cd.cx, y: cd.cy }, r = cd.r;
      var face = SS.path(el, 2);
      SS.ellipse(el, face, c.x, c.y, r, r);
      var ticks = SS.path(el, 1);
      SS.style(ticks, false, 1);
      for (var i = 0; i < 12; i++) { var a = i * Math.PI / 6, p0 = SS.pol(c, r * (i % 3 ? 0.88 : 0.8), a), p1 = SS.pol(c, r * 0.95, a); SS.contour(el, ticks, [[p0.x, p0.y], [p1.x, p1.y]], false); }
      var hands = SS.path(el, 0);
      SS.style(hands, false, 2);
      var ha = ((cd.hour + cd.minute / 60) * 30 - 90) * Math.PI / 180, ma = (cd.minute * 6 - 90) * Math.PI / 180;
      var h = SS.pol(c, r * 0.5, ha), mm = SS.pol(c, r * 0.8, ma);
      SS.contour(el, hands, [[c.x, c.y], [h.x, h.y]], false);
      SS.contour(el, hands, [[c.x, c.y], [mm.x, mm.y]], false);
      SS.trim(el, 3);
      var hh = cd.hour === 0 ? 12 : cd.hour, ms = (cd.minute < 10 ? "0" : "") + cd.minute;
      SS.cp(el, 0, c.x, c.y, "Klicken: aktuelle Uhrzeit (" + hh + ":" + ms + ")");
      SS.cp(el, 1, h.x, h.y, "Stunde: " + hh, "", true);
      SS.cp(el, 2, mm.x, mm.y, "Minute: " + ms, "", true);
      SS.cp(el, 3, c.x + r, c.y, "Größe");
      SS.cpTrim(el, 4);
    }
  },
  fileinfo: {
    init: function (cd) { cd.fmt = 0; },
    click: function (cd, i) { if (i === 0) cd.fmt = (cd.fmt + 1) % 3; },
    drag: function () {},
    build: function (el, cd) {
      var D = __in.doc || {}, W = __in.docWidth || 0, H = __in.docHeight || 0;
      var title = D.title || "Unbenannt", txt;
      if (cd.fmt === 1) txt = title;
      else if (cd.fmt === 2) txt = title + ": " + W + "x" + H;
      else txt = title + ": " + W + "x" + H + " @ " + Math.round(D.resolution || 72) + " Pixel/Zoll (" + ((D.page || 0) + 1) + " von " + (D.pages || 1) + " Seiten, " + (D.layers || 1) + " Ebenen, " + (D.frames || 1) + " Status)";
      var bar = SS.path(el, 1);
      SS.style(bar, false, 1);
      SS.contour(el, bar, [[0, H - 33], [W, H - 33], [W, H], [0, H]], false);
      SS.text(el, 0, txt, 3, H - 10, 14, "#000000");
      SS.trim(el, 2);
      SS.cp(el, 0, Math.min(W, 433), H - 16.5, "Klicken: Format ändern");
      SS.cpTrim(el, 1);
    }
  },
  timestamp: {
    init: function (cd, at, r) { cd.x = r ? r.left : at.x; cd.y = r ? r.bottom : at.y; cd.stamp = ""; cd.fmt = 0; },
    click: function (cd, i) { if (i === 0) cd.fmt = (cd.fmt + 1) % 2; },
    drag: function (cd, i, m, d, st) { if (i === 1) { cd.x = st.x + m.x - d.x; cd.y = st.y + m.y - d.y; } },
    build: function (el, cd) {
      var t = now(), p2 = function (n) { return (n < 10 ? "0" : "") + n; };
      var s = cd.fmt === 0 ? "Gespeichert: " + p2(t.getDate()) + "." + p2(t.getMonth() + 1) + "." + t.getFullYear() + " " + p2(t.getHours()) + ":" + p2(t.getMinutes()) : t.getFullYear() + "-" + p2(t.getMonth() + 1) + "-" + p2(t.getDate()) + "T" + p2(t.getHours()) + ":" + p2(t.getMinutes());
      cd.stamp = s;
      SS.text(el, 0, s, cd.x, cd.y, 12, "#000000");
      SS.trim(el, 1);
      SS.cp(el, 0, cd.x - 6, cd.y - 4, "Klicken: Format wechseln");
      SS.cp(el, 1, cd.x + s.length * 6.5 + 6, cd.y - 4, "Verschieben");
      SS.cpTrim(el, 2);
    }
  },
  inspire: {
    init: function (cd, at, r) { cd.cx = r ? (r.left + r.right) / 2 : at.x; cd.cy = r ? (r.top + r.bottom) / 2 : at.y; cd.seed = 1; cd.r = r ? Math.min(r.right - r.left, r.bottom - r.top) / 2 : 120; },
    click: function (cd, i) { if (i === 0) cd.seed = (cd.seed * 7919 + 13) % 100003; },
    drag: function (cd, i, m) { if (i === 1) cd.r = Math.max(8, Math.sqrt((m.x - cd.cx) * (m.x - cd.cx) + (m.y - cd.cy) * (m.y - cd.cy))); },
    build: function (el, cd) {
      var s = cd.seed, rnd = function () { s = (s * 16807) % 2147483647; return s / 2147483647; };
      var n = 3 + Math.floor(rnd() * 9), pts = [], c = { x: cd.cx, y: cd.cy };
      for (var i = 0; i < n; i++) {
        var a = i * 2 * Math.PI / n + rnd() * 0.5, rr = cd.r * (0.35 + rnd() * 0.65), p = SS.pol(c, rr, a), h = rr * (rnd() * 0.5);
        pts.push([p.x, p.y, p.x + h * Math.sin(a), p.y - h * Math.cos(a), p.x - h * Math.sin(a), p.y + h * Math.cos(a)]);
      }
      SS.contour(el, SS.path(el, 0), pts, true);
      SS.trim(el, 1);
      SS.cp(el, 0, c.x, c.y, "Klicken: neue Anregung");
      SS.cp(el, 1, c.x + cd.r, c.y, "Größe");
      SS.cpTrim(el, 2);
    }
  }
};

SS.run({
  init: function (cd, r, at) { DEFS[MK === "thinking" ? "talking" : MK].init(cd, at, r); cd.shapeName = MK; },
  click: function (cd, i) { var D = DEFS[MK === "thinking" ? "talking" : MK]; if (D.click) D.click(cd, i); },
  drag: function (cd, i, m, d, st) { DEFS[MK === "thinking" ? "talking" : MK].drag(cd, i, m, d, st); },
  build: function (el, cd) { DEFS[MK === "thinking" ? "talking" : MK].build(el, cd, MK === "thinking"); }
});
