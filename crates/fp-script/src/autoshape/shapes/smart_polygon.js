// Intelligentes Polygon (Firepixel): regelmäßiges Vieleck. Parameter: cx, cy, radius, sides (3–360),
// rotation (Grad), inner (0 = voll, sonst Radius eines ausgesparten inneren Vielecks).
SS.run({
  init: function (cd, r, at) {
    var q = r ? SS.radial(r, at) : null;
    var rad = Math.max(1, q ? q.rad : 100);
    cd.cx = q ? q.cx : at.x + rad; cd.cy = q ? q.cy : at.y + rad;
    cd.radius = rad;
    if (cd.sides === undefined) { cd.sides = 5; cd.rotation = -90; cd.inner = 0; }
    cd.shapeName = "polygon";
  },
  drag: function (cd, i, m, d, start, S) {
    var c = { x: cd.cx, y: cd.cy };
    if (i === 0) {
      cd.radius = Math.max(1, SS.dist(c, m));
      if (!S.shiftKeyDown) cd.rotation = SS.ang(c, m) * 180 / Math.PI;
    } else if (i === 1) cd.sides = SS.clamp(Math.round(start.sides + (m.x - d.x) / 10), 3, 360);
    else if (i === 2) cd.inner = SS.clamp(SS.dist(c, m), 0, cd.radius - 1);
  },
  build: function (el, cd) {
    var c = { x: cd.cx, y: cd.cy }, n = SS.clamp(Math.round(SS.num(cd.sides, 5)), 3, 360), rot = SS.num(cd.rotation, -90) * Math.PI / 180;
    var outer = [], inner = [], ri = SS.num(cd.inner, 0);
    for (var i = 0; i < n; i++) {
      var a = rot + i * 2 * Math.PI / n, p = SS.pol(c, cd.radius, a);
      outer.push([p.x, p.y]);
      if (ri > 0) { var q = SS.pol(c, ri, a); inner.push([q.x, q.y]); }
    }
    var path = SS.path(el, 0);
    SS.contour(el, path, outer, true);
    if (inner.length) SS.contour(el, path, inner, true);
    SS.trim(el, 1);
    var v = SS.pol(c, cd.radius, rot), s = SS.pol(c, ri, rot + Math.PI / n);
    SS.cp(el, 0, v.x, v.y, "Größe und Drehung (Umschalt: nur Größe)");
    SS.cp(el, 1, c.x - cd.radius - 10, c.y, "Seiten: " + n + " (waagerecht ziehen)", n, true);
    SS.cp(el, 2, ri > 0 ? s.x : c.x, ri > 0 ? s.y : c.y, "Innenausschnitt");
    SS.cpTrim(el, 3);
  }
});
