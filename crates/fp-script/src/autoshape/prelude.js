// Laufzeit der automatischen Formen in Firepixel (eigene Umsetzung der dokumentierten
// Skript-Schnittstelle: smartShape, Path/Contour/ContourNode, ControlPoint, Register*Move).
// Rust übergibt den Zustand als JSON (__in) und liest ihn nach jedem Ereignis zurück (__dump()).

var __moves = [];
var __events = [];
var __alerts = [];

function __list(factory, items) {
  var arr = items || [];
  return new Proxy(arr, {
    // Nur push/splice auf dem Ziel benutzen: direktes `t.length = …` bzw. `t[i] = …` hinter
    // einem verkürzten Array verhält sich in der JS-Engine nicht zuverlässig.
    set: function (t, k, v) {
      if (k === "length") {
        var n = Math.max(0, Math.floor(Number(v)) || 0);
        if (n < t.length) t.splice(n, t.length - n);
        while (t.length < n) t.push(factory(t.length));
        return true;
      }
      var i = Number(k);
      if (String(i) === k && i >= 0) {
        if (i < t.length) t[i] = v;
        else { while (t.length < i) t.push(factory(t.length)); t.push(v); }
        return true;
      }
      t[k] = v;
      return true;
    }
  });
}

function __num(v, d) { var n = Number(v); return isFinite(n) ? n : d; }

function RegisterMoveParms() {
  this.movePred = true; this.movePt = true; this.moveSucc = true;
  this.deltaXtoX = 1; this.deltaYtoY = 1; this.deltaXtoY = 0; this.deltaYtoX = 0;
  this.deltaShortestSideToX = 0; this.deltaLongestSideToX = 0;
  this.deltaShortestSideToY = 0; this.deltaLongestSideToY = 0;
  this.minX = -1e9; this.minY = -1e9; this.maxX = 1e9; this.maxY = 1e9;
  this.constrainXKey = ""; this.constrainX = false; this.constrainYKey = ""; this.constrainY = false;
  this.constrain45Key = ""; this.constrain90Key = ""; this.minMaxRelative = false;
  this.incrementX = 0; this.incrementY = 0;
  this.deltaLinearToLinear = 1; this.minLinear = -1e9; this.maxLinear = 1e9;
  this.constrainAngles = false; this.minAngle = -1e9; this.maxAngle = 1e9;
  this.deltaRtoR = 1; this.minRadius = 0; this.maxRadius = 1e9; this.incrementRadius = 0;
  this.rotate = false; this.constrainRotateKey = ""; this.disableRotateKey = "";
}

function __register(kind, target, a, b) {
  var parms = (kind === "move" || kind === "bbox") ? a : b;
  var other = (kind === "move" || kind === "bbox") ? null : a;
  __moves.push({ kind: kind, target: target, other: other ? { x: __num(other.x, 0), y: __num(other.y, 0) } : null, parms: parms || new RegisterMoveParms() });
}

function __addMoveMethods(proto) {
  proto.RegisterMove = function (p) { __register("move", this, p); };
  proto.RegisterLinearMove = function (o, p) { __register("linear", this, o, p); };
  proto.RegisterCircularMove = function (c, p) { __register("circular", this, c, p); };
  proto.RegisterPolygonMove = function (c, p) { __register("polygon", this, c, p); };
  proto.RegisterInsertBBoxMove = function (p) { __register("bbox", this, p); };
}

function ContourNode(x, y) {
  this.x = __num(x, 0); this.y = __num(y, 0);
  this.predX = this.x; this.predY = this.y; this.succX = this.x; this.succY = this.y;
  this.isCurvePoint = false; this.isSelectedPoint = false; this.randomSeed = 0; this.dynamicInfo = null; this.name = "";
}
__addMoveMethods(ContourNode.prototype);

function Contour() {
  this.isClosed = false;
  this.nodes = __list(function () { return new ContourNode(); });
}

function __defaultAttrs() {
  return JSON.parse(JSON.stringify(__in.defaultAttrs || { fill: null, fillColor: "#000000", brush: null, brushColor: "#000000" }));
}

function Path() {
  this.contours = __list(function () { return new Contour(); });
  this.pathAttributes = __defaultAttrs();
  this.isEvenOddFill = true;
  this.name = null; this.opacity = 100; this.visible = true; this.blendMode = "normal";
  this.customData = {};
  this.__id = null;
}
Path.prototype.toString = function () { return "[object Path]"; };

// Textobjekt einer Form (x/y = Grundlinie links)
function Text() {
  this.__type = "Text"; this.textChars = ""; this.font = "Arial"; this.fontsize = 12; this.fillColor = "#000000";
  this.x = 0; this.y = 0; this.name = null; this.visible = true; this.opacity = 100; this.customData = {}; this.__id = null;
}
Text.prototype.toString = function () { return "[object Text]"; };

function ControlPoint(i) {
  this.x = 0; this.y = 0; this.name = ""; this.toolTip = ""; this.toolTipTracksDrag = false;
  this.type = "default"; this.visible = true; this.index = i || 0; this.hiliteDragOverObject = false;
}
__addMoveMethods(ControlPoint.prototype);

function __mat(m) { return m && m.length === 9 ? m : [1, 0, 0, 0, 1, 0, 0, 0, 1]; }
function __apply(m, p) { return { x: p.x * m[0] + p.y * m[3] + m[6], y: p.x * m[1] + p.y * m[4] + m[7] }; }
function __invert(m) {
  var a = m[0], b = m[1], c = m[3], d = m[4], e = m[6], f = m[7];
  var det = a * d - b * c;
  if (Math.abs(det) < 1e-12) return [1, 0, 0, 0, 1, 0, 0, 0, 1];
  var ia = d / det, ib = -b / det, ic = -c / det, id = a / det;
  return [ia, ib, 0, ic, id, 0, -(e * ia + f * ic), -(e * ib + f * id), 1];
}

function __buildElem(j) {
  var el = {
    isSmartShape: true,
    name: j.name || null,
    customData: j.customData || {},
    transformMode: j.transformMode || "paths",
    __xform: __mat(j.transform),
    elements: __list(function () { return new Path(); }),
    controlPoints: __list(function (i) { return new ControlPoint(i); }),
    toString: function () { return "[object Group]"; },
    smartShapeToGlobalCoords: function (p) { return __apply(this.__xform, p); },
    globalToSmartShapeCoords: function (p) { return __apply(__invert(this.__xform), p); },
    RegisterForEvent: function (ev) { __events.push(String(ev)); return __events.length; },
    UnRegisterForEvent: function (ev) { __events = __events.filter(function (x) { return x !== String(ev); }); },
    UnRegisterAllEvents: function () { __events = []; }
  };
  (j.elements || []).forEach(function (pj) {
    if (pj.type === "text") {
      var tx = new Text();
      tx.__id = pj.id === undefined ? null : pj.id; tx.name = pj.name === undefined ? null : pj.name;
      tx.textChars = pj.text || ""; tx.font = pj.font || "Arial"; tx.fontsize = __num(pj.size, 12); tx.fillColor = pj.color || "#000000";
      tx.x = __num(pj.x, 0); tx.y = __num(pj.y, 0); tx.visible = pj.visible !== false; tx.opacity = __num(pj.opacity, 100);
      el.elements.push(tx);
      return;
    }
    if (pj.keep) { el.elements.push({ __keep: true, __id: pj.id }); return; }
    var p = new Path();
    p.__id = pj.id === undefined ? null : pj.id;
    p.name = pj.name === undefined ? null : pj.name;
    if (pj.pathAttributes) p.pathAttributes = pj.pathAttributes;
    if (pj.isEvenOddFill !== undefined) p.isEvenOddFill = pj.isEvenOddFill;
    if (pj.opacity !== undefined) p.opacity = pj.opacity;
    if (pj.visible !== undefined) p.visible = pj.visible;
    if (pj.customData) p.customData = pj.customData;
    (pj.contours || []).forEach(function (cj) {
      var c = new Contour();
      c.isClosed = !!cj.isClosed;
      (cj.nodes || []).forEach(function (nj) {
        var n = new ContourNode(nj.x, nj.y);
        n.predX = nj.predX; n.predY = nj.predY; n.succX = nj.succX; n.succY = nj.succY;
        n.isCurvePoint = !!nj.isCurvePoint; n.name = nj.name || "";
        c.nodes.push(n);
      });
      p.contours.push(c);
    });
    el.elements.push(p);
  });
  (j.controlPoints || []).forEach(function (cj, i) {
    var cp = new ControlPoint(i);
    cp.x = cj.x; cp.y = cj.y; cp.name = cj.name || ""; cp.toolTip = cj.toolTip || "";
    cp.toolTipTracksDrag = !!cj.toolTipTracksDrag; cp.visible = cj.visible !== false; cp.type = cj.type || "default";
    el.controlPoints.push(cp);
  });
  return el;
}

function __locate(target, el) {
  for (var i = 0; i < el.controlPoints.length; i++) if (el.controlPoints[i] === target) return { cp: i };
  for (var p = 0; p < el.elements.length; p++) {
    var cs = el.elements[p].contours || [];
    for (var c = 0; c < cs.length; c++) {
      var ns = cs[c].nodes;
      for (var n = 0; n < ns.length; n++) if (ns[n] === target) return { path: p, contour: c, node: n };
    }
  }
  return null;
}

function __plain(v) {
  if (v === null || v === undefined) return null;
  if (typeof v === "number") return isFinite(v) ? v : 0;
  if (typeof v !== "object") return v;
  if (Array.isArray(v) || (v.length !== undefined && typeof v !== "string" && typeof v.push === "function")) {
    var a = [];
    for (var i = 0; i < v.length; i++) a.push(__plain(v[i]));
    return a;
  }
  var o = {};
  for (var k in v) if (typeof v[k] !== "function" && k.indexOf("__") !== 0) o[k] = __plain(v[k]);
  return o;
}

function __dump() {
  var el = smartShape.elem;
  var out = {
    customData: __plain(el.customData),
    transformMode: el.transformMode,
    elements: [],
    controlPoints: [],
    getsDragEvents: !!smartShape.getsDragEvents,
    livePreview: !!smartShape.livePreview,
    events: __events.slice(0),
    alerts: __alerts.slice(0),
    moves: []
  };
  for (var p = 0; p < el.elements.length; p++) {
    var e = el.elements[p];
    if (!e) continue;
    if (e.__keep) { out.elements.push({ id: e.__id, keep: true }); continue; }
    if (e.__type === "Text") {
      out.elements.push({ id: e.__id === undefined ? null : e.__id, type: "text", name: e.name === undefined ? null : e.name, text: String(e.textChars),
        font: String(e.font || "Arial"), size: __num(e.fontsize, 12), color: String(e.fillColor || "#000000"), x: __num(e.x, 0), y: __num(e.y, 0),
        visible: e.visible !== false, opacity: __num(e.opacity, 100) });
      continue;
    }    var pj = { id: e.__id === undefined ? null : e.__id, name: e.name === undefined ? null : e.name, pathAttributes: __plain(e.pathAttributes),
      isEvenOddFill: e.isEvenOddFill !== false, opacity: __num(e.opacity, 100), visible: e.visible !== false, customData: __plain(e.customData), contours: [] };
    var cs = e.contours || [];
    for (var c = 0; c < cs.length; c++) {
      var ct = cs[c], nodes = [];
      for (var n = 0; n < ct.nodes.length; n++) {
        var nd = ct.nodes[n];
        nodes.push({ x: __num(nd.x, 0), y: __num(nd.y, 0), predX: __num(nd.predX, __num(nd.x, 0)), predY: __num(nd.predY, __num(nd.y, 0)),
          succX: __num(nd.succX, __num(nd.x, 0)), succY: __num(nd.succY, __num(nd.y, 0)), isCurvePoint: !!nd.isCurvePoint, name: nd.name || "" });
      }
      pj.contours.push({ isClosed: !!ct.isClosed, nodes: nodes });
    }
    out.elements.push(pj);
  }
  for (var i = 0; i < el.controlPoints.length; i++) {
    var cp = el.controlPoints[i];
    out.controlPoints.push({ x: __num(cp.x, 0), y: __num(cp.y, 0), name: cp.name === undefined || cp.name === null ? "" : String(cp.name),
      toolTip: cp.toolTip === undefined || cp.toolTip === null ? "" : String(cp.toolTip), toolTipTracksDrag: !!cp.toolTipTracksDrag,
      visible: cp.visible !== false, type: cp.type || "default" });
  }
  for (var m = 0; m < __moves.length; m++) {
    var mv = __moves[m], loc = __locate(mv.target, el);
    if (loc) out.moves.push({ kind: mv.kind, target: loc, other: mv.other, parms: __plain(mv.parms) });
  }
  return JSON.stringify(out);
}

var fw = {
  ellipseBCPConst: 0.55228474983079356,
  appName: "Firepixel",
  platform: "win",
  selection: [],
  getDocumentDOM: function () { return { width: __in.docWidth || 0, height: __in.docHeight || 0, redrawSmartShape: function () {} }; },
  getPref: function () { return null; },
  setPref: function () {},
  yesNoDialog: function () { return true; },
  locConfigDir: "",
  currentScriptDir: ""
};
function alert(t) { __alerts.push(String(t)); }
function prompt(t, d) { return d === undefined ? null : d; }
function confirm() { return true; }

var smartShape = {
  operation: "",
  elem: null,
  mouseDownPos: { x: 0, y: 0 }, prevMousePos: { x: 0, y: 0 }, currentMousePos: { x: 0, y: 0 },
  shiftKeyDown: false, ctrlCmdKeyDown: false, altOptKeyDown: false,
  currentControlPoint: null, currentControlPointName: "", currentControlPointIndex: -1,
  livePreview: true, getsDragEvents: false,
  constrainDragInsertAspect: false, constrainDragInsertAspectKey: "shift",
  GetDefaultMoveParms: function () { return new RegisterMoveParms(); }
};

function __begin(state) {
  __in = state;
  __moves = [];
  __alerts = [];
  smartShape.operation = state.operation;
  smartShape.elem = __buildElem(state.elem);
  smartShape.mouseDownPos = state.mouseDownPos;
  smartShape.prevMousePos = state.prevMousePos;
  smartShape.currentMousePos = state.currentMousePos;
  smartShape.shiftKeyDown = !!state.shift;
  smartShape.ctrlCmdKeyDown = !!state.ctrl;
  smartShape.altOptKeyDown = !!state.alt;
  smartShape.currentControlPointIndex = state.cpIndex === undefined ? -1 : state.cpIndex;
  var cps = smartShape.elem.controlPoints;
  smartShape.currentControlPoint = (state.cpIndex >= 0 && state.cpIndex < cps.length) ? cps[state.cpIndex] : null;
  smartShape.currentControlPointName = smartShape.currentControlPoint ? smartShape.currentControlPoint.name : "";
  smartShape.getsDragEvents = !!state.getsDragEvents;
  smartShape.livePreview = state.livePreview !== false;
  __events = state.elem.events || [];
}
var __in = {};
