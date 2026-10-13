// Stern (Firepixel). Parameter: cx, cy (Mitte), nodes (Zacken), radius0 (außen), radius1 (innen),
// round0/round1 (Rundung außen/innen: Abstand der Ankerpunkte; ≠ Radius ergibt gerundete Spitzen).
SS.run({
  init: function (cd, r, at) {
    var rad = r ? Math.min(r.right - r.left, r.bottom - r.top) / 2 : 100;
    rad = Math.max(1, rad);
    cd.cx = r ? (r.left + r.right) / 2 : at.x + rad;
    cd.cy = r ? (r.top + r.bottom) / 2 : at.y + rad;
    if (cd.nodes === undefined) cd.nodes = 5;
    var ratio = (cd.radius0 && cd.radius1) ? cd.radius1 / cd.radius0 : 0.5;
    cd.radius0 = rad; cd.radius1 = rad * ratio; cd.round0 = cd.radius0; cd.round1 = cd.radius1;
    cd.constants = { maxNodes: 50, minNodes: 3 };
    cd.shapeName = "star";
  },
  drag: function (cd, i, m, d, start) {
    var c = { x: cd.cx, y: cd.cy }, dist = SS.dist(c, m);
    if (i === 0) { cd.radius0 = Math.max(1, dist); cd.round0 = cd.radius0 + (start.round0 - start.radius0); }
    else if (i === 1) { cd.radius1 = Math.max(0, dist); cd.round1 = cd.radius1 + (start.round1 - start.radius1); }
    else if (i === 2) cd.round0 = dist;
    else if (i === 3) cd.round1 = dist;
    // Punkte-Griff wie im Original nur senkrecht: nach oben mehr, nach unten weniger Zacken (Radius 60:
    // +40 → 3, −80 → 34; waagrecht keine Änderung). Danach steht der Griff auf der Skala 3…50.
    else if (i === 4) cd.nodes = SS.clamp(Math.round(start.nodes - (m.y - d.y) * 21.75 / Math.max(1, start.radius0)), 3, 50);
    cd.pointsScale = true;
  },
  build: function (el, cd) {
    var n = SS.clamp(Math.round(SS.num(cd.nodes, 5)), 3, 50), c = { x: cd.cx, y: cd.cy };
    var r0 = SS.num(cd.radius0, 100), r1 = SS.num(cd.radius1, 50), q0 = SS.num(cd.round0, r0), q1 = SS.num(cd.round1, r1);
    var base = [], i;
    for (i = 0; i < 2 * n; i++) base.push(SS.pol(c, i % 2 ? r1 : r0, -Math.PI / 2 + i * Math.PI / n));
    var nodes = [];
    for (i = 0; i < 2 * n; i++) {
      var outer = i % 2 === 0, a = -Math.PI / 2 + i * Math.PI / n;
      var rad = outer ? r0 : r1, q = outer ? q0 : q1;
      var p = SS.pol(c, q, a);
      if (Math.abs(q - rad) < 1e-6) { nodes.push([p.x, p.y]); continue; }
      // halber Innenwinkel der Spitze im ungerundeten Stern
      var prev = base[(i + 2 * n - 1) % (2 * n)], next = base[(i + 1) % (2 * n)], v = base[i];
      var ax = prev.x - v.x, ay = prev.y - v.y, bx = next.x - v.x, by = next.y - v.y;
      var cosA = (ax * bx + ay * by) / (Math.sqrt(ax * ax + ay * ay) * Math.sqrt(bx * bx + by * by) || 1);
      var half = Math.acos(SS.clamp(cosA, -1, 1)) / 2;
      var t = (q - rad) * Math.tan(half) * (outer ? 1 : -1);
      var tx = -Math.sin(a), ty = Math.cos(a);
      nodes.push([p.x, p.y, p.x + t * tx, p.y + t * ty, p.x - t * tx, p.y - t * ty]);
    }
    var path = SS.path(el, 0);
    SS.contour(el, path, nodes, true);
    SS.trim(el, 1);
    var o0 = SS.pol(c, r0, -Math.PI / 2), i1 = SS.pol(c, r1, -Math.PI / 2 + Math.PI / n);
    var o1 = SS.pol(c, q0, -Math.PI / 2 + 2 * Math.PI / n), i2 = SS.pol(c, q1, -Math.PI / 2 + 3 * Math.PI / n);
    SS.cp(el, 0, o0.x, o0.y, "Radius 1");
    SS.cp(el, 1, i1.x, i1.y, "Radius 2");
    SS.cp(el, 2, o1.x, o1.y, "Rundung 1", "", true);
    SS.cp(el, 3, i2.x, i2.y, "Rundung 2", "", true);
    // frisch eingefügt bei 0,88 · Radius unter der Mitte, nach einem Griffzug auf der Skala von
    // 3 (Mitte + Radius) bis 50 (Mitte − Radius), wie die Griffdaten des Originals
    var py = cd.pointsScale ? c.y + r0 - (n - 3) * 2 * r0 / 47 : c.y + r0 * 0.88;
    SS.cp(el, 4, c.x - r0, py, "Punkte: " + n, n, true);
    SS.cp(el, 5, c.x, c.y, "", "", false, false);
    SS.cpTrim(el, 6);
  }
});
