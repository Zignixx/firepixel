"""Neutralitaet erzeugter Daten (ARCHITEKTUR §1, docs\\AGENTEN_REGELN.md §1a).

Die Namen des Originals und seines Herstellers stehen nirgends im Quelltext von Firepixel. Generatoren,
die Texte aus der Installationskopie bzw. aus ..\\re\\ uebernehmen, ersetzen sie mit `neutral()` bzw.
lassen Datensaetze mit `contains()` weg. Die Namen werden zur Laufzeit aus der Installationskopie
gelesen:

  Programmname = Dateiname (ohne Endung) der Haupt-EXE (der EXE mit der Ressource CMYK/1001, erkannt
                 am UTF-16-Ressourcennamen; die groesste EXE im Ordner),
  Hersteller   = `CompanyName` aus der Versionsressource dieser EXE.

Korpusordner: Umgebungsvariable FP_CORPUS_DIR, sonst <REA>\\korpus (Verzeichnisverbindung auf die
Installationskopie).
"""
import os
import re

REA = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", ".."))
CORPUS = os.environ.get("FP_CORPUS_DIR") or os.path.join(REA, "korpus")

_names = None


def _main_exe(root):
    exes = [os.path.join(root, f) for f in os.listdir(root) if f.lower().endswith(".exe")]
    exes.sort(key=os.path.getsize, reverse=True)
    for p in exes:
        with open(p, "rb") as fh:
            data = fh.read()
        if "CMYK".encode("utf-16-le") in data:
            return p, data
    raise FileNotFoundError("keine Haupt-EXE im Korpus " + root)


def _version_string(data, key):
    k = (key + "\0").encode("utf-16-le")
    i = data.find(k)
    if i < 0:
        return None
    i += len(k)
    while i % 4:  # Ausrichtung auf 32 Bit
        i += 1
    end = data.find(b"\0\0", i)
    while end >= 0 and (end - i) % 2:
        end = data.find(b"\0\0", end + 1)
    return data[i:end].decode("utf-16-le", errors="replace").strip() or None


def names():
    """(Programmname, Hersteller) aus der Installationskopie."""
    global _names
    if _names is None:
        exe, data = _main_exe(CORPUS)
        app = os.path.splitext(os.path.basename(exe))[0]
        vendor = _version_string(data, "CompanyName") or ""
        vendor = vendor.split()[0] if vendor else ""
        _names = (app, vendor)
    return _names


def contains(text):
    """Enthaelt der Text einen der beiden Namen (gleich welche Schreibweise)?"""
    app, vendor = names()
    pats = [re.escape(n) for n in (app, vendor) if n]
    return bool(pats) and re.search("(?i)" + "|".join(pats), text) is not None


def neutral(text):
    """Ersetzt die Namen: Hersteller + Programm bzw. Programm -> "Firepixel" (Schreibweise
    beibehalten), Hersteller als Vorsilbe ("<Hersteller>-Onlineforen", "<HERSTELLER> AI3") entfaellt."""
    app, vendor = names()
    if vendor:
        text = re.sub(re.escape(vendor) + r"[ -](?=" + re.escape(app) + ")", "", text, flags=re.I)
    def rep(m):
        s = m.group(0)
        if s.isupper():
            return "FIREPIXEL"
        if s[0].islower():
            return "firepixel"
        return "Firepixel"
    text = re.sub(re.escape(app), rep, text, flags=re.I)
    if vendor:
        text = re.sub(re.escape(vendor) + r"[ -]?", "", text, flags=re.I)
    return text


if __name__ == "__main__":
    a, v = names()
    print("Programmname gefunden:", bool(a), "Hersteller gefunden:", bool(v))
    print(neutral("Test"))
