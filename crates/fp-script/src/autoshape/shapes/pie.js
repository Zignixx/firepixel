// Torte und Donut (Firepixel, gemeinsames Skript; Vorgaben `__pieInner`/`__pieAngles` stehen davor).
// Parameter: cx, cy, radius0 (außen), radius1 (innen, 0 = Torte), angles (Segmentgrenzen in Grad,
// 0 = rechts, im Uhrzeigersinn; letzter Eintrag schließt den Kreis). Je Segment ein Pfad.
var PIE_INNER = typeof __pieInner === "number" ? __pieInner : 0;
var PIE_ANGLES = typeof __pieAngles === "object" ? __pieAngles : [-90, 0, 270];
var PIE_CENTERED = typeof __pieCentered === "boolean" ? __pieCentered : false;
SS.run({
  init: function (cd, r, at) {
    var q = r ? SS.radial(r, at) : null;
    var rad = Math.max(1, q ? q.rad : 100);
    var off = PIE_CENTERED ? 0 : rad;
    cd.cx = q ? q.cx : at.x + off; cd.cy = q ? q.cy : at.y + off;
    cd.radius0 = rad; cd.radius1 = rad * PIE_INNER;
    cd.angles = PIE_ANGLES.slice(0);
    cd.shapeName = PIE_CENTERED ? "piechart" : PIE_INNER > 0 ? "doughnut" : "pie";
  },
  click: function (cd, i) {
    if (i === 0) { cd.radius1 = cd.radius0 * PIE_INNER; cd.angles = PIE_ANGLES.slice(0); }
  },
  drag: function (cd, i, m, d, start, S) {
    var c = { x: cd.cx, y: cd.cy };
    if (i === 1) { cd.radius1 = SS.clamp(SS.dist(c, m), 0, cd.radius0 - 1); return; }
    if (i < 2) return;
    var k = i - 2, ang = SS.ang(c, m) * 180 / Math.PI;
    var a = start.angles.slice(0);
    if (S.shiftKeyDown) ang = Math.round(ang / 15) * 15;
    if (S.altOptKeyDown || PIE_CENTERED || a.length < 2) { a[k] = ang; }
    else {
      // ohne Alt: alle Grenzen gemeinsam drehen
      var delta = ang - a[k];
      for (var j = 0; j < a.length; j++) a[j] += delta;
    }
    cd.angles = a;
  },
  build: function (el, cd) {
    var c = { x: cd.cx, y: cd.cy }, r0 = Math.max(1, cd.radius0), r1 = SS.clamp(SS.num(cd.radius1, 0), 0, r0 - 0.01);
    var a = (cd.angles || [-90, 270]).map(function (v) { return Number(v); });
    var segs = [];
    var full = true;
    for (var i = 1; i < a.length; i++) if (Math.abs(((a[i] - a[0]) % 360 + 360) % 360) > 1e-6) full = false;
    if (a.length < 2 || full) segs.push(null);
    else {
      for (var j = 0; j + 1 < a.length; j++) {
        var s0 = a[j], s1 = a[j + 1];
        while (s1 <= s0) s1 += 360;
        while (s1 - s0 > 360) s1 -= 360;
        segs.push([s0 * Math.PI / 180, s1 * Math.PI / 180]);
      }
    }
    for (var s = 0; s < segs.length; s++) {
      var p = SS.path(el, s), g = segs[s];
      if (!g) {
        SS.ellipse(el, p, c.x, c.y, r0, r0, false);
        if (r1 > 0) SS.ellipse(el, p, c.x, c.y, r1, r1, false);
        continue;
      }
      var outer = SS.arc(c.x, c.y, r0, r0, g[0], g[1]);
      var n;
      if (r1 > 0) {
        var inner = SS.arc(c.x, c.y, r1, r1, g[1], g[0]);
        n = outer.concat(inner);
      } else n = [[c.x, c.y]].concat(outer);
      SS.contour(el, p, n, true);
    }
    SS.trim(el, segs.length);
    SS.cp(el, 0, c.x, c.y, "Klicken: Einstellungen zurücksetzen");
    SS.cp(el, 1, c.x + r1, c.y, "Innerer Radius");
    for (var q = 0; q < a.length; q++) {
      var pt = SS.pol(c, r0, a[q] * Math.PI / 180);
      var pct = Math.round(((a[q] - a[0]) % 360 + 360) % 360 / 3.6 * 10) / 10;
      if (q === a.length - 1 && q > 0 && pct === 0) pct = 100;
      SS.cp(el, 2 + q, pt.x, pt.y, PIE_CENTERED ? pct + "%" : "Segment drehen (Alt: einzeln verschieben)", "", PIE_CENTERED);
    }
    SS.cpTrim(el, 2 + a.length);
  }
});
