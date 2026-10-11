// Pfeillinie (Firepixel). Parameter: x1, y1, x2, y2 (Endpunkte), head1/head2 (Spitze am Anfang/Ende:
// 0 ohne, 1 offene Spitze, 2 gefüllte Spitze, 3 Querstrich, 4 Kästchen), size (Spitzengröße).
var HEADS = ["Ohne", "Spitze 1", "Spitze 2", "Querstrich", "Kästchen"];
SS.run({
  init: function (cd, r, at, S) {
    if (r) {
      var a = S.elem.globalToSmartShapeCoords(S.mouseDownPos), b = S.elem.globalToSmartShapeCoords(S.currentMousePos);
      cd.x1 = a.x; cd.y1 = a.y; cd.x2 = b.x; cd.y2 = b.y;
      if (S.shiftKeyDown) {
        var ang = Math.round(Math.atan2(cd.y2 - cd.y1, cd.x2 - cd.x1) / (Math.PI / 4)) * Math.PI / 4, len = SS.dist({ x: cd.x1, y: cd.y1 }, b);
        cd.x2 = cd.x1 + len * Math.cos(ang); cd.y2 = cd.y1 + len * Math.sin(ang);
      }
    } else { cd.x1 = at.x; cd.y1 = at.y; cd.x2 = at.x + 100; cd.y2 = at.y; }
    if (cd.head1 === undefined) { cd.head1 = 0; cd.head2 = 1; cd.size = 10; }
    cd.shapeName = "arrowtool";
  },
  click: function (cd, i) {
    if (i === 0) cd.head1 = (SS.num(cd.head1, 0) + 1) % HEADS.length;
    if (i === 1) cd.head2 = (SS.num(cd.head2, 1) + 1) % HEADS.length;
  },
  drag: function (cd, i, m, d, start, S) {
    var k = i === 0 ? "1" : "2", o = i === 0 ? "2" : "1";
    if (i > 1) return;
    var x = start["x" + k] + (m.x - d.x), y = start["y" + k] + (m.y - d.y);
    if (S.shiftKeyDown) {
      var ox = start["x" + o], oy = start["y" + o];
      var ang = Math.round(Math.atan2(y - oy, x - ox) / (Math.PI / 4)) * Math.PI / 4, len = SS.dist({ x: ox, y: oy }, { x: x, y: y });
      x = ox + len * Math.cos(ang); y = oy + len * Math.sin(ang);
    }
    cd["x" + k] = x; cd["y" + k] = y;
  },
  build: function (el, cd) {
    var a = { x: cd.x1, y: cd.y1 }, b = { x: cd.x2, y: cd.y2 }, s = SS.num(cd.size, 10);
    var line = SS.path(el, 0);
    SS.style(line, false, 1);
    SS.contour(el, line, [[a.x, a.y], [b.x, b.y]], false);
    // Spitze an p, Richtung von q nach p
    function head(i, p, q, type) {
      var path = SS.path(el, i), ang = Math.atan2(p.y - q.y, p.x - q.x), c = Math.cos(ang), sn = Math.sin(ang);
      var bx = p.x - s * 0.866 * c, by = p.y - s * 0.866 * sn, nx = -sn * s / 2, ny = c * s / 2;
      SS.style(path, false, 1);
      var filled = type === 2 || type === 4;
      path.pathAttributes.fill = filled ? { category: "fc_Solid", name: "Solid", shape: "solid" } : null;
      if (filled) path.pathAttributes.fillColor = path.pathAttributes.brushColor;
      if (type === 1) SS.contour(el, path, [[bx + nx, by + ny], [p.x, p.y], [bx - nx, by - ny]], false);
      else if (type === 2) { SS.contour(el, path, [[bx + nx, by + ny], [p.x, p.y], [bx - nx, by - ny]], true); }
      else if (type === 3) SS.contour(el, path, [[p.x + nx, p.y + ny], [p.x - nx, p.y - ny]], false);
      else if (type === 4) { var h = s / 2, hx = c * h, hy = sn * h; SS.contour(el, path, [[p.x + nx + hx, p.y + ny + hy], [p.x - nx + hx, p.y - ny + hy], [p.x - nx - hx, p.y - ny - hy], [p.x + nx - hx, p.y + ny - hy]], true); }
      else SS.contour(el, path, [[p.x, p.y], [p.x, p.y], [p.x, p.y]], false);
    }
    head(1, a, b, SS.num(cd.head1, 0));
    head(2, b, a, SS.num(cd.head2, 1));
    SS.trim(el, 3);
    SS.cp(el, 0, a.x, a.y, "Spitze: " + (SS.num(cd.head1, 0) + 1) + " von " + HEADS.length + " (klicken zum Wechseln)", "", true);
    SS.cp(el, 1, b.x, b.y, "Spitze: " + (SS.num(cd.head2, 1) + 1) + " von " + HEADS.length + " (klicken zum Wechseln)", "", true);
    SS.cpTrim(el, 2);
  }
});
