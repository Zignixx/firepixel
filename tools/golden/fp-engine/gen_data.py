# -*- coding: utf-8 -*-
"""Erzeugt die Datentabellen von fp-engine aus den Spezifikationsdaten (nur lesen).

Quellen:
  re/werkzeuge/menuebefehle.json   Hauptmenue (260 Befehle) + 20 Kuerzel ohne Menueeintrag
  korpus/German/Keyboard Shortcuts/*.xml   fuenf Tastenkuerzel-Sets (nur die Zuordnungen)
  re/farbe/voreinstellungen.json   644 Voreinstellungen mit Vorgaben

Ausgabe (crates/fp-engine/data/):
  commands.json   stabile Befehlsnamen, deutsche Menuepfade, Statuszeilentexte, JS-Namen
  shortcuts.json  je Set: cmd_-Name -> [[Modifikatoren, VK], ...] und Werkzeug-GUID -> [[0, VK], ...]
  prefs.json      Schluessel, Typ, Vorgabe, Kategorie, Wirkung

Neutralitaet: Programm- und Herstellername werden in allen Ausgaben ersetzt (tools/golden/neutral.py);
das Standardset heisst "Firepixel".

Aufruf: python -I tools/golden/fp-engine/gen_data.py
"""
import json
import os
import re
import sys
import xml.etree.ElementTree as ET

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
import neutral  # noqa: E402

HERE = os.path.dirname(os.path.abspath(__file__))
FP = os.path.normpath(os.path.join(HERE, "..", "..", ".."))
REA = os.path.normpath(os.path.join(FP, ".."))
OUT = os.path.join(FP, "crates", "fp-engine", "data")

MENU_JSON = os.path.join(REA, "re", "werkzeuge", "menuebefehle.json")
PREF_JSON = os.path.join(REA, "re", "farbe", "voreinstellungen.json")
SHORTCUT_DIR = os.path.join(neutral.CORPUS, "German", "Keyboard Shortcuts")
# erstes Set = Standardset des Originals (Dateiname = Programmname, aus der Installationskopie)
SETS = [neutral.names()[0], "Photoshop", "Illustrator", "FreeHand", "Web Standard"]

TOP = {
    "Datei": "file", "Bearbeiten": "edit", "Ansicht": "view", "Auswählen": "select",
    "Modifizieren": "modify", "Text": "text", "Befehle": "commands", "Filter": "filter",
    "Fenster": "window", "Hilfe": "help",
}

# Feste Namen fuer Eintraege ohne (brauchbaren) englischen Pfad.
OVERRIDE = {
    57616: "file.openRecent",
    57602: "file.close",
    35789: "file.closeAll",
    35850: "file.saveAll",
    35792: "file.fileInfo",
    57605: "file.pageSetup",
    57665: "file.exit",
    35415: "edit.copyHtmlCode",
    35133: "view.grid.editGrid",
    35849: "modify.snapToPixel",
    35343: "text.font",
    57648: "window.duplicateWindow",
    35848: "window.extensions",
    49125: "window.cssProperties",
    57650: "window.cascade",
    57652: "window.tile",
    57667: "help.firepixelHelp",
    35323: "help.supportCenter",
    7028: "help.productImprovement",
    7013: "help.firepixelExchange",
    35580: "help.onlineForums",
    57664: "help.about",
    35038: "select.all",
    35040: "select.none",
    35039: "select.inverse",
    35041: "select.similar",
    35194: "select.superselect",
    35193: "select.subselect",
    35035: "filter.repeat",
    43422: "window.autoShapes",
    35807: "modify.symbol.swapSymbol",
}

SHORTCUT_ONLY = {
    "cmd_BigNudgeDown": "edit.nudge.bigDown", "cmd_BigNudgeLeft": "edit.nudge.bigLeft",
    "cmd_BigNudgeRight": "edit.nudge.bigRight", "cmd_BigNudgeUp": "edit.nudge.bigUp",
    "cmd_NudgeDown": "edit.nudge.down", "cmd_NudgeLeft": "edit.nudge.left",
    "cmd_NudgeRight": "edit.nudge.right", "cmd_NudgeUp": "edit.nudge.up",
    "cmd_EditImageObject": "modify.editBitmap", "cmd_ExitImageEdit": "modify.exitBitmapMode",
    "cmd_Frame_Next": "view.state.next", "cmd_Frame_Previous": "view.state.previous",
    "cmd_Next_Selection": "select.next", "cmd_Prev_Selection": "select.previous",
    "cmd_Page_Next": "view.page.next", "cmd_Page_Previous": "view.page.previous",
    "cmd_TextAlign_Stretch": "text.align.stretched", "cmd_TextEditor": "text.editor",
    "cmd_ViewEditGuides": "view.guides.editGuides", "cmd_jquery": "window.jquery",
}


def camel(words):
    words = [w for w in re.split(r"[^0-9A-Za-z]+", words) if w]
    if not words:
        return ""
    # Abkuerzungen (JPEG, HTML, URL, PNG) wie normale Woerter behandeln
    words = [w.capitalize() if len(w) > 1 and w.isupper() else w for w in words]
    out = words[0][0].lower() + words[0][1:]
    for w in words[1:]:
        out += w[0].upper() + w[1:]
    return out


SEGMENT_FIX = {"9SliceScalingGuides": "nineSliceScalingGuides"}

LABEL_FIX = {
    "select.next": "Nächste Auswahl",
    "select.previous": "Vorherige Auswahl",
    "view.page.next": "Nächste Seite",
    "view.page.previous": "Vorherige Seite",
    "view.guides.editGuides": "Hilfslinien bearbeiten",
    "window.jquery": "jQuery",
    "text.font": "Schrift",
    "file.openRecent": "Zuletzt geöffnete Datei",
}


def name_from_en(path_en):
    parts = path_en.split("::")
    top = parts[0].lower()
    if top == "filters":
        top = "filter"
    segs = [top]
    for p in parts[1:]:
        c = camel(p)
        segs.append(SEGMENT_FIX.get(c, c))
    return ".".join(segs)


def label_parts(pfad):
    return [p.strip() for p in pfad.split(">")]


def vk_list(cmd_shortcuts):
    out = []
    for s in cmd_shortcuts:
        txt = s.get("text", "").split()
        if len(txt) == 2 and txt[0].isdigit() and txt[1].isdigit():
            out.append([int(txt[0]), int(txt[1])])
    return out


def norm_guid(g):
    h = "".join(c for c in g.lower() if c in "0123456789abcdef")
    if len(h) != 32:
        return g.lower()
    return "{%s-%s-%s-%s-%s}" % (h[0:8], h[8:12], h[12:16], h[16:20], h[20:32])


def main():
    menu = json.load(open(MENU_JSON, encoding="utf-8"))
    cmds = []
    seen = {}
    for b in menu["befehle"]:
        if not b.get("hauptmenue"):
            continue
        fid = b["id"]
        parts = label_parts(b["pfad_de"])
        if fid in OVERRIDE:
            name = OVERRIDE[fid]
        elif b.get("pfad_en_intern"):
            name = name_from_en(b["pfad_en_intern"])
        else:
            cmd = (b.get("cmd_name") or "").replace("cmd_", "")
            name = TOP.get(parts[0], "misc") + "." + camel(cmd)
        if name in seen:
            raise SystemExit("doppelter Name %s (%s, %s)" % (name, fid, seen[name]))
        seen[name] = fid
        status = (b.get("statuszeile_de") or "").split("\n")
        handlers = b.get("handler") or []
        placeholder = fid in menu["meta"]["statistik"].get("hauptmenue_ohne_handler", [])
        cmds.append({
            "fw_id": fid,
            "cmd": b.get("cmd_name"),
            "name": name,
            "path_de": parts,
            "path_en": b.get("pfad_en_intern"),
            "status_de": status[0] if status and status[0] else None,
            "label_de": parts[-1].rstrip(".").strip() or parts[-2],
            "menu_label_de": parts[-1],
            "main_menu": True,
            "js": b.get("js_api") or [],
            "undo_classes": b.get("undo_klassen") or [],
            "shortcuts_de": b.get("tastenkuerzel") or [],
            "placeholder": placeholder,
            "handlers": len(handlers),
        })
    for k in menu["kurzbefehle_ohne_menueeintrag"]:
        name = SHORTCUT_ONLY[k["cmd_name"]]
        if name in seen:
            raise SystemExit("doppelter Name " + name)
        seen[name] = k.get("id")
        status = (k.get("statuszeile_de") or "").split("\n")
        label = status[1] if len(status) > 1 and status[1] else (status[0] or k["cmd_name"])
        cmds.append({
            "fw_id": k.get("id"),
            "cmd": k["cmd_name"],
            "name": name,
            "path_de": [],
            "path_en": None,
            "status_de": status[0] if status and status[0] else None,
            "label_de": label,
            "menu_label_de": label,
            "main_menu": False,
            "js": [],
            "undo_classes": [],
            "shortcuts_de": k.get("tastenkuerzel") or [],
            "placeholder": False,
            "handlers": len(k.get("handler") or []),
        })
    for c in cmds:
        if c["name"] in LABEL_FIX:
            c["label_de"] = LABEL_FIX[c["name"]]
            if not c["main_menu"]:
                c["menu_label_de"] = c["label_de"]
    zoom = [z["faktor"] for z in menu["zoomtabelle"]]
    # technische Schluessel immer englisch (AGENTEN_REGELN 1c)
    panels = [{("command_id" if k == "befehl_id" else k): v for k, v in p.items()} for p in menu["bedienfelder"]]

    # Tastenkuerzel-Sets
    sets = {}
    for sname in SETS:
        root = ET.parse(os.path.join(SHORTCUT_DIR, sname + ".xml")).getroot()
        mc = {}
        for c in root.iter("command"):
            lst = vk_list(c.findall("shortcut"))
            if lst:
                mc[c.get("id")] = lst
        tools = {}
        tsec = root.find("tools")
        if tsec is not None:
            for t in tsec.iter("one_tool"):
                lst = vk_list(t.findall("shortcut"))
                if lst:
                    tools[norm_guid(t.get("id"))] = lst
        sets[sname] = {"menu": mc, "tools": tools}

    # Voreinstellungen
    pj = json.load(open(PREF_JSON, encoding="utf-8"))
    prefs = []
    for e in pj["eintraege"]:
        d = e.get("dialog") or {}
        prefs.append({
            "key": e["schluessel"],
            "type": e["typ"],
            "default": e["default"],
            "category": e.get("kategorie"),
            "effect": e.get("wirkung"),
            "dialog_page": d.get("seite"),
        })

    os.makedirs(OUT, exist_ok=True)
    def dump(name, obj):
        text = neutral.neutral(json.dumps(obj, ensure_ascii=False, indent=0))
        with open(os.path.join(OUT, name), "w", encoding="utf-8", newline="\n") as f:
            f.write(text)

    dump("commands.json", {"commands": cmds, "zoom_steps": zoom, "panels": panels})
    dump("shortcuts.json", {"sets": sets, "order": SETS})
    # Beschreibende Texte ohne fremde Produktnamen (F12, 14.10.2026); Schluessel bleiben wie im Original
    for p in prefs:
        for k in ("category", "effect", "dialog_page"):
            v = p.get(k)
            if isinstance(v, str):
                v = v.replace("Photoshop-Zusatzmodulordner", "Zusatzmodulordner (8BF)")
                v = v.replace("Photoshop-Zusatzmodule", "Zusatzmodule (8BF)").replace("Photoshop-", "PSD-")
                # F14: weitere Produktnamen desselben Herstellers/Vorgaengers
                v = v.replace("CS-Extensions (CSXS/Flash-Panels)", "CS-Erweiterungen (CSXS/SWF-Bedienfelder)")
                v = v.replace("Flash-Bedienfelder", "SWF-Bedienfelder").replace("Director-Export", "DIR-Export")
                v = v.replace("FreeHand-8-kompatibel", "FH8-kompatibel")
                p[k] = v
    dump("prefs.json", {"prefs": prefs})
    print("Befehle:", len(cmds), "davon Hauptmenue:", sum(1 for c in cmds if c["main_menu"]))
    print("Sets:", {k: (len(v["menu"]), len(v["tools"])) for k, v in sets.items()})
    print("Voreinstellungen:", len(prefs))


if __name__ == "__main__":
    sys.exit(main())
