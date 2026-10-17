// Messwerkzeug (Firepixel): Maßlinie mit Pfeilen, Endstrichen und Längenangabe. Parameter: x1, y1, x2,
// y2, units (px, inch, cm).
SS.run({
  init: function (cd, r, at, S) {
    if (r) {
      var a = S.elem.globalToSmartShapeCoords(S.mouseDownPos), b = S.elem.globalToSmartShapeCoords(S.currentMousePos);
      cd.x1 = a.x; cd.y1 = a.y; cd.x2 = b.x; cd.y2 = b.y;
    } else { cd.x1 = at.x; cd.y1 = at.y; cd.x2 = at.x + 100; cd.y2 = at.y; }
    if (cd.units === undefined) cd.units = "px";
    cd.shapeName = "measure";
  },
  click: function (cd, i) {
    if (i === 2 || i === 3) cd.units = cd.units === "px" ? "inch" : cd.units === "inch" ? "cm" : "px";
  },
  drag: function (cd, i, m, d, start, S) {
    var dx = m.x - d.x, dy = m.y - d.y;
    if (i === 0 || i === 1) {
      var k = i === 0 ? "1" : "2", o = i === 0 ? "2" : "1";
      var x = start["x" + k] + dx, y = start["y" + k] + dy;
      if (S.shiftKeyDown) {
        var ox = start["x" + o], oy = start["y" + o], ang = Math.round(Math.atan2(y - oy, x - ox) / (Math.PI / 4)) * Math.PI / 4, len = SS.dist({ x: ox, y: oy }, { x: x, y: y });
        x = ox + len * Math.cos(ang); y = oy + len * Math.sin(ang);
      }
      cd["x" + k] = x; cd["y" + k] = y;
    } else { cd.x1 = start.x1 + dx; cd.y1 = start.y1 + dy; cd.x2 = start.x2 + dx; cd.y2 = start.y2 + dy; }
  },
  build: function (el, cd) {
    var a = { x: cd.x1, y: cd.y1 }, b = { x: cd.x2, y: cd.y2 }, len = SS.dist(a, b);
    var ang = Math.atan2(b.y - a.y, b.x - a.x), c = Math.cos(ang), s = Math.sin(ang), nx = -s, ny = c;
    var gap = Math.min(20, len / 2), tick = 15, ah = 8, aw = 4;
    var p = SS.path(el, 0);
    SS.style(p, false, 1, "#ff0000");
    SS.contour(el, p, [[a.x, a.y], [a.x + c * gap, a.y + s * gap]], false);
    SS.contour(el, p, [[b.x, b.y], [b.x - c * gap, b.y - s * gap]], false);
    SS.contour(el, p, [[a.x + c * ah * 0.866 + nx * aw, a.y + s * ah * 0.866 + ny * aw], [a.x, a.y], [a.x + c * ah * 0.866 - nx * aw, a.y + s * ah * 0.866 - ny * aw]], false);
    SS.contour(el, p, [[b.x - c * ah * 0.866 - nx * aw, b.y - s * ah * 0.866 - ny * aw], [b.x, b.y], [b.x - c * ah * 0.866 + nx * aw, b.y - s * ah * 0.866 + ny * aw]], false);
    SS.contour(el, p, [[a.x + nx * tick, a.y + ny * tick], [a.x - nx * tick, a.y - ny * tick]], false);
    SS.contour(el, p, [[b.x + nx * tick, b.y + ny * tick], [b.x - nx * tick, b.y - ny * tick]], false);
    var val = cd.units === "inch" ? Math.round(len / 72 * 100) / 100 + " Zoll" : cd.units === "cm" ? Math.round(len / 72 * 2.54 * 100) / 100 + " cm" : Math.round(len) + " px";
    var deg = Math.round(((Math.atan2(a.y - b.y, a.x - b.x) * 180 / Math.PI) % 360 + 360) % 360 * 100) / 100;
    var t = SS.text(el, 1, val, (a.x + b.x) / 2 - val.length * 3.4, (a.y + b.y) / 2 + 4, 12, "#ff0000");
    t.font = "Arial";
    SS.trim(el, 2);
    var tip = "Länge: " + Math.round(len) + " px – Winkel: " + deg.toFixed(2) + "°";
    SS.cp(el, 0, a.x, a.y, tip, "", true);
    SS.cp(el, 1, b.x, b.y, tip, "", true);
    SS.cp(el, 2, a.x + c * gap, a.y + s * gap, "Ziehen: verschieben, klicken: Einheit", "", true);
    SS.cp(el, 3, b.x - c * gap, b.y - s * gap, "Ziehen: verschieben, klicken: Einheit", "", true);
    SS.cpTrim(el, 4);
  }
});
