// 3D-Kasten (Firepixel): perspektivisch gezeichneter Würfel, drehbar. Parameter: cx, cy, size,
// rotX, rotY (Grad), focal (Brennweite; kleiner = stärkere Perspektive).
SS.run({
  init: function (cd, r, at) {
    cd.size = r ? Math.max(4, Math.min(r.right - r.left, r.bottom - r.top) * 0.83) : 100;
    cd.cx = r ? (r.left + r.right) / 2 : at.x; cd.cy = r ? (r.top + r.bottom) / 2 : at.y;
    if (cd.rotX === undefined) { cd.rotX = 0; cd.rotY = 0; cd.focal = 300; }
    cd.shapeName = "box3d";
  },
  drag: function (cd, i, m, d, start, S) {
    var dx = m.x - d.x, dy = m.y - d.y;
    if (i === 0) {
      if (S.shiftKeyDown) cd.focal = SS.clamp(start.focal + dy * 3, 60, 5000);
      else { cd.rotY = start.rotY + dx * 0.5; cd.rotX = start.rotX - dy * 0.5; }
    } else if (i >= 1 && i <= 8) {
      // Ecke ziehen: Größe ändern
      var c = { x: cd.cx, y: cd.cy }, f = SS.dist(c, m) / Math.max(1, SS.dist(c, d));
      cd.size = SS.clamp(start.size * f, 4, 100000);
    }
  },
  build: function (el, cd) {
    var h = cd.size / 2, ax = cd.rotX * Math.PI / 180, ay = cd.rotY * Math.PI / 180, f = Math.max(10, cd.focal);
    var P = [[-h, -h, h], [h, -h, h], [h, h, h], [-h, h, h], [-h, -h, -h], [h, -h, -h], [h, h, -h], [-h, h, -h]];
    var V = [], Z = [];
    for (var i = 0; i < 8; i++) {
      var x = P[i][0], y = P[i][1], z = P[i][2];
      var y1 = y * Math.cos(ax) - z * Math.sin(ax), z1 = y * Math.sin(ax) + z * Math.cos(ax);
      var x2 = x * Math.cos(ay) + z1 * Math.sin(ay), z2 = -x * Math.sin(ay) + z1 * Math.cos(ay);
      var s = f / (f + z2 + h * 2);
      V.push({ x: cd.cx + x2 * s * 1.2, y: cd.cy + y1 * s * 1.2 }); Z.push(z2);
    }
    var faces = [[4, 5, 6, 7], [1, 5, 6, 2], [0, 4, 7, 3], [3, 2, 6, 7], [0, 1, 5, 4], [0, 1, 2, 3]];
    faces.sort(function (a, b) { var za = 0, zb = 0; for (var k = 0; k < 4; k++) { za += Z[a[k]]; zb += Z[b[k]]; } return za - zb; });
    // hintere Flächen zuerst (größeres z = weiter hinten wird zuerst gezeichnet: z > 0 hinten)
    faces.reverse();
    for (var fi = 0; fi < faces.length; fi++) {
      var p = SS.path(el, faces.length - 1 - fi), q = faces[fi];
      SS.contour(el, p, q.map(function (k) { return [V[k].x, V[k].y]; }), true);
    }
    SS.trim(el, 6);
    SS.cp(el, 0, cd.cx, cd.cy, "Drehen (Umschalt: Perspektive)", "", true);
    for (var c = 0; c < 8; c++) SS.cp(el, 1 + c, V[c].x, V[c].y, "Größe ändern", "", true);
    SS.cpTrim(el, 9);
  }
});
