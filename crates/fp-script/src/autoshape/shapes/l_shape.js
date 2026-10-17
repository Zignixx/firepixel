// L-Form (Firepixel, re 36 §4): oberer und linker Schenkel. Parameter: x, y, width, height,
// thickness (Schenkeldicke), roundness (Rundung der äußeren Ecke). Klick: 200 × 200, Dicke 100;
// Aufziehen: Außenmaße = Zugrechteck, Dicke = kürzere Seite / 3.
SS.run({
  init: function (cd, r, at) {
    cd.x = r ? r.left : at.x; cd.y = r ? r.top : at.y;
    cd.width = r ? Math.max(1, r.right - r.left) : 200; cd.height = r ? Math.max(1, r.bottom - r.top) : 200;
    cd.thickness = r ? Math.min(cd.width, cd.height) / 3 : 100; cd.roundness = 0;
    cd.shapeName = "lshape";
  },
  drag: function (cd, i, m, d, start) {
    var dx = m.x - d.x, dy = m.y - d.y, w = start.width, h = start.height, t = start.thickness, r0 = SS.num(start.roundness, 0);
    if (i === 0) {
      // erst bis zur Dicke (Innenecke bleibt eckig); liegt der Griff schon jenseits, bis min(B, H)
      var lim = r0 >= t ? Math.min(w, h) : t;
      cd.roundness = SS.clamp(r0 + dx, 0, lim);
    } else if (i === 1) {
      // Dicke beider Schenkel, begrenzt durch die freie Länge und die Rundung
      var hi = Math.min(w, h) - 1;
      cd.thickness = SS.clamp(t + dx, Math.max(1, Math.min(r0, hi)), Math.max(1, hi));
    } else if (i === 2) { cd.width = Math.max(t + 1, w + dx); cd.height = h; }
    else if (i === 3) { cd.height = Math.max(t + 1, h + dy); cd.width = w; }
  },
  build: function (el, cd) {
    var x = cd.x, y = cd.y, w = cd.width, h = cd.height, t = SS.clamp(SS.num(cd.thickness, 100), 1, Math.min(w, h));
    var r = SS.clamp(SS.num(cd.roundness, 0), 0, Math.min(w, h)), ri = Math.max(r - t, 0), k = SS.K;
    // 8 Knoten (re 36 §4.1): äußere Rundung oben, rechts oben, Schenkelende, Innenecke (2), Fuß, links unten, Rundung links
    var n = [
      [x + r, y, x + r - r * k, y, x + r, y],
      [x + w, y],
      [x + w, y + t],
      [x + t + ri, y + t, x + t + ri, y + t, x + t + ri - ri * k, y + t],
      [x + t, y + t + ri, x + t, y + t + ri - ri * k, x + t, y + t + ri],
      [x + t, y + h],
      [x, y + h],
      [x, y + r, x, y + r, x, y + r - r * k]
    ];
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
