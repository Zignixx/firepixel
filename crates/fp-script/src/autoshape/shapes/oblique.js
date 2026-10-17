// Würfel, Zylinder, Röhre (Firepixel, gemeinsames Skript; `__obliqueKind` steht davor: 0 Würfel,
// 1 Zylinder, 2 Röhre). Schrägbild: Vorderfläche, Rückfläche um (dx, dy) versetzt, Seitenflächen.
// Parameter: x, y (Vorderfläche links oben), size, dx, dy (Versatz), inner (Innenradius-Anteil der Röhre).
var OB_KIND = typeof __obliqueKind === "number" ? __obliqueKind : 0;
SS.run({
  init: function (cd, r, at) {
    var s = r ? Math.max(4, Math.min(r.right - r.left, r.bottom - r.top) * 0.8) : 160;
    cd.size = s; cd.dx = -s / 4; cd.dy = -s / 4;
    cd.x = r ? r.right - s : at.x - 60; cd.y = r ? r.bottom - s : at.y - 60;
    if (cd.inner === undefined) cd.inner = OB_KIND === 2 ? 0.6 : 0;
    cd.shapeName = ["cube", "cylinder", "tube"][OB_KIND];
  },
  click: function (cd, i) { if (i === 2) { cd.dx = -cd.size / 4; cd.dy = -cd.size / 4; } },
  drag: function (cd, i, m, d, start) {
    var mx = m.x - d.x, my = m.y - d.y;
    if (i === 0) cd.size = Math.max(4, start.size + Math.max(mx, my));
    else if (i === 1) { cd.dx = start.dx + mx; cd.dy = start.dy + my; }
    else if (i === 3) cd.inner = SS.clamp(start.inner + mx / start.size, 0.05, 0.95);
  },
  build: function (el, cd) {
    var x = cd.x, y = cd.y, s = cd.size, dx = cd.dx, dy = cd.dy, n = 0;
    function quad(p, a) { SS.contour(el, p, a, true); }
    if (OB_KIND === 0) {
      var F = [[x, y], [x + s, y], [x + s, y + s], [x, y + s]], B = F.map(function (q) { return [q[0] + dx, q[1] + dy]; });
      // hinten, Seiten, vorne (oben im Stapel zuerst)
      quad(SS.path(el, n++), F);
      var sides = [[1, 2], [2, 3], [3, 0], [0, 1]];
      for (var k = 0; k < 4; k++) { var a = sides[k][0], b = sides[k][1]; quad(SS.path(el, n++), [F[a], F[b], B[b], B[a]]); }
      quad(SS.path(el, n++), B);
    } else {
      var r = s / 2, cx = x + r, cy = y + r, ri = r * SS.clamp(cd.inner, 0, 0.95);
      var front = SS.path(el, n++);
      SS.ellipse(el, front, cx, cy, r, r);
      if (OB_KIND === 2) SS.ellipse(el, front, cx, cy, ri, ri, true);
      // Mantel: Umhüllende zweier Kreise
      var ang = Math.atan2(dy, dx) + Math.PI / 2, t1 = SS.pol({ x: cx, y: cy }, r, ang), t2 = SS.pol({ x: cx, y: cy }, r, ang + Math.PI);
      var body = SS.path(el, n++);
      var arcF = SS.arc(cx, cy, r, r, ang, ang + Math.PI), arcB = SS.arc(cx + dx, cy + dy, r, r, ang + Math.PI, ang + 2 * Math.PI);
      SS.contour(el, body, arcF.concat(arcB), true);
      if (OB_KIND === 2) {
        var inner = SS.path(el, n++);
        SS.ellipse(el, inner, cx + dx, cy + dy, ri, ri);
      }
      var back = SS.path(el, n++);
      SS.ellipse(el, back, cx + dx, cy + dy, r, r);
      t1 = t2;
    }
    SS.trim(el, n);
    SS.cp(el, 0, x + s, y + s, "Größe", OB_KIND, true);
    SS.cp(el, 1, x + s / 2 + dx, y + s / 2 + dy, "Versatz", "", true);
    SS.cp(el, 2, x + dx, y + dy, "Versatz zurücksetzen", "", true);
    if (OB_KIND === 2) { SS.cp(el, 3, x + s / 2 + s / 2 * cd.inner, y + s / 2, "Innerer Radius", "", true); SS.cpTrim(el, 4); }
    else SS.cpTrim(el, 3);
  }
});
