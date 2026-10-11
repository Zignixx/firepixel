// L-Form (Firepixel): oberer Balken und linker Balken. Parameter: x, y, width, height, thickness,
// roundness (Rundung der äußeren Ecke).
SS.run({
  init: function (cd, r, at) {
    cd.x = r ? r.left : at.x; cd.y = r ? r.top : at.y;
    cd.width = r ? Math.max(2, r.right - r.left) : 200; cd.height = r ? Math.max(2, r.bottom - r.top) : 200;
    cd.thickness = Math.min(cd.width, cd.height) / 2; cd.roundness = 0;
    cd.shapeName = "lshape";
  },
  drag: function (cd, i, m, d, start) {
    var dx = m.x - d.x, dy = m.y - d.y;
    if (i === 0) cd.roundness = SS.clamp(start.roundness + dx, 0, start.thickness);
    else if (i === 1) cd.thickness = SS.clamp(start.thickness + dx, 1, Math.min(start.width, start.height));
    else if (i === 2) cd.width = Math.max(cd.thickness, start.width + dx);
    else if (i === 3) cd.height = Math.max(cd.thickness, start.height + dy);
  },
  build: function (el, cd) {
    var x = cd.x, y = cd.y, w = cd.width, h = cd.height, t = SS.clamp(cd.thickness, 1, Math.min(w, h)), r = SS.clamp(SS.num(cd.roundness, 0), 0, t), k = SS.K;
    var n = [];
    if (r > 0) n.push([x, y + r, x, y + r, x, y + r - r * k], [x + r, y, x + r - r * k, y, x + r, y]);
    else n.push([x, y]);
    n.push([x + w, y], [x + w, y + t]);
    n.push([x + t, y + t]);
    n.push([x + t, y + h], [x, y + h]);
    var p = SS.path(el, 0);
    SS.contour(el, p, n, true);
    SS.trim(el, 1);
    SS.cp(el, 0, x + r, y, "Rundung");
    SS.cp(el, 1, x + t, y + h, "Dicke");
    SS.cp(el, 2, x + w, y + t / 2, "Breite");
    SS.cp(el, 3, x + t / 2, y + h, "Höhe");
    SS.cpTrim(el, 4);
  }
});
