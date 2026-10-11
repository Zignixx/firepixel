// Rechteck mit Ecken (Firepixel): gemeinsames Skript für „Abgerundetes“, „Abgeflachtes“ und
// „Abgefastes Rechteck“ (Vorgabe-Eckenart `__cornerDefault` steht davor).
// Parameter: x, y, width, height, corner0..3 (Größe je Ecke, oben links im Uhrzeigersinn),
// cornerType0..3 (0 gerade abgeschrägt, 1 rund, 2 nach innen gewölbt), locked (Ecken gemeinsam).
var CORNER_DEFAULT = typeof __cornerDefault === "number" ? __cornerDefault : 1;
SS.run({
  init: function (cd, r, at) {
    var w = r ? r.right - r.left : 200, h = r ? r.bottom - r.top : 150;
    cd.x = r ? r.left : at.x; cd.y = r ? r.top : at.y;
    cd.width = Math.max(1, w); cd.height = Math.max(1, h);
    var c = Math.min(cd.width, cd.height) / 4;
    cd.corner0 = c; cd.corner1 = c; cd.corner2 = c; cd.corner3 = c;
    if (cd.locked === undefined) cd.locked = true;
    for (var i = 0; i < 4; i++) if (cd["cornerType" + i] === undefined) cd["cornerType" + i] = CORNER_DEFAULT;
    cd.shapeName = "rectangle";
  },
  click: function (cd, i) {
    if (i < 0 || i > 3) return;
    var keys = cd.locked ? [0, 1, 2, 3] : [i];
    var next = (SS.num(cd["cornerType" + i], 1) + 1) % 3;
    for (var k = 0; k < keys.length; k++) cd["cornerType" + keys[k]] = next;
  },
  drag: function (cd, i, m, d, start, S) {
    var dx = m.x - d.x, dy = m.y - d.y, lim = Math.min(cd.width, cd.height) / 2;
    if (i === 4) {
      cd.width = Math.max(1, start.width + dx);
      cd.height = Math.max(1, start.height + dy);
      if (S.shiftKeyDown) { var s = Math.max(cd.width, cd.height); cd.width = s; cd.height = s; }
      return;
    }
    if (i < 0 || i > 3) return;
    var sign = (i === 0 || i === 3) ? 1 : -1;
    var single = S.altOptKeyDown || !cd.locked;
    if (S.altOptKeyDown) cd.locked = false;
    for (var k = 0; k < 4; k++) {
      if (single && k !== i) continue;
      cd["corner" + k] = SS.clamp(SS.num(start["corner" + k], 0) + sign * dx, 0, lim);
    }
  },
  build: function (el, cd) {
    var x = cd.x, y = cd.y, w = cd.width, h = cd.height, lim = Math.min(w, h) / 2;
    var r = [], t = [];
    for (var i = 0; i < 4; i++) { r.push(SS.clamp(SS.num(cd["corner" + i], 0), 0, lim)); t.push(SS.num(cd["cornerType" + (cd.locked ? 0 : i)], 1)); }
    var k = SS.K;
    // a = Anfang (auf der vorherigen Kante), b = Ende (auf der nächsten Kante), c = Eckpunkt
    function corner(ax, ay, bx, by, cx, cy, rr, type) {
      if (rr <= 0 || type === 0) return [[ax, ay], [bx, by]];
      if (type === 2) {
        var mx = ax + bx - cx, my = ay + by - cy;
        return [[ax, ay, ax, ay, ax + (mx - ax) * k, ay + (my - ay) * k], [bx, by, bx + (mx - bx) * k, by + (my - by) * k, bx, by]];
      }
      return [[ax, ay, ax, ay, ax + (cx - ax) * k, ay + (cy - ay) * k], [bx, by, bx + (cx - bx) * k, by + (cy - by) * k, bx, by]];
    }
    var n = [].concat(
      corner(x, y + r[0], x + r[0], y, x, y, r[0], t[0]),
      corner(x + w - r[1], y, x + w, y + r[1], x + w, y, r[1], t[1]),
      corner(x + w, y + h - r[2], x + w - r[2], y + h, x + w, y + h, r[2], t[2]),
      corner(x + r[3], y + h, x, y + h - r[3], x, y + h, r[3], t[3]));
    var p = SS.path(el, 0);
    SS.contour(el, p, n, true);
    SS.trim(el, 1);
    var tipA = "Klicken: Eckenart wechseln", tipB = "Alt beim Ziehen: nur diese Ecke";
    SS.cp(el, 0, x + r[0], y, tipA, t[0]);
    SS.cp(el, 1, x + w - r[1], y, tipB, t[1]);
    SS.cp(el, 2, x + w - r[2], y + h, tipA, t[2]);
    SS.cp(el, 3, x + r[3], y + h, tipB, t[3]);
    SS.cp(el, 4, x + w + 10, y + h + 10, "Größe ändern", "");
    SS.cpTrim(el, 5);
  }
});
