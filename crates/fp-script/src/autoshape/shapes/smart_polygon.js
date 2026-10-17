// Intelligentes Polygon (Firepixel, re 36 §3): regelmäßiges Vieleck, Vorgabe 7 Ecken, erste Ecke oben,
// im Uhrzeigersinn, nur Eckpunkte. Parameter: cx, cy, radius (Umkreis), sides (3–25), segments
// (1–sides; weniger = offen zum Mittelpunkt), rotation (Grad, 0 = erste Ecke oben), inner
// (Innenradius, 0 = kein Loch), scaled (Griffe nach Größenzug auf der Skala).
function jsRound(v) { return Math.floor(v + 0.5); }
SS.run({
  lockAspect: true,
  init: function (cd, r, at) {
    var q = r ? SS.radial(r, at) : null;
    var rad = Math.max(1, q ? q.rad : 100);
    cd.cx = q ? q.cx : at.x + rad; cd.cy = q ? q.cy : at.y + rad;
    cd.radius = rad;
    if (cd.sides === undefined) { cd.sides = 7; cd.segments = 7; cd.rotation = 0; cd.inner = 0; }
    delete cd.handle3y; delete cd.handle4y; delete cd.scaled;
    cd.shapeName = "polygon";
  },
  click: function (cd, i, S) {
    if (i !== 0) return;
    if (S.altOptKeyDown) cd.rotation = 0; else cd.inner = 0;
  },
  drag: function (cd, i, m, d, start, S) {
    var c = { x: cd.cx, y: cd.cy }, R = Math.max(1, SS.num(start.radius, 100));
    var dy = m.y - d.y;
    if (i === 1) {
      cd.inner = SS.clamp(SS.dist(c, m), 0, R);
      if (cd.inner < 3) cd.inner = 0;
    } else if (i === 2) {
      // Größe und Drehung; Alt: nur Drehung. Innenradius wächst mit.
      var nr = S.altOptKeyDown ? R : Math.max(1, SS.dist(c, m));
      cd.radius = nr;
      cd.inner = SS.num(start.inner, 0) * nr / R;
      cd.rotation = SS.ang(c, m) * 180 / Math.PI + 90;
      cd.scaled = true;
    } else if (i === 3) {
      var y3 = SS.clamp(SS.num(start.handle3y, c.y + 0.6 * R) + dy, c.y - R, c.y + R);
      cd.sides = SS.clamp(jsRound(22 * (c.y + R - y3) / (2 * R)) + 3, 3, 25);
      cd.segments = cd.sides;
      cd.handle3y = y3;
      cd.split = !!S.altOptKeyDown;
    } else if (i === 4) {
      var N = SS.clamp(Math.round(SS.num(start.sides, 7)), 3, 25);
      var y4 = SS.clamp(SS.num(start.handle4y, c.y - R) + dy, c.y - R, c.y + R);
      cd.segments = SS.clamp(jsRound((c.y + R - y4) / (2 * R / (N - 1))) + 1, 1, N);
      cd.handle4y = y4;
    }
  },
  build: function (el, cd) {
    var c = { x: cd.cx, y: cd.cy }, R = Math.max(1, SS.num(cd.radius, 100));
    var n = SS.clamp(Math.round(SS.num(cd.sides, 7)), 3, 25), seg = SS.clamp(Math.round(SS.num(cd.segments, n)), 1, n);
    var rot = (SS.num(cd.rotation, 0) - 90) * Math.PI / 180, ri = SS.clamp(SS.num(cd.inner, 0), 0, R);
    var outer = [], inner = [], i;
    for (i = 0; i < n; i++) {
      var a = rot + i * 2 * Math.PI / n, p = SS.pol(c, R, a);
      outer.push([p.x, p.y]);
      if (ri > 0) { var q = SS.pol(c, ri, a); inner.push([q.x, q.y]); }
    }
    var path = SS.path(el, 0);
    if (seg >= n) {
      SS.contour(el, path, outer, true);
      if (inner.length) SS.contour(el, path, inner.reverse(), true);
    } else {
      // offenes Polygon: S Segmente, über den Mittelpunkt (bzw. das Innenpolygon) geschlossen
      var o = [];
      for (i = 0; i <= seg; i++) o.push(outer[i % n]);
      if (inner.length) for (i = seg; i >= 0; i--) o.push(inner[i % n]);
      else o.push([c.x, c.y]);
      SS.contour(el, path, o, true);
    }
    SS.trim(el, 1);
    // Griffe: 0 Mitte, 1 Innenradius, 2 Größe/Drehung, 3 Ecken, 4 Segmente (re 36 §3.3)
    var h3 = cd.scaled ? c.y + R - jsRound(2 * R * (n - 3) / 22) : c.y + 0.6 * R;
    var h4 = cd.scaled ? c.y + R - jsRound(2 * R * (seg - 1) / (n - 1)) : c.y - R;
    if (cd.handle3y !== undefined && !cd.scaled) h3 = cd.handle3y;
    if (cd.handle4y !== undefined && !cd.scaled) h4 = cd.handle4y;
    h3 = SS.clamp(h3, c.y - R, c.y + R); h4 = SS.clamp(h4, c.y - R, c.y + R);
    cd.handle3y = h3; cd.handle4y = h4;
    var v = SS.pol(c, R, rot), s = SS.pol(c, ri, rot);
    SS.cp(el, 0, c.x, c.y, "Innenpolygon entfernen (Alt: Drehung zurücksetzen)");
    SS.cp(el, 1, ri > 0 ? s.x : c.x, ri > 0 ? s.y : c.y, "Innenradius");
    SS.cp(el, 2, v.x, v.y, "Größe und Drehung (Alt: nur Drehung)");
    SS.cp(el, 3, c.x - R, h3, "Seiten: " + n, n, true);
    SS.cp(el, 4, c.x + R, h4, "Segmente: " + seg, seg, true);
    SS.cpTrim(el, 5);
  }
});
