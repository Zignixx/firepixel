// Spirale (Firepixel, re 36 §2): gleichwinklige (logarithmische) Spirale r = a·e^(θ·cot b) um die
// Mitte, im Uhrzeigersinn ab θ = 0 (rechts) bis r = R. Parameter: cx, cy, radius (R), d (Lage des
// Windungsgriffs, −1 oben … +1 unten), closed, clockwise, a (Anfangsradius; fehlt = 0,5; beim
// Aufziehen wird die Nennspirale nur skaliert, a schrumpft mit), turns (Anzeige/Eingabe).
var SP_A0 = 0.5, SP_BMIN = 50, SP_BMAX = 89, SP_D0 = 1 - 2 * (80 - 50) / (89 - 50);
function spB(d) { var t = (1 - d) / 2 * 9; return Math.log(t + 1) / Math.log(12) * (SP_BMAX - SP_BMIN) + SP_BMIN; }
// Umkehrung zu spB: Steigung (Grad) → Griffwert d
function spD(b) { var t = Math.pow(12, (b - SP_BMIN) / (SP_BMAX - SP_BMIN)) - 1; return 1 - 2 * t / 9; }
SS.run({
  lockAspect: true,
  init: function (cd, r, at) {
    var q = r ? SS.radial(r, at) : null;
    var rad = q ? Math.max(1, q.rad) : 100;
    cd.cx = q ? q.cx : at.x + rad; cd.cy = q ? q.cy : at.y + rad;
    cd.radius = rad;
    // Aufziehen skaliert die Nennspirale (R 100, a 0,5): gleiche Knotenzahl und Windungen
    if (q) cd.a = SP_A0 * rad / 100; else delete cd.a;
    if (cd.d === undefined) { cd.d = SP_D0; cd.clockwise = true; cd.closed = false; }
    delete cd.turns; delete cd.turnsShown;
    cd.shapeName = "spiral";
  },
  click: function (cd, i) { if (i === 0) { cd.closed = !cd.closed; delete cd.a; } },
  drag: function (cd, i, m, d, start) {
    var R = Math.max(1, SS.num(start.radius, 100));
    if (i === 1) { cd.radius = Math.max(1, Math.round(SS.dist({ x: cd.cx, y: cd.cy }, m))); delete cd.a; }
    else if (i === 2) {
      var y0 = start.cy + R * SS.clamp(SS.num(start.d, SP_D0), -1, 1);
      var y = SS.clamp(y0 + (m.y - d.y), start.cy - R, start.cy + R);
      cd.d = (y - start.cy) / R;
      delete cd.a;
    }
  },
  build: function (el, cd) {
    var R = Math.max(1, SS.num(cd.radius, 100)), a = SS.num(cd.a, SP_A0);
    if (!(a > 0)) a = SP_A0;
    if (a >= R) a = R / 2;
    // Windungen als Eingabe (Eigenschaften): in die Steigung b umrechnen
    if (cd.turns !== undefined && cd.turnsShown !== undefined && Number(cd.turns) !== Number(cd.turnsShown)) {
      var n0 = Math.max(0.25, SS.num(cd.turns, 6)), cot0 = Math.log(R / a) / (2 * Math.PI * n0);
      var b0 = SS.clamp(Math.atan(1 / cot0) * 180 / Math.PI, SP_BMIN, spB(-1));
      cd.d = SS.clamp(spD(b0), -1, 1);
      delete cd.a; a = SP_A0;
      if (a >= R) a = R / 2;
    }
    var d = SS.clamp(SS.num(cd.d, SP_D0), -1, 1), b = spB(d) * Math.PI / 180, cot = 1 / Math.tan(b);
    var dir = cd.clockwise === false ? -1 : 1, cx = cd.cx, cy = cd.cy, half = Math.PI / 2, K = SS.K;
    var ppc = a * Math.exp(4 * Math.PI * cot) >= R ? 8 : 4, dt = 2 * Math.PI / ppc;
    var thEnd = Math.max(1e-6, Math.log(R / a) / cot), th = [];
    for (var i = 0; i * dt < thEnd - 1e-9; i++) th.push(i * dt);
    th.push(thEnd);
    function rad(t) { return a * Math.exp(t * cot); }
    function pt(t) { var r = rad(t); return { x: cx + r * Math.cos(t), y: cy + dir * r * Math.sin(t) }; }
    // Einheitsvektor in Laufrichtung (wachsendes θ): Tangentenwinkel θ + b
    function fw(t) { return { x: Math.cos(t + b), y: dir * Math.sin(t + b) }; }
    // Henkellänge zur Spanne s (Bogenmaß): r·κ·s/(π/2)
    function hl(t, s) { return rad(t) * K * s / half; }
    var n = [], N = th.length;
    for (i = 0; i < N; i++) {
      var t = th[i], p = pt(t), f = fw(t);
      var li = i > 0 ? hl(t, t - th[i - 1]) : 0, lo = i < N - 1 ? hl(t, th[i + 1] - t) : 0;
      n.push([p.x, p.y, p.x - f.x * li, p.y - f.y * li, p.x + f.x * lo, p.y + f.y * lo]);
    }
    if (cd.closed) {
      // Rückbahn: gerade vom Ende zum Punkt eine Windung weiter innen, dann die Bahn rückwärts zur
      // Mitte (vertauschte Henkel); der Schluss zur Mitte bekommt einen Henkel am ersten Knoten.
      var tq = thEnd - 2 * Math.PI;
      if (tq > 0) {
        var j = 0;
        while (j + 1 < N && th[j + 1] < tq - 1e-9) j++;
        var q = pt(tq), fq = fw(tq), lq = hl(tq, tq - th[j]);
        n.push([q.x, q.y, q.x, q.y, q.x - fq.x * lq, q.y - fq.y * lq]);
        for (var k = j; k >= 1; k--) {
          var tk = th[k], pk = pt(tk), fk = fw(tk), nxt = k === j ? tq : th[k + 1];
          var lin = hl(tk, nxt - tk), lout = hl(tk, tk - th[k - 1]);
          n.push([pk.x, pk.y, pk.x + fk.x * lin, pk.y + fk.y * lin, pk.x - fk.x * lout, pk.y - fk.y * lout]);
        }
        var f0 = fw(0), l0 = hl(0, (N > 1 ? th[1] : dt));
        n[0][2] = n[0][0] + f0.x * l0; n[0][3] = n[0][1] + f0.y * l0;
      }
    }
    var p0 = SS.path(el, 0);
    if (p0.__id === null || p0.__id === undefined) {
      // offen ohne Füllung; ohne Werkzeugstrich 1 px schwarz
      var at = p0.pathAttributes;
      at.fill = null;
      if (!at.brush) { at.brush = { category: "bc_Pencil", name: "1-Pixel Soft", diameter: 1 }; at.brushColor = "#000000"; }
    }
    SS.contour(el, p0, n, !!cd.closed);
    SS.trim(el, 1);
    var turns = Math.round(thEnd / (2 * Math.PI));
    cd.turns = turns; cd.turnsShown = turns;
    var e = n[N - 1];
    SS.cp(el, 0, cx, cy, "Spirale öffnen/schließen", cd.closed ? "closed" : "open");
    SS.cp(el, 1, e[0], e[1], "", "", false, false); // Radiusgriff wie im Original ohne Text (TW-90)
    SS.cp(el, 2, cx - R, cy + R * d, "Spiralen: " + turns, turns, true);
    SS.cpTrim(el, 3);
  }
});
