// Verbindungslinie (Firepixel): rechtwinklige Verbindung zweier Punkte. Parameter: x1, y1, x2, y2,
// mode (0 waagerecht–senkrecht–waagerecht, 1 senkrecht–waagerecht–senkrecht), mid (Lage des
// Mittelstücks), corner1/corner2 (Rundung der beiden Knicke).
SS.run({
  init: function (cd, r, at, S) {
    if (r) {
      var a = S.elem.globalToSmartShapeCoords(S.mouseDownPos), b = S.elem.globalToSmartShapeCoords(S.currentMousePos);
      cd.x1 = a.x; cd.y1 = a.y; cd.x2 = b.x; cd.y2 = b.y;
    } else { cd.x1 = at.x; cd.y1 = at.y; cd.x2 = at.x + 200; cd.y2 = at.y + 200; }
    if (cd.mode === undefined) cd.mode = 0;
    cd.mid = cd.mode === 0 ? (cd.x1 + cd.x2) / 2 : (cd.y1 + cd.y2) / 2;
    cd.corner1 = 0; cd.corner2 = 0;
    cd.shapeName = "connector";
  },
  click: function (cd, i) {
    if (i !== 0) return;
    cd.mode = cd.mode === 0 ? 1 : 0;
    cd.mid = cd.mode === 0 ? (cd.x1 + cd.x2) / 2 : (cd.y1 + cd.y2) / 2;
  },
  drag: function (cd, i, m, d, start, S) {
    var dx = m.x - d.x, dy = m.y - d.y;
    if (i === 1) { cd.x1 = start.x1 + dx; cd.y1 = start.y1 + dy; }
    else if (i === 2) { cd.x2 = start.x2 + dx; cd.y2 = start.y2 + dy; }
    else if (i === 3 || i === 4) {
      if (S.altOptKeyDown) {
        var k = i === 3 ? "corner1" : "corner2";
        cd[k] = Math.max(0, start[k] + (cd.mode === 0 ? Math.abs(dy) : Math.abs(dx)));
      } else cd.mid = start.mid + (cd.mode === 0 ? dx : dy);
    }
    if (i === 1 || i === 2) {
      // Mittelstück relativ zur alten Lage mitführen
      var lo0 = cd.mode === 0 ? Math.min(start.x1, start.x2) : Math.min(start.y1, start.y2);
      var hi0 = cd.mode === 0 ? Math.max(start.x1, start.x2) : Math.max(start.y1, start.y2);
      var f = hi0 > lo0 ? (start.mid - lo0) / (hi0 - lo0) : 0.5;
      var lo = cd.mode === 0 ? Math.min(cd.x1, cd.x2) : Math.min(cd.y1, cd.y2), hi = cd.mode === 0 ? Math.max(cd.x1, cd.x2) : Math.max(cd.y1, cd.y2);
      cd.mid = lo + f * (hi - lo);
    }
  },
  build: function (el, cd) {
    var a = { x: cd.x1, y: cd.y1 }, b = { x: cd.x2, y: cd.y2 }, c1, c2;
    if (cd.mode === 0) { c1 = { x: cd.mid, y: a.y }; c2 = { x: cd.mid, y: b.y }; }
    else { c1 = { x: a.x, y: cd.mid }; c2 = { x: b.x, y: cd.mid }; }
    function knick(prev, c, next, r) {
      var l1 = SS.dist(prev, c), l2 = SS.dist(c, next);
      r = Math.min(r, l1 / 2, l2 / 2);
      if (r <= 0.01) return [[c.x, c.y], [c.x, c.y]];
      var u1 = { x: (prev.x - c.x) / l1, y: (prev.y - c.y) / l1 }, u2 = { x: (next.x - c.x) / l2, y: (next.y - c.y) / l2 };
      var p1 = { x: c.x + u1.x * r, y: c.y + u1.y * r }, p2 = { x: c.x + u2.x * r, y: c.y + u2.y * r }, k = SS.K;
      return [[p1.x, p1.y, p1.x, p1.y, p1.x - u1.x * r * k, p1.y - u1.y * r * k], [p2.x, p2.y, p2.x - u2.x * r * k, p2.y - u2.y * r * k, p2.x, p2.y]];
    }
    var n = [[a.x, a.y]].concat(knick(a, c1, c2, SS.num(cd.corner1, 0)), knick(c1, c2, b, SS.num(cd.corner2, 0)), [[b.x, b.y]]);
    var p = SS.path(el, 0);
    // no fill, the tool stroke; without a tool stroke its own black `bn_1-Pixel` 1 px (RE spec 59 section 1.3)
    SS.style(p, false);
    if (!p.pathAttributes.brush) SS.brush(p, "bc_Pencil", "bn_1-Pixel", 1, "#000000");
    SS.contour(el, p, n, false);
    SS.trim(el, 1);
    SS.cp(el, 0, (a.x + b.x) / 2, (a.y + b.y) / 2, "Richtung drehen", cd.mode);
    SS.cp(el, 1, a.x, a.y, "Anfang");
    SS.cp(el, 2, b.x, b.y, "Ende");
    SS.cp(el, 3, c1.x, c1.y, "Mittelstück verschieben (Alt: Knick runden)");
    SS.cp(el, 4, c2.x, c2.y, "Mittelstück verschieben (Alt: Knick runden)");
    SS.cpTrim(el, 5);
  }
});
