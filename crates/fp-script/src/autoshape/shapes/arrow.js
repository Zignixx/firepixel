// Pfeil (Firepixel): Blockpfeil nach rechts. Parameter: x, y (links oben des Rahmens), width (Schaftlänge),
// height (Schaftdicke), arrowWidth (Breite der Spitze), arrowTip (Länge der Spitze), roundness (Rundung
// der hinteren Ecken).
SS.run({
  init: function (cd, r, at) {
    if (r) {
      var w = r.right - r.left, h = r.bottom - r.top;
      cd.x = r.left; cd.y = r.top;
      cd.arrowTip = Math.max(1, w / 3); cd.width = Math.max(1, w - cd.arrowTip);
      cd.arrowWidth = Math.max(1, h); cd.height = Math.max(1, h / 2);
    } else { cd.x = at.x; cd.y = at.y; cd.width = 200; cd.height = 100; cd.arrowWidth = 200; cd.arrowTip = 100; }
    cd.thickness = cd.height; cd.roundness = 0; cd.arrowLength = 0;
    cd.shapeName = "arrow";
  },
  drag: function (cd, i, m, d, start) {
    var dx = m.x - d.x, dy = m.y - d.y;
    switch (i) {
      case 0: cd.roundness = SS.clamp(start.roundness + dx, 0, Math.min(start.width, start.height / 2)); break;
      case 1: cd.height = SS.clamp(start.height + 2 * dy, 1, start.arrowWidth); break;
      case 2: cd.width = Math.max(1, start.width + dx); break;
      case 3: cd.x = start.x + Math.min(dx, start.width - 1); cd.width = Math.max(1, start.width - dx); cd.height = SS.clamp(start.height + dy, 1, start.arrowWidth); break;
      case 4: cd.arrowWidth = Math.max(cd.height, start.arrowWidth - 2 * dy); cd.y = start.y + (start.arrowWidth - cd.arrowWidth) / 2; break;
      case 5: cd.arrowTip = Math.max(1, start.arrowTip + dx); break;
    }
    cd.thickness = cd.height;
  },
  build: function (el, cd) {
    var x = cd.x, y = cd.y, w = cd.width, t = cd.height, aw = cd.arrowWidth, tip = cd.arrowTip, r = SS.clamp(SS.num(cd.roundness, 0), 0, Math.min(w, t / 2));
    var cy = y + aw / 2, top = cy - t / 2, bot = cy + t / 2, hx = x + w, k = SS.K;
    var n = [];
    if (r > 0) n.push([x, top + r, x, top + r, x, top + r - r * k], [x + r, top, x + r - r * k, top, x + r, top]);
    else n.push([x, top]);
    n.push([hx, top], [hx, y], [hx + tip, cy], [hx, y + aw], [hx, bot]);
    if (r > 0) n.push([x + r, bot, x + r, bot, x + r - r * k, bot], [x, bot - r, x, bot - r + r * k, x, bot - r]);
    else n.push([x, bot]);
    var p = SS.path(el, 0);
    SS.contour(el, p, n, true);
    SS.trim(el, 1);
    SS.cp(el, 0, x + r, top, "Rundung");
    SS.cp(el, 1, x + w / 2, bot, "Dicke");
    SS.cp(el, 2, hx, cy, "Länge des Schafts");
    SS.cp(el, 3, x + w / 4, bot, "Länge und Dicke");
    SS.cp(el, 4, hx, y, "Breite der Spitze");
    SS.cp(el, 5, hx + tip, cy, "Länge der Spitze");
    SS.cpTrim(el, 6);
  }
});
