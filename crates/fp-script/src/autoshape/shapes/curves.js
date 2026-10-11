// Mondsichel, Herz, Abgerundetes Polygon, Rotationspfad, Welle, Windrad, Zahnrad (Firepixel,
// gemeinsames Skript; `__curveKind` steht davor).
var CK = typeof __curveKind === "string" ? __curveKind : "crescent";

var DEFS = {
  crescent: {
    init: function (cd, c, s) { cd.r = s / 2 * 0.7; cd.arc = 135; cd.hollow = 0.62; },
    drag: function (cd, i, m, d, st) {
      var c = { x: cd.cx, y: cd.cy };
      if (i === 1) cd.arc = SS.clamp(Math.abs(SS.ang(c, m)) * 360 / Math.PI, 10, 350);
      if (i === 2) cd.hollow = SS.clamp((m.x - c.x) / cd.r, 0.05, 0.95);
      if (i === 3) cd.r = Math.max(2, SS.dist(c, m));
    },
    build: function (el, cd) {
      // Öffnung nach rechts: Spitzen bei ±arc/2, Innenkreis um hollow·r nach rechts versetzt
      var c = { x: cd.cx, y: cd.cy }, r = cd.r, half = cd.arc / 2 * Math.PI / 180, a0 = -half;
      var outer = SS.arc(c.x, c.y, r, r, half, 2 * Math.PI - half), p1 = SS.pol(c, r, half);
      var ic = { x: c.x + cd.hollow * r, y: c.y }, rin = SS.dist(ic, p1), bh = SS.ang(ic, p1);
      if (bh < 0) bh += 2 * Math.PI;
      var inner = SS.arc(ic.x, ic.y, rin, rin, 2 * Math.PI - bh, bh);
      SS.contour(el, SS.path(el, 0), outer.concat(inner.slice(1, inner.length - 1)), true);
      SS.trim(el, 1);
      var tip = SS.pol(c, r, a0);
      SS.cp(el, 0, c.x, c.y, "", "", false, false);
      SS.cp(el, 1, tip.x, tip.y, "Bogen: " + Math.round(cd.arc) + "°", "", true);
      SS.cp(el, 2, ic.x, ic.y, "Hohl: " + Math.round(cd.hollow * 100) + "%", "", true);
      SS.cp(el, 3, c.x - r, c.y, "Größe");
      SS.cpTrim(el, 4);
    }
  },
  heart: {
    init: function (cd, c, s) { cd.w = s / 4; cd.gap = s / 24; cd.apex = s * 0.75; cd.apexArc = s / 5; },
    drag: function (cd, i, m, d, st) {
      var dx = m.x - d.x, dy = m.y - d.y;
      if (i === 0) cd.w = Math.max(2, st.w - dx);
      if (i === 1) cd.gap = SS.clamp(st.gap + dy, -cd.w, cd.apex);
      if (i === 2) cd.apex = Math.max(cd.gap + 2, st.apex + dy);
      if (i === 3) cd.apexArc = SS.clamp(st.apexArc - dy, 0, cd.apex);
    },
    build: function (el, cd) {
      var c = { x: cd.cx, y: cd.cy }, w = cd.w, k = SS.K * w;
      var top = c.y + cd.gap, ap = { x: c.x, y: c.y + cd.apex };
      // rechte Hälfte: Kerbe → Bogen → Spitze; linke gespiegelt
      var n = [
        [c.x, top, c.x - k * 0.6, top - k, c.x + k * 0.6, top - k],
        [c.x + w, c.y - w, c.x + w - k, c.y - w, c.x + w + k, c.y - w],
        [c.x + 2 * w, c.y, c.x + 2 * w, c.y - k, c.x + 2 * w, c.y + k * 1.2],
        [ap.x, ap.y, ap.x + cd.apexArc, ap.y - cd.apexArc, ap.x - cd.apexArc, ap.y - cd.apexArc],
        [c.x - 2 * w, c.y, c.x - 2 * w, c.y + k * 1.2, c.x - 2 * w, c.y - k],
        [c.x - w, c.y - w, c.x - w - k, c.y - w, c.x - w + k, c.y - w]
      ];
      SS.contour(el, SS.path(el, 0), n, true);
      SS.trim(el, 1);
      SS.cp(el, 0, c.x - w, c.y, "Breite: " + Math.round(w), "Breite", true);
      SS.cp(el, 1, c.x, top, "Spalt: " + Math.round(cd.gap), "Spalt", true);
      SS.cp(el, 2, ap.x, ap.y, "Scheitelpunkt: " + Math.round(cd.apex), "Scheitelpunkt", true);
      SS.cp(el, 3, ap.x, ap.y - cd.apexArc, "Scheitelbogen: " + Math.round(cd.apexArc), "Scheitelbogen", true);
      SS.cpTrim(el, 4);
    }
  },
  roundpoly: {
    init: function (cd, c, s) { cd.r = s / 2 * 0.9; cd.sides = 5; cd.curve = 10; },
    drag: function (cd, i, m, d, st) {
      var c = { x: cd.cx, y: cd.cy };
      if (i === 1) cd.curve = SS.clamp(st.curve + (m.y - d.y) * -1, 0, cd.r);
      if (i === 2) cd.sides = SS.clamp(Math.round(st.sides + (m.x - d.x) / 10), 3, 64);
      if (i === 3) cd.r = Math.max(2, SS.dist(c, m));
    },
    build: function (el, cd) {
      var c = { x: cd.cx, y: cd.cy }, n = SS.clamp(Math.round(cd.sides), 3, 64), V = [], out = [];
      for (var i = 0; i < n; i++) V.push(SS.pol(c, cd.r, -Math.PI / 2 + i * 2 * Math.PI / n));
      for (var j = 0; j < n; j++) {
        var p = V[(j + n - 1) % n], v = V[j], q = V[(j + 1) % n];
        var lp = SS.dist(p, v), lq = SS.dist(v, q), t = Math.min(cd.curve, lp / 2, lq / 2);
        var a = { x: v.x + (p.x - v.x) / lp * t, y: v.y + (p.y - v.y) / lp * t }, b = { x: v.x + (q.x - v.x) / lq * t, y: v.y + (q.y - v.y) / lq * t };
        out.push([a.x, a.y, a.x, a.y, a.x + (v.x - a.x) * SS.K, a.y + (v.y - a.y) * SS.K]);
        out.push([b.x, b.y, b.x + (v.x - b.x) * SS.K, b.y + (v.y - b.y) * SS.K, b.x, b.y]);
      }
      SS.contour(el, SS.path(el, 0), out, true);
      SS.trim(el, 1);
      SS.cp(el, 0, c.x, c.y, "Mitte", "Center");
      SS.cp(el, 1, V[0].x, V[0].y + cd.curve, "Kurvenradius", "Curve Radius");
      SS.cp(el, 2, c.x - cd.r - 10, c.y + 15, "Seiten: " + n, "Sides", true);
      SS.cp(el, 3, V[0].x, V[0].y, "Größe");
      SS.cpTrim(el, 4);
    }
  },
  revolve: {
    init: function (cd, c, s) { cd.px = 0; cd.py = -s / 2 * 0.6; cd.qx = -s / 2 * 0.15; cd.qy = -s / 2 * 0.26; cd.div = 6; },
    drag: function (cd, i, m, d, st) {
      if (i === 1) { cd.qx = m.x - cd.cx; cd.qy = m.y - cd.cy; }
      if (i === 2) { cd.px = m.x - cd.cx; cd.py = m.y - cd.cy; }
      if (i === 3) cd.div = SS.clamp(Math.round(st.div + (m.x - d.x) / 10), 2, 64);
    },
    build: function (el, cd) {
      var c = { x: cd.cx, y: cd.cy }, n = cd.div, out = [];
      var rp = Math.sqrt(cd.px * cd.px + cd.py * cd.py), ap = Math.atan2(cd.py, cd.px), rq = Math.sqrt(cd.qx * cd.qx + cd.qy * cd.qy), aq = Math.atan2(cd.qy, cd.qx);
      for (var i = 0; i < n; i++) {
        var o = i * 2 * Math.PI / n, a = SS.pol(c, rp, ap + o), b = SS.pol(c, rq, aq + o);
        out.push([a.x, a.y], [b.x, b.y]);
      }
      SS.contour(el, SS.path(el, 0), out, true);
      SS.trim(el, 1);
      SS.cp(el, 0, c.x, c.y, "Mittelpunkt", "origin");
      SS.cp(el, 1, c.x + cd.qx, c.y + cd.qy, "Radius: " + Math.round(rq), "pt", true);
      SS.cp(el, 2, c.x + cd.px, c.y + cd.py, "Punkt 1: (" + Math.round(cd.px) + "," + Math.round(cd.py) + ")", "pt", true);
      SS.cp(el, 3, c.x + rp + 10, c.y, "Unterteilungen: " + n, "", true);
      SS.cpTrim(el, 4);
    }
  },
  wave: {
    init: function (cd, c, s, r) { cd.x1 = r ? r.left : c.x; cd.y1 = r ? (r.top + r.bottom) / 2 : c.y; cd.len = r ? Math.max(10, r.right - r.left) : 100; cd.angle = 0; cd.h = r ? Math.max(2, (r.bottom - r.top) / 2) : 20; cd.wl = 20; cd.curv = 0; },
    drag: function (cd, i, m, d, st) {
      var a = { x: cd.x1, y: cd.y1 };
      if (i === 0) { cd.x1 = st.x1 + m.x - d.x; cd.y1 = st.y1 + m.y - d.y; }
      if (i === 1) { cd.len = Math.max(4, SS.dist(a, m)); cd.angle = SS.ang(a, m) * 180 / Math.PI; }
      if (i === 2) cd.h = Math.max(0, st.h - (m.y - d.y));
      if (i === 3) cd.wl = SS.clamp(st.wl + (m.x - d.x), 2, cd.len);
      if (i === 4) cd.curv = SS.clamp(st.curv - (m.y - d.y), 0, 100);
    },
    build: function (el, cd) {
      var segs = Math.max(1, Math.round(cd.len / cd.wl)), step = cd.len / segs, ang = cd.angle * Math.PI / 180, c = Math.cos(ang), s = Math.sin(ang);
      var pts = [];
      function P(u, v) { return [cd.x1 + u * c - v * s, cd.y1 + u * s + v * c]; }
      var hk = step / 3 * (1 + cd.curv / 50);
      for (var i = 0; i <= segs; i++) {
        var u = i * step, v = (i % 2 === 0 ? 0 : -cd.h);
        var p = P(u, v), pr = P(u - hk, v), su = P(u + hk, v);
        pts.push([p[0], p[1], i === 0 ? p[0] : pr[0], i === 0 ? p[1] : pr[1], i === segs ? p[0] : su[0], i === segs ? p[1] : su[1]]);
      }
      var path = SS.path(el, 0);
      SS.style(path, false, 1);
      SS.contour(el, path, pts, false);
      SS.trim(el, 1);
      var e = P(cd.len, 0), hh = P(0, -cd.h), w = P(cd.wl, -cd.h), k = P(0, -cd.h - 8 - cd.curv / 5);
      SS.cp(el, 0, cd.x1, cd.y1, "Anker", "", true);
      SS.cp(el, 1, e[0], e[1], "Größe ändern; Drehung: " + Math.round(cd.angle) + "° Länge: " + Math.round(cd.len) + "px", "", true);
      SS.cp(el, 2, hh[0], hh[1], "Höhe: " + Math.round(cd.h) + "px", "", true);
      SS.cp(el, 3, w[0], w[1], "Wellenlänge: " + Math.round(cd.wl) + "px, " + segs + " Segmente", "", true);
      SS.cp(el, 4, k[0], k[1], "Krümmung: " + Math.round(cd.curv) + "px", "", true);
      SS.cpTrim(el, 5);
    }
  },
  pinwheel: {
    init: function (cd, c, s) { cd.r = s / 2 * 0.6; cd.blades = 2; cd.curve = 34.5; cd.rot = 0; cd.inner = 1; cd.style = 0; },
    drag: function (cd, i, m, d, st) {
      var c = { x: cd.cx, y: cd.cy };
      if (i === 1) cd.curve = SS.clamp(st.curve + (m.x - d.x), -170, 170);
      if (i === 2) { cd.r = Math.max(4, SS.dist(c, m)); cd.rot = SS.ang(c, m) * 180 / Math.PI; }
      if (i === 3) cd.inner = SS.clamp(SS.dist(c, m), 0, cd.r - 1);
      if (i === 4) cd.blades = SS.clamp(Math.round(st.blades - (m.x - d.x) / 10), 1, 36);
    },
    click: function (cd, i) { if (i === 4) cd.style = cd.style ? 0 : 1; },
    build: function (el, cd) {
      var c = { x: cd.cx, y: cd.cy }, n = cd.blades, rot = cd.rot * Math.PI / 180, cur = cd.curve * Math.PI / 180;
      for (var b = 0; b < n; b++) {
        var a0 = rot + b * 2 * Math.PI / n, aw = Math.PI / n;
        var p0 = SS.pol(c, cd.inner, a0), tip = SS.pol(c, cd.r, a0 + cur * 0.3), p1 = SS.pol(c, cd.r * 0.95, a0 + aw), mid = SS.pol(c, cd.r * 0.55, a0 + cur * 0.5);
        var path = SS.path(el, b);
        SS.contour(el, path, [[p0.x, p0.y], [tip.x, tip.y, mid.x, mid.y, tip.x, tip.y], [p1.x, p1.y]], true);
        if (cd.style === 1 && b % 2 === 1) path.pathAttributes.fillColor = "#ffffff";
      }
      SS.trim(el, n);
      var e = SS.pol(c, cd.r, rot);
      SS.cp(el, 0, c.x, c.y, "Innenradius", "Innenradius");
      SS.cp(el, 1, c.x + cd.r * 0.4, c.y, "Kurve: " + cd.curve.toFixed(1) + "°", "Kurve und Art", true);
      SS.cp(el, 2, e.x, e.y, "Größe und Drehung: " + Math.round(cd.r) + "px, " + Math.round(cd.rot) + "°", "Größe und Drehung", true);
      SS.cp(el, 3, c.x + cd.inner, c.y, "Innenradius: " + Math.round(cd.inner) + "px", "Innenradius", true);
      SS.cp(el, 4, c.x - cd.r, c.y, "Anzahl und Stil: " + n + (cd.style ? ", zweifarbig" : ", einfarbig"), "Anzahl und Stil", true);
      SS.cpTrim(el, 5);
    }
  },
  cog: {
    init: function (cd, c, s) { cd.r = s / 2 * 0.8; cd.teeth = 12; cd.depth = s / 2 * 0.15; cd.hole = s / 2 * 0.25; },
    drag: function (cd, i, m, d, st) {
      var c = { x: cd.cx, y: cd.cy };
      if (i === 0) cd.r = Math.max(4, SS.dist(c, m));
      if (i === 1) cd.depth = SS.clamp(cd.r - SS.dist(c, m), 1, cd.r - 2);
      if (i === 2) cd.hole = SS.clamp(SS.dist(c, m), 0, cd.r - cd.depth - 1);
      if (i === 3) cd.teeth = SS.clamp(Math.round(st.teeth + (m.x - d.x) / 8), 3, 120);
    },
    build: function (el, cd) {
      var c = { x: cd.cx, y: cd.cy }, n = cd.teeth, ro = cd.r, ri = cd.r - cd.depth, out = [];
      for (var i = 0; i < n; i++) {
        var a = -Math.PI / 2 + i * 2 * Math.PI / n, w = Math.PI / n;
        [[ri, a - w * 0.5], [ro, a - w * 0.3], [ro, a + w * 0.3], [ri, a + w * 0.5]].forEach(function (q) { var p = SS.pol(c, q[0], q[1]); out.push([p.x, p.y]); });
      }
      var path = SS.path(el, 0);
      SS.contour(el, path, out, true);
      if (cd.hole > 0) SS.ellipse(el, path, c.x, c.y, cd.hole, cd.hole, true);
      SS.trim(el, 1);
      var t = SS.pol(c, ri, -Math.PI / 2 + Math.PI / n);
      SS.cp(el, 0, c.x + ro, c.y, "Radius");
      SS.cp(el, 1, t.x, t.y, "Zahntiefe");
      SS.cp(el, 2, c.x + cd.hole, c.y, "Loch");
      SS.cp(el, 3, c.x - ro - 10, c.y, "Zähne: " + n, n, true);
      SS.cpTrim(el, 4);
    }
  }
};

SS.run({
  init: function (cd, r, at) {
    var s = r ? Math.max(8, Math.min(r.right - r.left, r.bottom - r.top)) : 120;
    cd.cx = r ? (r.left + r.right) / 2 : at.x; cd.cy = r ? (r.top + r.bottom) / 2 : at.y;
    DEFS[CK].init(cd, { x: cd.cx, y: cd.cy }, s, r);
    cd.shapeName = CK;
  },
  click: function (cd, i) { if (DEFS[CK].click) DEFS[CK].click(cd, i); },
  drag: function (cd, i, m, d, st) { DEFS[CK].drag(cd, i, m, d, st); },
  build: function (el, cd) { DEFS[CK].build(el, cd); }
});
