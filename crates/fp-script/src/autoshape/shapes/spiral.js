// Spirale (Firepixel): archimedische Spirale von der Mitte nach außen. Parameter: cx, cy, radius,
// turns (Windungen, auch gebrochen), clockwise, closed (Ende mit der Mitte verbunden).
SS.run({
  init: function (cd, r, at) {
    var rad = r ? Math.max(2, Math.min(r.right - r.left, r.bottom - r.top) / 2) : 100;
    cd.cx = r ? (r.left + r.right) / 2 : at.x + rad; cd.cy = r ? (r.top + r.bottom) / 2 : at.y + rad;
    cd.radius = rad;
    if (cd.turns === undefined) { cd.turns = 6; cd.clockwise = true; cd.closed = false; }
    cd.shapeName = "spiral";
  },
  click: function (cd, i) { if (i === 0) cd.closed = !cd.closed; },
  drag: function (cd, i, m, d, start) {
    if (i === 1) cd.radius = Math.max(2, SS.dist({ x: cd.cx, y: cd.cy }, m));
    else if (i === 2) cd.turns = SS.clamp(start.turns - (m.y - d.y) / 10, 0.25, 50);
  },
  build: function (el, cd) {
    var c = { x: cd.cx, y: cd.cy }, R = Math.max(2, cd.radius), T = SS.clamp(SS.num(cd.turns, 6), 0.25, 50), dir = cd.clockwise === false ? -1 : 1;
    var total = T * 2 * Math.PI, steps = Math.max(2, Math.ceil(T * 4)), dt = total / steps, n = [];
    var start = Math.PI; // Anfang links neben der Mitte, wie eine von Hand gezeichnete Spirale
    for (var s = 0; s <= steps; s++) {
      var th = s * dt, r = R * th / total, a = start + dir * th;
      var x = c.x + r * Math.cos(a), y = c.y + r * Math.sin(a);
      // Ableitung nach th: dr/dth = R/total
      var dr = R / total, tx = dr * Math.cos(a) - r * dir * Math.sin(a), ty = dr * Math.sin(a) + r * dir * Math.cos(a), h = dt / 3;
      n.push([x, y, s === 0 ? x : x - tx * h, s === 0 ? y : y - ty * h, s === steps ? x : x + tx * h, s === steps ? y : y + ty * h]);
    }
    var p = SS.path(el, 0);
    SS.style(p, false, 1);
    SS.contour(el, p, n, !!cd.closed);
    SS.trim(el, 1);
    var e = n[n.length - 1];
    SS.cp(el, 0, c.x, c.y, "Klicken: Spirale öffnen/schließen", cd.closed ? "closed" : "open");
    SS.cp(el, 1, e[0], e[1], "Radius");
    SS.cp(el, 2, c.x - R, c.y - R * 0.54, "Windungen: " + Math.round(T * 4) / 4, "", true);
    SS.cpTrim(el, 3);
  }
});
