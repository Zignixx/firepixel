# Erzeugt crates/fp-script/src/bindings_data.rs aus re/api/fw_api.json (Abschnitt exe_bindings).
# Aufruf: python -I tools\golden\fp-script\gen_bindings.py
# Inhalt: je JS-Klasse die Methoden (Parameter mit Typcode und Vorgabe, Rückgabetyp) und Properties
# (Typcode, read-only) sowie die Errors-Tabelle (Name, HRESULT). Nur Namen/Typen/Vorgaben, keine Texte.
# Neutralitaet: der Programmname des Originals in Klassen-/Methodennamen wird durch "Firepixel" ersetzt
# (tools/golden/neutral.py), z. B. Klasse `Firepixel`, `createFirepixelDocument`.
import json
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
import neutral  # noqa: E402

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", "..", ".."))
API = os.path.normpath(os.path.join(ROOT, "..", "re", "api", "fw_api.json"))
OUT = os.path.join(ROOT, "crates", "fp-script", "src", "bindings_data.rs")

CODE = {
    "%Aa": "Array", "%Ab": "Bool", "%Ac": "Color", "%Ad": "Dict", "%Ae": "Date", "%Af": "Float",
    "%Ai": "Int", "%Am": "Matrix", "%Ao": "Object", "%Ap": "Point", "%Ar": "Rect", "%As": "Str",
    "%At": "Any", "%Au": "Url", "%Az": "Resolution", "%Av": "Void", "": "Void",
}


def default(p):
    if p["required"]:
        return "D::Req"
    d = p.get("default")
    if d is None:
        return "D::Null"
    if d is True:
        return "D::Bool(true)"
    if d is False:
        return "D::Bool(false)"
    if isinstance(d, int):
        return "D::Int(%d)" % d
    if isinstance(d, float):
        return "D::Int(%d)" % int(d)
    return "D::Null"


def rs(s):
    return json.dumps(neutral.neutral(s), ensure_ascii=False)


def main():
    api = json.load(open(API, encoding="utf-8"))
    eb = api["exe_bindings"]
    lines = [
        "// Erzeugt von tools/golden/fp-script/gen_bindings.py aus re/api/fw_api.json (exe_bindings). Nicht von Hand ändern.",
        "// Inhalt: Namen, Typcodes, Vorgaben und read-only-Angaben der 1.289 Registrierungen des Originals (Programmname durch „Firepixel“ ersetzt).",
        "",
        "use crate::bindings::{ClassSpec, ErrorSpec, MethodSpec, ParamSpec, PropSpec, TypeCode as T, DefaultArg as D};",
        "",
        "/// Alle Klassen mit ihren Methoden und Properties.",
        "pub static CLASSES: &[ClassSpec] = &[",
    ]
    nm = np = 0
    for tag, c in eb["classes"].items():
        lines.append("    ClassSpec {")
        lines.append("        tag: %s," % rs(tag))
        lines.append("        js: %s," % rs(c.get("js_object") or tag))
        lines.append("        methods: &[")
        for m in c.get("methods", []):
            nm += 1
            ps = ", ".join(
                "ParamSpec { name: %s, code: T::%s, default: %s }" % (rs(p["name"]), CODE.get(p["code"], "Any"), default(p))
                for p in m["params"]
            )
            lines.append(
                "            MethodSpec { name: %s, params: &[%s], ret: T::%s },"
                % (rs(m["name"]), ps, CODE.get(m.get("returns_code") or "", "Any"))
            )
        lines.append("        ],")
        lines.append("        props: &[")
        for p in c.get("properties", []):
            np += 1
            lines.append(
                "            PropSpec { name: %s, code: T::%s, readonly: %s },"
                % (rs(p["name"]), CODE.get(p["code"], "Any"), "true" if p["readonly"] else "false")
            )
        lines.append("        ],")
        lines.append("    },")
    lines.append("];")
    lines.append("")
    lines.append("/// Errors-Codes (Name → HRESULT).")
    lines.append("pub static ERRORS: &[ErrorSpec] = &[")
    for e in eb["errors"]:
        hr = int(e["hresult"], 16) if e.get("hresult") else 0
        lines.append("    ErrorSpec { name: %s, hresult: 0x%08x }," % (rs(e["name"]), hr))
    lines.append("];")
    lines.append("")
    lines.append("/// Anzahl Methoden und Properties.")
    lines.append("pub const COUNTS: (usize, usize) = (%d, %d);" % (nm, np))
    lines.append("")
    with open(OUT, "w", encoding="utf-8", newline="\n") as f:
        f.write("\n".join(lines))
    print("Methoden", nm, "Properties", np, "->", OUT)


if __name__ == "__main__":
    main()
