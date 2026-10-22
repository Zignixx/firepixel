// Hilfen für die eingebauten automatischen Formen von Firepixel (eigener Code).
// Jede Form beschreibt sich mit SS.shape({...}); SS.run() verteilt die Ereignisse.

var SS = {
  K: 0.55228474983079356,

  num: function (v, d) { var n = Number(v); return isFinite(n) ? n : d; },
  clamp: function (v, a, b) { return Math.max(a, Math.min(b, v)); },
  dist: function (a, b) { return Math.sqrt((a.x - b.x) * (a.x - b.x) + (a.y - b.y) * (a.y - b.y)); },
  ang: function (c, p) { return Math.atan2(p.y - c.y, p.x - c.x); },
  pol: function (c, r, a) { return { x: c.x + r * Math.cos(a), y: c.y + r * Math.sin(a) }; },
  copy: function (o) { return JSON.parse(JSON.stringify(o)); },

  // Pfad Nummer i der Form (vorhandener bleibt erhalten, Konturen werden geleert)
  path: function (el, i) {
    while (el.elements.length <= i) el.elements.push(new Path());
    var p = el.elements[i];
    if (!p || !p.contours) { p = new Path(); el.elements[i] = p; }
    p.contours.length = 0;
    return p;
  },
  // Text Nummer i (x/y = Grundlinie links, Formkoordinaten)
  text: function (el, i, s, x, y, size, color) {
    while (el.elements.length <= i) el.elements.push(new Text());
    var t = el.elements[i];
    if (!t || t.__type !== "Text") { t = new Text(); el.elements[i] = t; }
    var g = el.smartShapeToGlobalCoords({ x: x, y: y });
    t.textChars = String(s); t.x = g.x; t.y = g.y;
    if (size) t.fontsize = size;
    if (color) t.fillColor = color;
    return t;
  },
  // Pfadattribute: Füllung an/aus, Strich (Breite, Farbe)
  // (nur für neue Pfade: Änderungen des Nutzers an vorhandenen Pfaden bleiben erhalten)
  style: function (p, fill, strokeWidth, strokeColor, fillColor) {
    if (p.__id !== null && p.__id !== undefined) return;
    var a = p.pathAttributes;
    if (fill === false) a.fill = null; else if (fill === true && !a.fill) a.fill = { category: "fc_Solid", name: "Solid", shape: "solid" };
    if (fillColor) a.fillColor = fillColor;
    if (strokeWidth === 0) a.brush = null;
    else if (strokeWidth) a.brush = { category: "bc_Pencil", name: "1-Pixel Hard", diameter: strokeWidth };
    if (strokeColor) a.brushColor = strokeColor;
  },
  // Own tip of a new path (Arrow Line, Measure Tool, Connector Line and Spiral, RE spec 59 section 1.5):
  // stroke preset by internal category and name (`bc_Basic`/`bn_Hard Line` …), size and colour; the
  // preset decides anti-aliasing. Existing paths keep the user's changes.
  brush: function (p, category, name, diameter, color) {
    if (p.__id !== null && p.__id !== undefined) return;
    var a = p.pathAttributes;
    a.brush = { category: category, name: name, diameter: diameter };
    if (color) a.brushColor = color;
  },
  // überzählige Pfade ab Nummer n entfernen
  trim: function (el, n) { if (el.elements.length > n) el.elements.length = n; },

  // Kontur aus Knoten [[x, y, predX, predY, succX, succY] | [x, y]] in Formkoordinaten
  contour: function (el, p, nodes, closed) {
    var c = new Contour();
    c.isClosed = closed !== false;
    for (var i = 0; i < nodes.length; i++) {
      var n = nodes[i];
      var a = el.smartShapeToGlobalCoords({ x: n[0], y: n[1] });
      var pr = n.length >= 4 ? el.smartShapeToGlobalCoords({ x: n[2], y: n[3] }) : a;
      var su = n.length >= 6 ? el.smartShapeToGlobalCoords({ x: n[4], y: n[5] }) : a;
      var cn = new ContourNode(a.x, a.y);
      cn.predX = pr.x; cn.predY = pr.y; cn.succX = su.x; cn.succY = su.y;
      cn.isCurvePoint = n.length >= 4 && (n[2] !== n[0] || n[3] !== n[1] || n[4] !== n[0] || n[5] !== n[1]);
      c.nodes.push(cn);
    }
    p.contours.push(c);
    return c;
  },

  // Kreis/Ellipse als 4 Knoten
  ellipse: function (el, p, cx, cy, rx, ry, reverse) {
    var k = SS.K, n = [
      [cx, cy - ry, cx - rx * k, cy - ry, cx + rx * k, cy - ry],
      [cx + rx, cy, cx + rx, cy - ry * k, cx + rx, cy + ry * k],
      [cx, cy + ry, cx + rx * k, cy + ry, cx - rx * k, cy + ry],
      [cx - rx, cy, cx - rx, cy + ry * k, cx - rx, cy - ry * k]
    ];
    if (reverse) {
      n.reverse();
      for (var i = 0; i < n.length; i++) { var t = [n[i][2], n[i][3]]; n[i][2] = n[i][4]; n[i][3] = n[i][5]; n[i][4] = t[0]; n[i][5] = t[1]; }
    }
    return SS.contour(el, p, n, true);
  },

  // Bogen um (cx, cy) von Winkel a0 nach a1 (Bogenmaß) als Knotenliste, Segmente ≤ 90°
  arc: function (cx, cy, rx, ry, a0, a1) {
    var out = [], n = Math.max(1, Math.ceil(Math.abs(a1 - a0) / (Math.PI / 2) - 1e-9));
    var step = (a1 - a0) / n, h = 4 / 3 * Math.tan(step / 4);
    for (var i = 0; i <= n; i++) {
      var a = a0 + step * i, c = Math.cos(a), s = Math.sin(a);
      var x = cx + rx * c, y = cy + ry * s;
      var tx = -rx * s * h, ty = ry * c * h;
      out.push([x, y, i === 0 ? x : x - tx, i === 0 ? y : y - ty, i === n ? x : x + tx, i === n ? y : y + ty]);
    }
    return out;
  },

  // Steuerpunkt i setzen (Formkoordinaten)
  cp: function (el, i, x, y, tip, name, tracks, visible) {
    while (el.controlPoints.length <= i) el.controlPoints.push(new ControlPoint(el.controlPoints.length));
    var g = el.smartShapeToGlobalCoords({ x: x, y: y }), c = el.controlPoints[i];
    c.x = g.x; c.y = g.y; c.toolTip = tip || ""; c.name = name === undefined ? "" : String(name);
    c.toolTipTracksDrag = !!tracks; c.visible = visible !== false; c.index = i;
    return c;
  },
  cpTrim: function (el, n) { if (el.controlPoints.length > n) el.controlPoints.length = n; },

  // Ziehrahmen beim Aufziehen (Alt = von der Mitte). Formen mit `lockAspect` (Spirale, Polygon, Stern,
  // Kreisring, Torte) machen daraus immer ein Quadrat mit der längeren Seite; die Zugrichtung bleibt
  // erhalten (re 36 §1.3). Umschalt macht bei freien Formen (Rechteck-Familie, Pfeil, L-Form …) dasselbe,
  // bei gesperrten ändert es nichts (TW-88, Lauf 9: abgerundetes Rechteck und Pfeil 121 × 121).
  dragRect: function (a, b, S, lock) {
    var dx = b.x - a.x, dy = b.y - a.y;
    if (lock || S.shiftKeyDown) {
      // ohne Breite (bzw. Höhe) folgt die fehlende Richtung der anderen (senkrechter Zug nach oben → links)
      var m = Math.max(Math.abs(dx), Math.abs(dy)), sx = dx < 0 ? -1 : dx > 0 ? 1 : 0, sy = dy < 0 ? -1 : dy > 0 ? 1 : 0;
      if (sx === 0) sx = sy || 1;
      if (sy === 0) sy = sx;
      dx = sx * m; dy = sy * m;
    }
    var l, t, r, bt;
    if (S.altOptKeyDown) { l = a.x - dx; r = a.x + dx; t = a.y - dy; bt = a.y + dy; } else { l = a.x; r = a.x + dx; t = a.y; bt = a.y + dy; }
    return { left: Math.min(l, r), top: Math.min(t, bt), right: Math.max(l, r), bottom: Math.max(t, bt) };
  },

  // Runde Formen (Stern, Torte/Donut, Intelligentes Polygon, Spirale) aus dem Ziehrahmen wie im
  // Original: Radius = halbe längere Seite, der Kreis beginnt am Startpunkt und liegt in Zugrichtung
  // (120 × 100 ab 100,100 → Mitte 160,160, Radius 60; senkrechter Zug ohne Breite: waagrecht in
  // Richtung des senkrechten Zugs). Rahmen um den Startpunkt (Alt = von der Mitte): Mitte = Startpunkt.
  radial: function (r, at) {
    var e = 1e-9, rad = Math.max(r.right - r.left, r.bottom - r.top) / 2;
    if ((r.left < at.x - e && r.right > at.x + e) || (r.top < at.y - e && r.bottom > at.y + e))
      return { cx: (r.left + r.right) / 2, cy: (r.top + r.bottom) / 2, rad: rad };
    var sx = r.right > at.x + e ? 1 : r.left < at.x - e ? -1 : 0;
    var sy = r.bottom > at.y + e ? 1 : r.top < at.y - e ? -1 : 0;
    if (sx === 0) sx = sy || 1;
    if (sy === 0) sy = sx;
    return { cx: at.x + sx * rad, cy: at.y + sy * rad, rad: rad };
  },

  // Ereignisverteilung. def: { init(cd, rect|null, at, S), build(el, cd, S), drag(cd, index, mouse, down, start, S),
  //   click(cd, index, S) (optional, Steuerpunkt ohne Bewegung angeklickt), defaultSize: [w, h],
  //   lockAspect (Seitenverhältnis beim Aufziehen gesperrt) }
  run: function (def) {
    var S = smartShape, el = S.elem, cd = el.customData;
    var L = function (p) { return el.globalToSmartShapeCoords(p); };
    switch (S.operation) {
      case "InsertSmartShapeAt":
        def.init(cd, null, L(S.mouseDownPos), S);
        break;
      case "BeginDragInsert":
      case "DragInsert":
      case "EndDragInsert": {
        var r = SS.dragRect(L(S.mouseDownPos), L(S.currentMousePos), S, !!def.lockAspect);
        if (r.right - r.left < 1 && r.bottom - r.top < 1) def.init(cd, null, L(S.mouseDownPos), S);
        else def.init(cd, r, L(S.mouseDownPos), S);
        break;
      }
      case "BeginDragControlPoint":
        S.getsDragEvents = true;
        var st = {};
        for (var k in cd) if (k !== "_drag") st[k] = cd[k];
        cd._drag = SS.copy(st);
        return;
      case "DragControlPoint":
      case "EndDragControlPoint": {
        var start = cd._drag || SS.copy(cd);
        for (var k2 in start) cd[k2] = SS.copy(start[k2]);
        var m = L(S.currentMousePos), d = L(S.mouseDownPos);
        var moved = Math.abs(S.currentMousePos.x - S.mouseDownPos.x) + Math.abs(S.currentMousePos.y - S.mouseDownPos.y) > 0.01;
        if (!moved && S.operation === "EndDragControlPoint" && def.click) def.click(cd, S.currentControlPointIndex, S);
        else if (moved) def.drag(cd, S.currentControlPointIndex, m, d, start, S);
        if (S.operation === "EndDragControlPoint") delete cd._drag;
        break;
      }
      default:
        break;
    }
    def.build(el, cd, S);
  }
};
